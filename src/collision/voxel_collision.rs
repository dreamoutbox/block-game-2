use bevy::prelude::{IVec3, Vec3};

use crate::collision::Aabb3d;

/// Maximum substep distance along any axis to avoid tunneling through thin blocks at high speeds.
const MAX_SUBSTEP_DISTANCE: f32 = 0.25;

/// Resolves motion of an AABB against solid unit voxels using axis-by-axis sweeping and sliding.
///
/// # Arguments
/// * `pos` - Initial center position of the moving AABB.
/// * `delta` - Requested movement vector.
/// * `half_extents` - Half extents of the moving AABB.
/// * `is_solid` - Function returning `true` if the block at `IVec3` is solid.
///
/// # Returns
/// The resolved new center position of the AABB after sliding against solid blocks.
///
/// # Example
/// ```rust
/// use block_game_2::collision::resolve_voxel_collision;
/// use bevy::prelude::{IVec3, Vec3};
///
/// let pos = Vec3::new(0.5, 0.5, 0.5);
/// let delta = Vec3::new(1.0, 0.0, 0.0);
/// let half_extents = Vec3::splat(0.3);
/// let is_solid = |p: IVec3| p == IVec3::new(1, 0, 0);
/// let final_pos = resolve_voxel_collision(pos, delta, half_extents, is_solid);
/// assert!(final_pos.x <= 0.7);
/// ```
pub fn resolve_voxel_collision(
    pos: Vec3,
    delta: Vec3,
    half_extents: Vec3,
    is_solid: impl Fn(IVec3) -> bool,
) -> Vec3 {
    if delta == Vec3::ZERO {
        return pos;
    }

    // Determine number of substeps based on max displacement along any axis
    let max_disp = delta.x.abs().max(delta.y.abs()).max(delta.z.abs());
    let substeps = ((max_disp / MAX_SUBSTEP_DISTANCE).ceil() as usize).max(1);
    let step_delta = delta / (substeps as f32);

    let mut current_pos = pos;

    for _ in 0..substeps {
        // Resolve axis by axis: X, then Y, then Z to allow smooth sliding
        current_pos = resolve_single_axis(current_pos, step_delta.x, 0, half_extents, &is_solid);
        current_pos = resolve_single_axis(current_pos, step_delta.y, 1, half_extents, &is_solid);
        current_pos = resolve_single_axis(current_pos, step_delta.z, 2, half_extents, &is_solid);
    }

    current_pos
}

/// Checks whether an AABB at `center` with `half_extents` overlaps the unit block at `voxel_pos`.
///
/// # Arguments
/// * `center` - Center of the AABB.
/// * `half_extents` - Half extents of the AABB.
/// * `voxel_pos` - Unit voxel position.
///
/// # Example
/// ```rust
/// use block_game_2::collision::aabb_overlaps_block;
/// use bevy::prelude::{IVec3, Vec3};
///
/// assert!(aabb_overlaps_block(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(0.3), IVec3::new(0, 0, 0)));
/// assert!(!aabb_overlaps_block(Vec3::new(2.5, 0.5, 0.5), Vec3::splat(0.3), IVec3::new(0, 0, 0)));
/// ```
pub fn aabb_overlaps_block(center: Vec3, half_extents: Vec3, voxel_pos: IVec3) -> bool {
    let aabb = Aabb3d::from_center_half_extents(center, half_extents);
    aabb.intersects_block(voxel_pos)
}

/// Helper moving an AABB along a single axis and clamping its movement against solid blocks.
fn resolve_single_axis(
    mut pos: Vec3,
    axis_delta: f32,
    axis: usize,
    half_extents: Vec3,
    is_solid: &impl Fn(IVec3) -> bool,
) -> Vec3 {
    if axis_delta == 0.0 {
        return pos;
    }

    let initial_pos = pos;
    let target_axis_val = pos[axis] + axis_delta;
    pos[axis] = target_axis_val;

    let aabb = Aabb3d::from_center_half_extents(pos, half_extents);
    let (min_voxel, max_voxel) = aabb.voxel_bounds();

    // Check all potentially overlapping voxels in the broadphase box
    for vz in min_voxel.z..=max_voxel.z {
        for vy in min_voxel.y..=max_voxel.y {
            for vx in min_voxel.x..=max_voxel.x {
                let voxel = IVec3::new(vx, vy, vz);
                if !is_solid(voxel) {
                    continue;
                }

                // If moving positive (+), we hit the min face of the block
                // If moving negative (-), we hit the max face of the block
                let block_min = voxel.as_vec3();
                let block_max = block_min + Vec3::ONE;

                if axis_delta > 0.0 {
                    // Contact on the negative face of the block
                    let allowed_max = block_min[axis] - half_extents[axis] - 1e-4;
                    // Only clamp if we didn't start deeper than allowed (handling stuck situations cleanly)
                    if initial_pos[axis] <= allowed_max + 1e-3 {
                        pos[axis] = pos[axis].min(allowed_max);
                    }
                } else {
                    // Contact on the positive face of the block
                    let allowed_min = block_max[axis] + half_extents[axis] + 1e-4;
                    if initial_pos[axis] >= allowed_min - 1e-3 {
                        pos[axis] = pos[axis].max(allowed_min);
                    }
                }
            }
        }
    }

    pos
}
