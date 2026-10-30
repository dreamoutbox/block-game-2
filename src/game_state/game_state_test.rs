use bevy::prelude::*;

use crate::chunk::ChunkPos;
use crate::game_state::{AppState, GameStatePlugin, InGameEntity, MenuCamera};
use crate::streaming::Player;
use crate::world::VoxelWorld;

#[test]
fn test_app_state_transitions_and_world_cleanup() {
    let mut app = App::new();
    app.init_resource::<VoxelWorld>()
        .add_plugins(GameStatePlugin);

    app.update();

    // Default state is MainMenu
    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::MainMenu);
    // MenuCamera should exist and have PrimaryEguiContext
    assert_eq!(app.world_mut().query_filtered::<Entity, With<MenuCamera>>().iter(app.world()).count(), 1);
    assert_eq!(app.world_mut().query_filtered::<Entity, With<bevy_egui::PrimaryEguiContext>>().iter(app.world()).count(), 1);
    // Player should not exist
    assert_eq!(app.world_mut().query_filtered::<Entity, With<Player>>().iter(app.world()).count(), 0);

    // Enter InGame
    app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::InGame);
    app.update();

    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);
    // MenuCamera despawned
    assert_eq!(app.world_mut().query_filtered::<Entity, With<MenuCamera>>().iter(app.world()).count(), 0);
    // Player spawned with PrimaryEguiContext
    assert_eq!(app.world_mut().query_filtered::<Entity, With<Player>>().iter(app.world()).count(), 1);
    assert_eq!(app.world_mut().query_filtered::<Entity, With<bevy_egui::PrimaryEguiContext>>().iter(app.world()).count(), 1);
    // InGameEntities (Player + Light) exist
    assert_eq!(app.world_mut().query_filtered::<Entity, With<InGameEntity>>().iter(app.world()).count(), 2);

    // Simulate chunk in VoxelWorld
    let fake_chunk = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<VoxelWorld>().insert(ChunkPos::new(0, 0, 0), fake_chunk);
    assert_eq!(app.world().resource::<VoxelWorld>().len(), 1);

    // Transition to Paused
    app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::Paused);
    app.update();

    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::Paused);
    // Entities and chunks still exist, PrimaryEguiContext persists for pause menu
    assert_eq!(app.world().resource::<VoxelWorld>().len(), 1);
    assert_eq!(app.world_mut().query_filtered::<Entity, With<Player>>().iter(app.world()).count(), 1);
    assert_eq!(app.world_mut().query_filtered::<Entity, With<bevy_egui::PrimaryEguiContext>>().iter(app.world()).count(), 1);

    // Transition back to MainMenu
    app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::MainMenu);
    app.update();

    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::MainMenu);
    // VoxelWorld is cleaned up
    assert_eq!(app.world().resource::<VoxelWorld>().len(), 0);
    // Player & InGameEntity are despawned
    assert_eq!(app.world_mut().query_filtered::<Entity, With<Player>>().iter(app.world()).count(), 0);
    assert_eq!(app.world_mut().query_filtered::<Entity, With<InGameEntity>>().iter(app.world()).count(), 0);
    // MenuCamera is restored with PrimaryEguiContext
    assert_eq!(app.world_mut().query_filtered::<Entity, With<MenuCamera>>().iter(app.world()).count(), 1);
    assert_eq!(app.world_mut().query_filtered::<Entity, With<bevy_egui::PrimaryEguiContext>>().iter(app.world()).count(), 1);
}

#[test]
fn test_cursor_grab_pause_and_unpause() {
    let mut app = App::new();
    app.init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<VoxelWorld>()
        .add_plugins(GameStatePlugin)
        .add_systems(Update, crate::fly_cam::cursor_grab_system);

    // Spawn window with CursorOptions
    app.world_mut().spawn((
        bevy::window::Window::default(),
        bevy::window::PrimaryWindow,
        bevy::window::CursorOptions {
            visible: true,
            grab_mode: bevy::window::CursorGrabMode::None,
            hit_test: true,
        },
    ));

    app.update();

    // Transition to InGame
    app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::InGame);
    app.update();

    // Simulate clicking in-game -> cursor becomes locked
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<MouseButton>>().reset_all();

    let cursor = app
        .world_mut()
        .query_filtered::<&bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .expect("primary window cursor");
    assert_eq!(cursor.grab_mode, bevy::window::CursorGrabMode::Locked);
    assert!(!cursor.visible);

    // Simulate pressing Escape -> transitions to Paused and unlocks cursor
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
    app.update(); // cursor_grab_system runs in Update and queues NextState::Pending(Paused)
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.update(); // State transition applies in PreUpdate

    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::Paused);
    let cursor = app
        .world_mut()
        .query_filtered::<&bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .expect("primary window cursor");
    assert_eq!(cursor.grab_mode, bevy::window::CursorGrabMode::None);
    assert!(cursor.visible);

    // Simulate pressing Escape while Paused -> returns to InGame and locks cursor
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.update();

    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);
    let cursor = app
        .world_mut()
        .query_filtered::<&bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .expect("primary window cursor");
    assert_eq!(cursor.grab_mode, bevy::window::CursorGrabMode::Locked);
    assert!(!cursor.visible);

    // Simulate releasing Escape (browser pointer-lock exit) -> pauses game
    // First mark Escape as pressed, then release it
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::Escape);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.update();
    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::Paused);

    // Simulate KeyP to unpause and pause
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyP);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.update();
    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);

    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyP);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
    app.update();
    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::Paused);
}
