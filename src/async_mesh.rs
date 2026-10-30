use bevy::prelude::*;
use bevy::tasks::futures_lite::future::block_on;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use std::sync::Arc;

use crate::chunk::{Chunk, ChunkDirty, ChunkPos, CHUNK_SIZE_I32};
use crate::mesh::{build_chunk_mesh, VoxelMaterialHandle};
use crate::registry::{BlockFace, BlockRegistry};
use crate::world::VoxelWorld;

/// Global configuration controlling debounce delay before kicking off background meshing.
#[derive(Resource, Clone, Copy, Debug)]
pub struct DebounceSettings {
    /// Number of frames to wait after the most recent edit before submitting task.
    pub debounce_frames: u32,
}

impl Default for DebounceSettings {
    fn default() -> Self {
        Self { debounce_frames: 2 }
    }
}

/// Component tracking frame countdown for debouncing rapid block edits on a chunk.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkDebounce {
    pub frames_remaining: u32,
}

impl ChunkDebounce {
    /// Creates a new debounce tracker with the specified countdown.
    ///
    /// # Arguments
    /// * `frames` - Number of frames to debounce.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::async_mesh::ChunkDebounce;
    ///
    /// let debounce = ChunkDebounce::new(3);
    /// assert_eq!(debounce.frames_remaining, 3);
    /// ```
    pub fn new(frames: u32) -> Self {
        Self {
            frames_remaining: frames,
        }
    }
}

/// Marker component indicating that a chunk was edited while an async remesh task was in-flight.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequeueRemesh;

/// Component holding an in-flight background mesh generation task.
#[derive(Component)]
pub struct ComputeMeshTask(pub Task<Mesh>);

/// Self-contained immutable snapshot of a chunk and its 6 immediate orthogonal neighbors.
/// Safe to transfer across threads without live borrowing from the ECS world.
#[derive(Clone)]
pub struct ChunkSnapshot {
    pub pos: ChunkPos,
    pub center: Arc<Chunk>,
    pub neighbors: [Option<Arc<Chunk>>; 6],
}

impl ChunkSnapshot {
    /// Builds a snapshot by capturing immutable `Arc` copies of the target chunk and adjacent neighbors.
    ///
    /// # Arguments
    /// * `pos` - Target chunk position.
    /// * `center` - Center chunk reference.
    /// * `voxel_world` - Resource locating chunk entities by grid position.
    /// * `all_chunks` - ECS query for chunk components.
    pub fn from_world(
        pos: ChunkPos,
        center: &Chunk,
        voxel_world: &VoxelWorld,
        all_chunks: &Query<&Chunk>,
    ) -> Self {
        let center_arc = Arc::new(center.clone());
        let mut neighbors: [Option<Arc<Chunk>>; 6] = [None, None, None, None, None, None];

        for (idx, face) in BlockFace::ALL.iter().enumerate() {
            let normal = face.normal();
            let neighbor_pos = ChunkPos::new(
                pos.0.x + normal.x,
                pos.0.y + normal.y,
                pos.0.z + normal.z,
            );

            if let Some(neighbor_chunk) = voxel_world
                .get(&neighbor_pos)
                .and_then(|entity| all_chunks.get(entity).ok())
            {
                neighbors[idx] = Some(Arc::new(neighbor_chunk.clone()));
            }
        }

        Self {
            pos,
            center: center_arc,
            neighbors,
        }
    }

    /// Computes the mesh for this snapshot using the provided block registry.
    ///
    /// # Arguments
    /// * `registry` - Block descriptor registry for face culling and UVs.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::async_mesh::ChunkSnapshot;
    /// use block_game_2::chunk::{Chunk, ChunkPos, BlockId};
    /// use block_game_2::registry::BlockRegistry;
    /// use std::sync::Arc;
    ///
    /// let mut chunk = Chunk::new(BlockId(0));
    /// chunk.set_block(0, 0, 0, BlockId(1));
    /// let snapshot = ChunkSnapshot {
    ///     pos: ChunkPos::new(0, 0, 0),
    ///     center: Arc::new(chunk),
    ///     neighbors: [None, None, None, None, None, None],
    /// };
    /// let registry = BlockRegistry::with_defaults();
    /// let mesh = snapshot.build_mesh(&registry);
    /// assert!(mesh.indices().is_some());
    /// ```
    pub fn build_mesh(&self, registry: &BlockRegistry) -> Mesh {
        let get_chunk = |neighbor_pos: ChunkPos| -> Option<&Chunk> {
            let offset = neighbor_pos.0 - self.pos.0;
            for (idx, face) in BlockFace::ALL.iter().enumerate() {
                if face.normal() == offset {
                    return self.neighbors[idx].as_deref();
                }
            }
            None
        };

        build_chunk_mesh(self.pos, &self.center, &get_chunk, registry)
    }
}

/// Marks a chunk as dirty with debouncing to coalesce rapid successive edits.
///
/// # Arguments
/// * `commands` - Entity commands to update components.
/// * `entity` - Target chunk entity.
/// * `debounce_frames` - Number of frames before meshing task is launched.
/// * `is_task_running` - True if a ComputeMeshTask is already in-flight.
///
/// # Example
/// ```rust
/// use block_game_2::async_mesh::request_chunk_remesh;
/// use bevy::prelude::*;
///
/// let mut app = App::new();
/// let entity = app.world_mut().spawn_empty().id();
/// request_chunk_remesh(&mut app.world_mut().commands(), entity, 2, false);
/// ```
pub fn request_chunk_remesh(
    commands: &mut Commands,
    entity: Entity,
    debounce_frames: u32,
    is_task_running: bool,
) {
    if is_task_running {
        commands.entity(entity).insert(RequeueRemesh);
    } else {
        commands.entity(entity).insert((
            ChunkDirty,
            ChunkDebounce::new(debounce_frames),
        ));
    }
}

/// System debouncing edits and dispatching async compute meshing tasks to the background thread pool.
#[allow(clippy::type_complexity)]
pub fn queue_async_remesh_system(
    mut commands: Commands,
    mut dirty_chunks: Query<
        (
            Entity,
            &ChunkPos,
            &Chunk,
            Option<&mut ChunkDebounce>,
            Option<&ComputeMeshTask>,
        ),
        With<ChunkDirty>,
    >,
    all_chunks: Query<&Chunk>,
    voxel_world: Res<VoxelWorld>,
    registry: Res<BlockRegistry>,
    debounce_settings: Option<Res<DebounceSettings>>,
) {
    let pool = AsyncComputeTaskPool::get();
    let registry_arc = Arc::new(registry.clone());
    let default_debounce = debounce_settings.map(|s| s.debounce_frames).unwrap_or(2);

    for (entity, &pos, chunk, debounce_opt, in_flight) in dirty_chunks.iter_mut() {
        if in_flight.is_some() {
            // Task already active: coalesce this edit into a requeue flag
            commands.entity(entity).insert(RequeueRemesh);
            commands.entity(entity).remove::<ChunkDirty>();
            continue;
        }

        if let Some(mut debounce) = debounce_opt {
            if debounce.frames_remaining > 0 {
                debounce.frames_remaining -= 1;
                continue;
            }
        } else if default_debounce > 0 {
            // Initialize debounce countdown
            commands
                .entity(entity)
                .insert(ChunkDebounce::new(default_debounce.saturating_sub(1)));
            continue;
        }

        // Ready to dispatch background task
        commands.entity(entity).remove::<ChunkDirty>();
        commands.entity(entity).remove::<ChunkDebounce>();

        let snapshot = ChunkSnapshot::from_world(pos, chunk, &voxel_world, &all_chunks);
        let reg = registry_arc.clone();

        let task = pool.spawn(async move {
            snapshot.build_mesh(&reg)
        });

        commands.entity(entity).insert(ComputeMeshTask(task));
    }
}

/// System polling in-flight background meshing tasks each frame and applying completed meshes to chunk entities.
pub fn poll_mesh_tasks_system(
    mut commands: Commands,
    mut tasks: Query<(
        Entity,
        &ChunkPos,
        &mut ComputeMeshTask,
        Option<&RequeueRemesh>,
    )>,
    material_res: Option<Res<VoxelMaterialHandle>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, &pos, mut task_comp, requeue) in tasks.iter_mut() {
        if task_comp.0.is_finished() {
            let mesh = block_on(&mut task_comp.0);
            let mesh_handle = meshes.add(mesh);

            let translation = Vec3::new(
                (pos.0.x * CHUNK_SIZE_I32) as f32,
                (pos.0.y * CHUNK_SIZE_I32) as f32,
                (pos.0.z * CHUNK_SIZE_I32) as f32,
            );

            let mut entity_cmds = commands.entity(entity);
            entity_cmds.remove::<ComputeMeshTask>();
            entity_cmds.insert((
                Mesh3d(mesh_handle),
                Transform::from_translation(translation),
                Visibility::default(),
            ));

            if let Some(ref mat) = material_res {
                entity_cmds.insert(MeshMaterial3d(mat.material.clone()));
            }

            if requeue.is_some() {
                entity_cmds.remove::<RequeueRemesh>();
                entity_cmds.insert(ChunkDirty);
            }
        }
    }
}
