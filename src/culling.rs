use bevy::camera::primitives::{Aabb, Frustum};
use bevy::math::Vec3A;
use bevy::prelude::*;

use crate::chunk::{ChunkPos, CHUNK_SIZE, CHUNK_SIZE_I32};

/// Computes an exact local bounding box for a 16x16x16 chunk.
///
/// # Example
/// ```rust
/// use block_game_2::culling::chunk_local_aabb;
///
/// let aabb = chunk_local_aabb();
/// assert_eq!(aabb.center.x, 8.0);
/// assert_eq!(aabb.half_extents.x, 8.0);
/// ```
pub fn chunk_local_aabb() -> Aabb {
    let half_size = (CHUNK_SIZE as f32) / 2.0;
    Aabb {
        center: Vec3A::splat(half_size),
        half_extents: Vec3A::splat(half_size),
    }
}

/// Computes a world-space bounding box for a chunk at the specified grid position.
///
/// # Arguments
/// * `pos` - Chunk grid coordinate.
///
/// # Example
/// ```rust
/// use block_game_2::culling::chunk_world_aabb;
/// use block_game_2::chunk::ChunkPos;
///
/// let aabb = chunk_world_aabb(ChunkPos::new(1, 0, 0));
/// assert_eq!(aabb.center.x, 24.0);
/// ```
pub fn chunk_world_aabb(pos: ChunkPos) -> Aabb {
    let half_size = (CHUNK_SIZE as f32) / 2.0;
    let origin_x = (pos.0.x * CHUNK_SIZE_I32) as f32;
    let origin_y = (pos.0.y * CHUNK_SIZE_I32) as f32;
    let origin_z = (pos.0.z * CHUNK_SIZE_I32) as f32;

    Aabb {
        center: Vec3A::new(
            origin_x + half_size,
            origin_y + half_size,
            origin_z + half_size,
        ),
        half_extents: Vec3A::splat(half_size),
    }
}

/// Checks whether a camera frustum has been computed with valid normal vectors.
pub fn is_frustum_initialized(frustum: &Frustum) -> bool {
    frustum.half_spaces[0].normal() != Vec3A::ZERO
}

/// Tests whether a chunk's bounding box intersects the given view frustum.
///
/// # Arguments
/// * `pos` - Chunk grid coordinate.
/// * `frustum` - Camera frustum planes.
///
/// # Example
/// ```rust
/// use block_game_2::culling::{chunk_world_aabb, is_chunk_in_frustum};
/// use block_game_2::chunk::ChunkPos;
/// use bevy::camera::primitives::Frustum;
///
/// // Verify function signature and invocation
/// let frustum = Frustum::default();
/// let _ = is_chunk_in_frustum(ChunkPos::new(0, 0, 0), &frustum);
/// ```
pub fn is_chunk_in_frustum(pos: ChunkPos, frustum: &Frustum) -> bool {
    let aabb = chunk_world_aabb(pos);
    frustum.intersects_obb(&aabb, &GlobalTransform::IDENTITY.affine(), true, false)
}

/// Resource recording frustum culling visibility statistics for diagnostics.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelRenderStats {
    pub total_chunks: usize,
    pub visible_chunks: usize,
    pub culled_chunks: usize,
}

/// System collecting frustum culling statistics based on ECS `ViewVisibility`.
pub fn update_render_stats_system(
    chunks: Query<(&ChunkPos, &ViewVisibility)>,
    mut stats: ResMut<VoxelRenderStats>,
) {
    let mut total = 0;
    let mut visible = 0;

    for (_pos, view_vis) in chunks.iter() {
        total += 1;
        if view_vis.get() {
            visible += 1;
        }
    }

    stats.total_chunks = total;
    stats.visible_chunks = visible;
    stats.culled_chunks = total.saturating_sub(visible);
}
