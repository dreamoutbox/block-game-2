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
    // MenuCamera should exist
    assert_eq!(app.world_mut().query_filtered::<Entity, With<MenuCamera>>().iter(app.world()).count(), 1);
    // Player should not exist
    assert_eq!(app.world_mut().query_filtered::<Entity, With<Player>>().iter(app.world()).count(), 0);

    // Enter InGame
    app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::InGame);
    app.update();

    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);
    // MenuCamera despawned
    assert_eq!(app.world_mut().query_filtered::<Entity, With<MenuCamera>>().iter(app.world()).count(), 0);
    // Player spawned
    assert_eq!(app.world_mut().query_filtered::<Entity, With<Player>>().iter(app.world()).count(), 1);
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
    // Entities and chunks still exist
    assert_eq!(app.world().resource::<VoxelWorld>().len(), 1);
    assert_eq!(app.world_mut().query_filtered::<Entity, With<Player>>().iter(app.world()).count(), 1);

    // Transition back to MainMenu
    app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::MainMenu);
    app.update();

    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::MainMenu);
    // VoxelWorld is cleaned up
    assert_eq!(app.world().resource::<VoxelWorld>().len(), 0);
    // Player & InGameEntity are despawned
    assert_eq!(app.world_mut().query_filtered::<Entity, With<Player>>().iter(app.world()).count(), 0);
    assert_eq!(app.world_mut().query_filtered::<Entity, With<InGameEntity>>().iter(app.world()).count(), 0);
    // MenuCamera is restored
    assert_eq!(app.world_mut().query_filtered::<Entity, With<MenuCamera>>().iter(app.world()).count(), 1);
}
