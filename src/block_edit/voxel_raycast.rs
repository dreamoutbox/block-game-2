use bevy::prelude::{IVec3, Vec3};

use crate::block_edit::RayHit;

/// Casts a ray through the voxel grid (Amanatides-Woo DDA) and returns the first solid block.
///
/// # Arguments
/// * `origin` - Ray start in world space.
/// * `direction` - Ray direction; need not be normalized, zero returns `None`.
/// * `max_distance` - Maximum travel distance in blocks.
/// * `is_solid` - Lookup telling whether the block at a world coordinate is solid.
///
/// # Example
/// ```ignore
/// let hit = voxel_raycast(cam_pos, cam_forward, REACH_DISTANCE, |p| world_is_solid(p));
/// ```
pub fn voxel_raycast(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    is_solid: impl Fn(IVec3) -> bool,
) -> Option<RayHit> {
    let dir = direction.try_normalize()?;
    let mut cell = origin.floor().as_ivec3();
    let step = IVec3::new(
        dir.x.signum() as i32,
        dir.y.signum() as i32,
        dir.z.signum() as i32,
    );

    // Distance along the ray to cross one cell, and to reach the first boundary, per axis.
    let mut t_max = Vec3::ZERO;
    let mut t_delta = Vec3::ZERO;
    for axis in 0..3 {
        if dir[axis] == 0.0 {
            t_max[axis] = f32::INFINITY;
            t_delta[axis] = f32::INFINITY;
        } else {
            let next_boundary = if step[axis] > 0 {
                cell[axis] as f32 + 1.0
            } else {
                cell[axis] as f32
            };
            t_max[axis] = (next_boundary - origin[axis]) / dir[axis];
            t_delta[axis] = 1.0 / dir[axis].abs();
        }
    }

    // Starting inside a solid block gives no meaningful face; treat as no hit.
    let mut normal = IVec3::ZERO;
    loop {
        if normal != IVec3::ZERO && is_solid(cell) {
            return Some(RayHit { block: cell, normal });
        }
        let axis = if t_max.x <= t_max.y && t_max.x <= t_max.z {
            0
        } else if t_max.y <= t_max.z {
            1
        } else {
            2
        };
        if t_max[axis] > max_distance {
            return None;
        }
        cell[axis] += step[axis];
        t_max[axis] += t_delta[axis];
        normal = IVec3::ZERO;
        normal[axis] = -step[axis];
    }
}
