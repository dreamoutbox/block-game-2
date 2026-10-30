use bevy::prelude::*;

use crate::block_edit::{RayHit, voxel_raycast};

fn solid_at(target: IVec3) -> impl Fn(IVec3) -> bool {
    move |p| p == target
}

#[test]
fn hits_along_each_axis_with_correct_normal() {
    let target = IVec3::new(5, 5, 5);
    let center = Vec3::new(5.5, 5.5, 5.5);
    for (dir, normal) in [
        (Vec3::X, IVec3::NEG_X),
        (Vec3::NEG_X, IVec3::X),
        (Vec3::Y, IVec3::NEG_Y),
        (Vec3::NEG_Y, IVec3::Y),
        (Vec3::Z, IVec3::NEG_Z),
        (Vec3::NEG_Z, IVec3::Z),
    ] {
        let origin = center - dir * 3.0;
        let hit = voxel_raycast(origin, dir, 5.0, solid_at(target));
        assert_eq!(hit, Some(RayHit { block: target, normal }), "dir {dir}");
    }
}

#[test]
fn misses_beyond_reach() {
    let hit = voxel_raycast(Vec3::new(0.5, 0.5, 0.5), Vec3::X, 3.0, solid_at(IVec3::new(10, 0, 0)));
    assert_eq!(hit, None);
}

#[test]
fn handles_negative_coordinates() {
    let target = IVec3::new(-3, -2, -1);
    let hit = voxel_raycast(Vec3::new(0.5, -1.5, -0.5), Vec3::NEG_X, 5.0, solid_at(IVec3::new(-3, -2, -1)));
    assert_eq!(hit, Some(RayHit { block: target, normal: IVec3::X }));
}

#[test]
fn zero_direction_is_none() {
    assert_eq!(voxel_raycast(Vec3::ZERO, Vec3::ZERO, 5.0, |_| true), None);
}

#[test]
fn diagonal_ray_hits_floor() {
    let hit = voxel_raycast(Vec3::new(0.5, 3.5, 0.5), Vec3::new(1.0, -1.0, 0.0), 10.0, |p| p.y <= 0);
    let hit = hit.expect("hit");
    assert_eq!(hit.block.y, 0);
    assert_eq!(hit.normal, IVec3::Y);
}
