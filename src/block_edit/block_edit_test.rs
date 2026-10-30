use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::block_edit::{get_block_at, BlockEditPlugin, SelectedBlock};
use crate::chunk::{BlockId, Chunk, ChunkPos};
use crate::fly_cam::FlyCamera;
use crate::registry::{BlockRegistry, BlockType};
use crate::world::VoxelWorld;

#[test]
fn input_does_nothing_when_cursor_unlocked() {
    let mut app = App::new();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<VoxelWorld>();
    app.insert_resource(BlockRegistry::with_defaults());
    app.add_plugins(BlockEditPlugin);

    // Primary window with None grab mode
    app.world_mut().spawn((
        PrimaryWindow,
        CursorOptions {
            grab_mode: CursorGrabMode::None,
            visible: true,
            ..default()
        },
    ));

    // Camera pointing at (0, 0, 0) from (0, 0, 3)
    app.world_mut().spawn((
        FlyCamera::default(),
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Spawn chunk at 0,0,0
    let chunk_entity = app.world_mut().spawn((ChunkPos::new(0, 0, 0), Chunk::new(BlockId(1)))).id();
    app.world_mut().resource_mut::<VoxelWorld>().insert(ChunkPos::new(0, 0, 0), chunk_entity);

    // Press left click
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);

    app.update();

    // Block at 0,0,0 should still be stone (1), not broken to air (0)
    let b = app
        .world_mut()
        .run_system_once(|vw: Res<VoxelWorld>, q: Query<&Chunk>| {
            get_block_at(&vw, &q, IVec3::ZERO)
        })
        .unwrap();
    assert_eq!(b, Some(BlockId(1)));
}

#[test]
fn input_breaks_and_places_when_cursor_locked() {
    let mut app = App::new();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<VoxelWorld>();
    app.insert_resource(BlockRegistry::with_defaults());
    app.add_plugins(BlockEditPlugin);

    // Primary window with Locked grab mode
    app.world_mut().spawn((
        PrimaryWindow,
        CursorOptions {
            grab_mode: CursorGrabMode::Locked,
            visible: false,
            ..default()
        },
    ));

    // Camera at (0.5, 0.5, 2.5) looking towards -Z: at (0, 0, 0)
    app.world_mut().spawn((
        FlyCamera::default(),
        Transform::from_xyz(0.5, 0.5, 2.5).looking_to(Vec3::NEG_Z, Vec3::Y),
    ));

    // Spawn chunk at 0,0,0
    let chunk_entity = app.world_mut().spawn((ChunkPos::new(0, 0, 0), Chunk::new(BlockId(0)))).id();
    app.world_mut().resource_mut::<VoxelWorld>().insert(ChunkPos::new(0, 0, 0), chunk_entity);

    // Set (0, 0, 0) to Stone
    app.world_mut().get_mut::<Chunk>(chunk_entity).unwrap().set_block(0, 0, 0, BlockId(1));

    // Left click to break (0, 0, 0)
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().clear_just_pressed(MouseButton::Left);

    let b = app
        .world_mut()
        .run_system_once(|vw: Res<VoxelWorld>, q: Query<&Chunk>| {
            get_block_at(&vw, &q, IVec3::new(0, 0, 0))
        })
        .unwrap();
    assert_eq!(b, Some(BlockType::Air.into()));

    // Put stone back to test placing
    app.world_mut().get_mut::<Chunk>(chunk_entity).unwrap().set_block(0, 0, 0, BlockId(1));

    // Right click to place on hit face (which will be +Z face: 0, 0, 1)
    app.world_mut().resource_mut::<SelectedBlock>().0 = BlockType::Dirt;
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Right);
    app.update();

    let placed = app
        .world_mut()
        .run_system_once(|vw: Res<VoxelWorld>, q: Query<&Chunk>| {
            get_block_at(&vw, &q, IVec3::new(0, 0, 1))
        })
        .unwrap();
    assert_eq!(placed, Some(BlockType::Dirt.into()));
}
