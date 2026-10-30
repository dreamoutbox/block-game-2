use bevy::prelude::{IVec3, Vec3};

use crate::collision::{Aabb3d, aabb_overlaps_block, resolve_voxel_collision};

#[test]
fn test_aabb3d_intersects() {
    let a = Aabb3d::from_center_half_extents(Vec3::ZERO, Vec3::splat(0.5));
    let b = Aabb3d::from_center_half_extents(Vec3::new(0.8, 0.0, 0.0), Vec3::splat(0.5));
    let c = Aabb3d::from_center_half_extents(Vec3::new(1.2, 0.0, 0.0), Vec3::splat(0.5));

    assert!(a.intersects(&b));
    assert!(!a.intersects(&c));
}

#[test]
fn test_aabb_overlaps_block() {
    let half_extents = Vec3::splat(0.3);
    // Camera at (0.5, 0.5, 0.5) overlaps unit block [0, 1)
    assert!(aabb_overlaps_block(Vec3::new(0.5, 0.5, 0.5), half_extents, IVec3::new(0, 0, 0)));

    // Camera at (1.5, 0.5, 0.5) does not overlap unit block [0, 1)
    assert!(!aabb_overlaps_block(Vec3::new(1.5, 0.5, 0.5), half_extents, IVec3::new(0, 0, 0)));

    // Camera at (1.2, 0.5, 0.5) has min.x = 0.9, which overlaps block [0, 1)
    assert!(aabb_overlaps_block(Vec3::new(1.2, 0.5, 0.5), half_extents, IVec3::new(0, 0, 0)));
}

#[test]
fn test_stopped_by_wall_on_each_axis() {
    let half_extents = Vec3::splat(0.3);

    // Target block at (1, 0, 0)
    let is_solid = |p: IVec3| p == IVec3::new(1, 0, 0);

    // Moving +X from 0.0 into block at X=1
    let pos = Vec3::new(0.0, 0.5, 0.5);
    let delta = Vec3::new(2.0, 0.0, 0.0);
    let res = resolve_voxel_collision(pos, delta, half_extents, is_solid);
    assert!(res.x < 1.0 - half_extents.x);
    assert_eq!(res.y, 0.5);
    assert_eq!(res.z, 0.5);

    // Target block at (-1, 0, 0)
    let is_solid_neg = |p: IVec3| p == IVec3::new(-1, 0, 0);
    let pos = Vec3::new(0.5, 0.5, 0.5);
    let delta = Vec3::new(-2.0, 0.0, 0.0);
    let res = resolve_voxel_collision(pos, delta, half_extents, is_solid_neg);
    assert!(res.x > 0.0 + half_extents.x);

    // Floor at Y = -1 (block is [-1, 0))
    let is_solid_floor = |p: IVec3| p.y == -1;
    let pos = Vec3::new(0.5, 1.0, 0.5);
    let delta = Vec3::new(0.0, -2.0, 0.0);
    let res = resolve_voxel_collision(pos, delta, half_extents, is_solid_floor);
    assert!(res.y >= half_extents.y);

    // Ceiling at Y = 2 (block is [2, 3))
    let is_solid_ceil = |p: IVec3| p.y == 2;
    let pos = Vec3::new(0.5, 1.0, 0.5);
    let delta = Vec3::new(0.0, 2.0, 0.0);
    let res = resolve_voxel_collision(pos, delta, half_extents, is_solid_ceil);
    assert!(res.y <= 2.0 - half_extents.y);

    // Wall at +Z = 1
    let is_solid_z = |p: IVec3| p == IVec3::new(0, 0, 1);
    let pos = Vec3::new(0.5, 0.5, 0.0);
    let delta = Vec3::new(0.0, 0.0, 2.0);
    let res = resolve_voxel_collision(pos, delta, half_extents, is_solid_z);
    assert!(res.z <= 1.0 - half_extents.z);

    // Wall at -Z = -1
    let is_solid_neg_z = |p: IVec3| p == IVec3::new(0, 0, -1);
    let pos = Vec3::new(0.5, 0.5, 0.5);
    let delta = Vec3::new(0.0, 0.0, -2.0);
    let res = resolve_voxel_collision(pos, delta, half_extents, is_solid_neg_z);
    assert!(res.z >= 0.0 + half_extents.z);
}

#[test]
fn test_diagonal_sliding_along_wall() {
    let half_extents = Vec3::splat(0.3);
    // Wall on the right at X = 1
    let is_solid = |p: IVec3| p.x == 1;

    let pos = Vec3::new(0.5, 0.5, 0.5);
    // Moving diagonally forward and right: +X and +Z
    let delta = Vec3::new(1.0, 0.0, 1.0);
    let res = resolve_voxel_collision(pos, delta, half_extents, is_solid);

    // X should be clamped before entering X=1
    assert!(res.x <= 1.0 - half_extents.x);
    // Z should slide uninhibited
    assert!((res.z - 1.5).abs() < 1e-4);
}

#[test]
fn test_corner_stops_movement_on_both_axes() {
    let half_extents = Vec3::splat(0.3);
    // Walls at X = 1 and Z = 1
    let is_solid = |p: IVec3| p.x == 1 || p.z == 1;

    let pos = Vec3::new(0.5, 0.5, 0.5);
    let delta = Vec3::new(1.0, 0.0, 1.0);
    let res = resolve_voxel_collision(pos, delta, half_extents, is_solid);

    assert!(res.x <= 1.0 - half_extents.x);
    assert!(res.z <= 1.0 - half_extents.z);
    assert_eq!(res.y, 0.5);
}

#[test]
fn test_large_delta_no_tunneling() {
    let half_extents = Vec3::splat(0.3);
    // A single 1-block thick wall at X = 2 (air at X=0, 1, 3, 4...)
    let is_solid = |p: IVec3| p.x == 2;

    let pos = Vec3::new(0.5, 0.5, 0.5);
    // Moving with high speed across the wall
    let delta = Vec3::new(10.0, 0.0, 0.0);
    let res = resolve_voxel_collision(pos, delta, half_extents, is_solid);

    // Must be stopped in front of the wall at X=2, not tunnel through to X=10.5
    assert!(res.x <= 2.0 - half_extents.x);
    assert!(res.x >= 1.0);
}
