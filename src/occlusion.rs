use bevy::camera::primitives::Frustum;
use bevy::prelude::*;

use crate::chunk::{world_to_chunk_and_local, Chunk, ChunkPos, CHUNK_SIZE_I32};
use crate::culling::is_chunk_in_frustum;
use crate::registry::BlockRegistry;
use crate::world::VoxelWorld;

/// Toggle resource enabling or disabling raycast occlusion culling for performance comparison.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OcclusionCullingEnabled(pub bool);

impl Default for OcclusionCullingEnabled {
    fn default() -> Self {
        Self(true)
    }
}

/// Marker component denoting that a chunk is 100% composed of opaque blocks and completely occludes rays.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SolidChunk;

/// Marker component placed on chunk entities currently occluded by intervening solid chunks.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OccludedChunk;

/// Determines whether all blocks in a chunk are opaque.
///
/// # Arguments
/// * `chunk` - Chunk voxel data.
/// * `registry` - Block descriptor registry.
///
/// # Example
/// ```rust
/// use block_game_2::occlusion::check_chunk_solid;
/// use block_game_2::chunk::{Chunk, BlockId};
/// use block_game_2::registry::BlockRegistry;
///
/// let mut chunk = Chunk::new(BlockId(1)); // Stone
/// let registry = BlockRegistry::with_defaults();
/// assert!(check_chunk_solid(&chunk, &registry));
/// ```
pub fn check_chunk_solid(chunk: &Chunk, registry: &BlockRegistry) -> bool {
    chunk.raw_blocks().iter().all(|&id| registry.is_opaque(id))
}

/// Generates ray sample points within the target chunk volume based on distance and jitter offset.
///
/// # Arguments
/// * `pos` - Target chunk grid position.
/// * `distance` - Distance from camera to chunk center in world units.
/// * `jitter` - Pseudo-random or frame-varying jitter offset vector within `[-2.0, 2.0]`.
///
/// # Example
/// ```rust
/// use block_game_2::occlusion::get_chunk_sample_points;
/// use block_game_2::chunk::ChunkPos;
/// use bevy::prelude::Vec3;
///
/// let samples = get_chunk_sample_points(ChunkPos::new(0, 0, 0), 20.0, Vec3::ZERO);
/// assert_eq!(samples.len(), 1);
/// ```
pub fn get_chunk_sample_points(pos: ChunkPos, distance: f32, jitter: Vec3) -> Vec<Vec3> {
    let base_x = (pos.0.x * CHUNK_SIZE_I32) as f32;
    let base_y = (pos.0.y * CHUNK_SIZE_I32) as f32;
    let base_z = (pos.0.z * CHUNK_SIZE_I32) as f32;
    let center = Vec3::new(base_x + 8.0, base_y + 8.0, base_z + 8.0) + jitter;

    if distance < 32.0 || distance > 128.0 {
        // Near chunks and far chunks use a single center sample (distance-based cheapening)
        vec![center]
    } else {
        // Intermediate chunks sample center plus 4 distributed sample points
        vec![
            center,
            Vec3::new(base_x + 3.0, base_y + 3.0, base_z + 3.0),
            Vec3::new(base_x + 13.0, base_y + 13.0, base_z + 3.0),
            Vec3::new(base_x + 3.0, base_y + 13.0, base_z + 13.0),
            Vec3::new(base_x + 13.0, base_y + 3.0, base_z + 13.0),
        ]
    }
}

/// Tests whether a single ray between `camera_pos` and `target` is blocked by intervening solid chunks.
///
/// # Arguments
/// * `camera_pos` - World position of the camera.
/// * `target` - Target sample point in world coordinates.
/// * `target_chunk` - Target chunk position.
/// * `is_solid_fn` - Query function returning true if chunk at ChunkPos is 100% solid.
pub fn is_ray_blocked<F>(
    camera_pos: Vec3,
    target: Vec3,
    target_chunk: ChunkPos,
    is_solid_fn: &F,
) -> bool
where
    F: Fn(ChunkPos) -> bool,
{
    let dir = target - camera_pos;
    let total_dist = dir.length();
    if total_dist <= 16.0 {
        return false;
    }

    let (camera_chunk, _) = world_to_chunk_and_local(camera_pos.as_ivec3());
    let step_size = 8.0;
    let mut traveled = 16.0;

    while traveled < total_dist - 8.0 {
        let sample = camera_pos + dir * (traveled / total_dist);
        let (intervening_chunk, _) = world_to_chunk_and_local(sample.as_ivec3());

        if intervening_chunk != camera_chunk
            && intervening_chunk != target_chunk
            && is_solid_fn(intervening_chunk)
        {
            return true;
        }
        traveled += step_size;
    }

    false
}

/// Timer throttling occlusion culling checks (10 Hz) to avoid CPU spikes.
#[derive(Resource, Debug)]
pub struct OcclusionCullingTimer(pub Timer);

impl Default for OcclusionCullingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.1, TimerMode::Repeating))
    }
}

/// System testing frustum-visible chunks for raycast occlusion against intervening solid chunks.
#[allow(clippy::too_many_arguments)]
pub fn occlusion_culling_system(
    time: Option<Res<Time>>,
    mut commands: Commands,
    camera_query: Query<(&GlobalTransform, Option<&Frustum>), With<Camera3d>>,
    chunks: Query<(Entity, &ChunkPos, Option<&OccludedChunk>)>,
    solid_chunks: Query<&SolidChunk>,
    voxel_world: Res<VoxelWorld>,
    enabled: Option<Res<OcclusionCullingEnabled>>,
    timer_res: Option<ResMut<OcclusionCullingTimer>>,
) {
    if let (Some(mut timer), Some(time)) = (timer_res, time) {
        timer.0.tick(time.delta());
        if !timer.0.just_finished() {
            return;
        }
    }

    let is_enabled = enabled.map(|e| e.0).unwrap_or(true);

    if !is_enabled {
        // Restore all occluded chunks if culling is toggled off
        for (entity, _pos, occluded) in chunks.iter() {
            if occluded.is_some() {
                commands.entity(entity).remove::<OccludedChunk>();
                commands.entity(entity).insert(Visibility::Inherited);
            }
        }
        return;
    }

    let (cam_transform, frustum_opt) = match camera_query.single() {
        Ok(res) => res,
        Err(_) => return,
    };

    let camera_pos = cam_transform.translation();
    let is_solid_fn = |pos: ChunkPos| -> bool {
        voxel_world
            .get(&pos)
            .and_then(|e| solid_chunks.get(e).ok())
            .is_some()
    };

    // Small jitter based on camera position components
    let jitter = Vec3::new(
        ((camera_pos.x * 13.0).sin()) * 1.5,
        ((camera_pos.y * 17.0).cos()) * 1.5,
        ((camera_pos.z * 19.0).sin()) * 1.5,
    );

    for (entity, &pos, occluded_opt) in chunks.iter() {
        // Cheap prefilter from Phase 5: skip occlusion testing if outside camera frustum
        if frustum_opt.is_some_and(|frustum| crate::culling::is_frustum_initialized(frustum) && !is_chunk_in_frustum(pos, frustum)) {
            continue;
        }

        let chunk_center = Vec3::new(
            (pos.0.x * CHUNK_SIZE_I32) as f32 + 8.0,
            (pos.0.y * CHUNK_SIZE_I32) as f32 + 8.0,
            (pos.0.z * CHUNK_SIZE_I32) as f32 + 8.0,
        );
        let dist = camera_pos.distance(chunk_center);
        let samples = get_chunk_sample_points(pos, dist, jitter);

        // Occluded if ALL sample rays are blocked
        let is_occluded = !samples.is_empty()
            && samples
                .iter()
                .all(|&sample| is_ray_blocked(camera_pos, sample, pos, &is_solid_fn));

        if is_occluded && occluded_opt.is_none() {
            commands.entity(entity).insert((
                OccludedChunk,
                Visibility::Hidden,
            ));
        } else if !is_occluded && occluded_opt.is_some() {
            commands.entity(entity).remove::<OccludedChunk>();
            commands.entity(entity).insert(Visibility::Inherited);
        }
    }
}
