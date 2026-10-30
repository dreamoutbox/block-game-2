use bevy::prelude::*;
use tempfile::tempdir;

use crate::game_state::{AppState, GameStatePlugin};
use crate::menu::{start_game_with_world, MenuPlugin, MenuState};
use crate::save::{create_world, ActiveWorld};
use crate::world::VoxelWorld;
use crate::worldgen::TerrainGenerator;

#[test]
fn test_menu_state_defaults() {
    let state = MenuState::default();
    assert_eq!(state.new_world_name, "World_1");
    assert_eq!(state.new_world_seed, "42");
    assert!(state.selected_world.is_none());
    assert!(state.status_message.is_none());
    assert!(state.error_message.is_none());
}

#[test]
fn test_start_game_with_world_sets_active_resources_and_state() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let meta = create_world(root, "MenuTestWorld", 999).unwrap();

    let mut app = App::new();
    app.init_resource::<VoxelWorld>()
        .add_plugins(GameStatePlugin)
        .add_plugins(MenuPlugin);

    app.update();
    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::MainMenu);

    // Call start_game_with_world helper
    app.world_mut().resource_scope(|world, mut next_state: Mut<NextState<AppState>>| {
        let mut commands = world.commands();
        start_game_with_world(&mut commands, &mut next_state, meta);
    });

    app.update();

    // Verify active world and generator are inserted and state transitioned to InGame
    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::InGame);
    assert!(app.world().contains_resource::<ActiveWorld>());
    assert_eq!(app.world().resource::<ActiveWorld>().name, "MenuTestWorld");
    assert!(app.world().contains_resource::<TerrainGenerator>());
    assert_eq!(app.world().resource::<TerrainGenerator>().config().seed, 999);
}
