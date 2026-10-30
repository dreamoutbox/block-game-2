use bevy::prelude::*;
use bevy::tasks::futures_lite::future::block_on;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::chunk::{chunk_and_local_to_world, BlockId, Chunk, ChunkDirty, ChunkPos, CHUNK_SIZE};
use crate::registry::BlockType;
use crate::world::VoxelWorld;

/// Configuration options for procedural terrain generation.
#[derive(Debug, Clone)]
pub struct TerrainConfig {
    /// Random seed determining terrain topology.
    pub seed: u32,
    /// Base sea-level or baseline elevation in block units.
    pub base_height: f64,
    /// Horizontal frequency scale for sampling noise.
    pub scale: f64,
    /// Vertical amplitude multiplier for elevation variation.
    pub amplitude: f64,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            base_height: 16.0,
            scale: 0.02,
            amplitude: 14.0,
        }
    }
}

/// Pure, deterministic terrain generator producing voxel chunks from a seed and position.
#[derive(Resource, Clone)]
pub struct TerrainGenerator {
    config: TerrainConfig,
    noise: Fbm<Perlin>,
}

impl TerrainGenerator {
    /// Constructs a new terrain generator with the given configuration.
    ///
    /// # Arguments
    /// * `config` - Terrain generation settings including seed and scaling.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world_gen::{TerrainConfig, TerrainGenerator};
    ///
    /// let generator = TerrainGenerator::new(TerrainConfig::default());
    /// let h = generator.height_at(0, 0);
    /// assert!(h > 0);
    /// ```
    pub fn new(config: TerrainConfig) -> Self {
        let noise = Fbm::<Perlin>::new(config.seed)
            .set_octaves(4)
            .set_frequency(1.0)
            .set_lacunarity(2.0)
            .set_persistence(0.5);

        Self { config, noise }
    }

    /// Computes the integer surface height for a column at global world coordinates (x, z).
    ///
    /// # Arguments
    /// * `world_x` - Global world X block coordinate.
    /// * `world_z` - Global world Z block coordinate.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world_gen::{TerrainConfig, TerrainGenerator};
    ///
    /// let generator = TerrainGenerator::new(TerrainConfig::default());
    /// let h1 = generator.height_at(10, 20);
    /// let h2 = generator.height_at(10, 20);
    /// assert_eq!(h1, h2);
    /// ```
    pub fn height_at(&self, world_x: i32, world_z: i32) -> i32 {
        let nx = (world_x as f64) * self.config.scale;
        let nz = (world_z as f64) * self.config.scale;
        let val = self.noise.get([nx, nz]);
        let height = self.config.base_height + val * self.config.amplitude;
        height.round() as i32
    }

    /// Evaluates which block type should occupy the given global elevation relative to surface height.
    ///
    /// # Arguments
    /// * `world_y` - Global world Y block coordinate.
    /// * `surface_y` - Global column surface height.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world_gen::TerrainGenerator;
    /// use block_game_2::registry::BlockType;
    ///
    /// assert_eq!(TerrainGenerator::block_for_height(10, 5), BlockType::Air.into());
    /// assert_eq!(TerrainGenerator::block_for_height(5, 5), BlockType::Grass.into());
    /// assert_eq!(TerrainGenerator::block_for_height(4, 5), BlockType::Dirt.into());
    /// assert_eq!(TerrainGenerator::block_for_height(0, 5), BlockType::Stone.into());
    /// ```
    #[inline]
    pub fn block_for_height(world_y: i32, surface_y: i32) -> BlockId {
        if world_y > surface_y {
            BlockType::Air.into()
        } else if world_y == surface_y {
            BlockType::Grass.into()
        } else if world_y >= surface_y - 3 {
            BlockType::Dirt.into()
        } else {
            BlockType::Stone.into()
        }
    }

    /// Deterministically generates a chunk at `pos` without modifying any shared state.
    ///
    /// # Arguments
    /// * `pos` - Target chunk position.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world_gen::{TerrainConfig, TerrainGenerator};
    /// use block_game_2::chunk::ChunkPos;
    ///
    /// let generator = TerrainGenerator::new(TerrainConfig::default());
    /// let chunk = generator.generate_chunk(ChunkPos::new(0, 0, 0));
    /// assert!(chunk.get_block(0, 0, 0).is_some());
    /// ```
    pub fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let mut chunk = Chunk::new(BlockType::Air.into());

        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let world_col = chunk_and_local_to_world(pos, lx, 0, lz);
                let surface_y = self.height_at(world_col.x, world_col.z);

                for ly in 0..CHUNK_SIZE {
                    let world_pos = chunk_and_local_to_world(pos, lx, ly, lz);
                    let block = Self::block_for_height(world_pos.y, surface_y);
                    chunk.set_block(lx, ly, lz, block);
                }
            }
        }

        chunk
    }
}

/// Spawns a procedurally generated chunk into the ECS world if not already present.
/// Returns the existing or newly created Entity.
///
/// # Arguments
/// * `commands` - Entity commands to spawn entities and components.
/// * `world` - VoxelWorld resource tracking loaded chunks.
/// * `generator` - Procedural terrain generator.
/// * `pos` - Chunk grid coordinate to spawn.
///
/// # Example
/// ```rust
/// use block_game_2::world_gen::{spawn_chunk, TerrainConfig, TerrainGenerator};
/// use block_game_2::world::VoxelWorld;
/// use block_game_2::chunk::ChunkPos;
/// use bevy::prelude::*;
///
/// let mut app = App::new();
/// let mut world = VoxelWorld::default();
/// let generator = TerrainGenerator::new(TerrainConfig::default());
/// let entity = spawn_chunk(&mut app.world_mut().commands(), &mut world, &generator, ChunkPos::new(0, 0, 0));
/// assert!(world.contains(&ChunkPos::new(0, 0, 0)));
/// ```
pub fn spawn_chunk(
    commands: &mut Commands,
    world: &mut VoxelWorld,
    generator: &TerrainGenerator,
    pos: ChunkPos,
) -> Entity {
    if let Some(existing) = world.get(&pos) {
        return existing;
    }

    let chunk = generator.generate_chunk(pos);
    let entity = commands
        .spawn((pos, chunk, ChunkDirty, crate::culling::chunk_local_aabb()))
        .id();
    world.insert(pos, entity);
    entity
}

/// Component holding an in-flight background procedural chunk generation task.
#[derive(Component)]
pub struct GenerateChunkTask(pub Task<Chunk>);

/// Dispatches asynchronous procedural chunk generation to the compute task pool.
///
/// # Arguments
/// * `commands` - Entity commands to spawn entities and components.
/// * `world` - VoxelWorld resource tracking loaded chunks.
/// * `generator` - Procedural terrain generator.
/// * `pos` - Chunk grid coordinate to spawn.
///
/// # Example
/// ```rust
/// use block_game_2::world_gen::{spawn_chunk_async, TerrainConfig, TerrainGenerator};
/// use block_game_2::world::VoxelWorld;
/// use block_game_2::chunk::ChunkPos;
/// use bevy::prelude::*;
///
/// let mut app = App::new();
/// bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
/// let mut world = VoxelWorld::default();
/// let generator = TerrainGenerator::new(TerrainConfig::default());
/// let entity = spawn_chunk_async(&mut app.world_mut().commands(), &mut world, &generator, ChunkPos::new(0, 0, 0));
/// assert!(world.contains(&ChunkPos::new(0, 0, 0)));
/// ```
pub fn spawn_chunk_async(
    commands: &mut Commands,
    world: &mut VoxelWorld,
    generator: &TerrainGenerator,
    pos: ChunkPos,
) -> Entity {
    if let Some(existing) = world.get(&pos) {
        return existing;
    }

    let pool = AsyncComputeTaskPool::get();
    let generator_clone = generator.clone();
    let task = pool.spawn(async move {
        generator_clone.generate_chunk(pos)
    });

    let entity = commands
        .spawn((
            pos,
            GenerateChunkTask(task),
            crate::culling::chunk_local_aabb(),
        ))
        .id();

    world.insert(pos, entity);
    entity
}

/// System polling in-flight background chunk generation tasks and adding chunk voxel data and ChunkDirty.
pub fn poll_chunk_generation_system(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut GenerateChunkTask)>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if task.0.is_finished() {
            let chunk = block_on(&mut task.0);
            commands
                .entity(entity)
                .remove::<GenerateChunkTask>()
                .insert((chunk, ChunkDirty));
        }
    }
}
