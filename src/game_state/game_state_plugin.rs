use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

use crate::fly_cam::FlyCamera;
use crate::game_state::app_state::AppState;
use crate::game_state::in_game_entity::InGameEntity;
use crate::game_state::menu_camera::MenuCamera;
use crate::save::ActiveWorld;
use crate::settings::{
    SUN_LIGHT_ILLUMINANCE, SUN_LIGHT_POSITION, SUN_LIGHT_TARGET,
};
use crate::streaming::Player;
use crate::world::VoxelWorld;

/// Run condition returning true if either no `AppState` resource exists (e.g. in isolated unit tests)
/// or the current state is `AppState::InGame`.
///
/// # Arguments
/// * `state` - Current Bevy state resource.
///
/// # Example
/// ```rust
/// use bevy::prelude::*;
/// use block_game_2::game_state::in_game_or_no_state;
///
/// let mut app = App::new();
/// app.add_systems(Update, (|| {}).run_if(in_game_or_no_state));
/// ```
pub fn in_game_or_no_state(state: Option<Res<State<AppState>>>) -> bool {
    state.is_none_or(|s| *s.get() == AppState::InGame)
}

/// Sets up the 2D menu camera and ensures cursor is visible for main menu navigation.
pub fn setup_main_menu_system(
    mut commands: Commands,
    camera_query: Query<Entity, With<MenuCamera>>,
    mut cursor_query: Query<&mut bevy::window::CursorOptions, With<PrimaryWindow>>,
) {
    if camera_query.is_empty() {
        commands.spawn((Camera2d, MenuCamera));
    }
    if let Ok(mut cursor) = cursor_query.single_mut() {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}

/// Cleans up the 2D menu camera upon exiting the main menu.
pub fn cleanup_main_menu_system(
    mut commands: Commands,
    camera_query: Query<Entity, With<MenuCamera>>,
) {
    for entity in camera_query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Sets up the 3D player camera and directional sunlight when entering gameplay.
pub fn setup_in_game_system(
    mut commands: Commands,
    active_world: Option<Res<ActiveWorld>>,
    player_query: Query<Entity, With<Player>>,
) {
    // Only spawn scene entities if player does not already exist (e.g. resuming from Paused)
    if player_query.is_empty() {
        let spawn_pos = active_world
            .as_ref()
            .map(|w| Vec3::from_array(w.meta.player_position))
            .unwrap_or(Vec3::new(0.0, 32.0, 32.0));

        // First-person creative-mode fly camera
        commands.spawn((
            Camera3d::default(),
            Transform::from_translation(spawn_pos).looking_at(Vec3::new(0.0, 16.0, 0.0), Vec3::Y),
            Player,
            FlyCamera::default(),
            InGameEntity,
        ));

        // Directional sunlight
        commands.spawn((
            DirectionalLight {
                illuminance: SUN_LIGHT_ILLUMINANCE,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_translation(SUN_LIGHT_POSITION).looking_at(SUN_LIGHT_TARGET, Vec3::Y),
            InGameEntity,
        ));
    }
}

/// Unlocks cursor and makes it visible upon entering the paused state.
pub fn setup_paused_system(
    mut cursor_query: Query<&mut bevy::window::CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(mut cursor) = cursor_query.single_mut() {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}

/// Despawns all in-game world chunks and player entities when returning to the main menu.
pub fn cleanup_in_game_world_system(
    mut commands: Commands,
    mut voxel_world: ResMut<VoxelWorld>,
    in_game_query: Query<Entity, With<InGameEntity>>,
) {
    // Despawn in-game entities (player camera, sunlight)
    for entity in in_game_query.iter() {
        commands.entity(entity).despawn();
    }

    // Despawn all loaded chunk entities and clear map
    for (_pos, entity) in voxel_world.drain() {
        commands.entity(entity).despawn();
    }

    // Remove active world resource
    commands.remove_resource::<ActiveWorld>();
}

/// Plugin managing game lifecycle states, scene transitions, and resource cleanup.
pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::state::app::StatesPlugin>() {
            app.add_plugins(bevy::state::app::StatesPlugin);
        }

        if !app.world().contains_resource::<State<AppState>>() {
            app.init_state::<AppState>();
        }

        app.add_systems(
            OnEnter(AppState::MainMenu),
            (cleanup_in_game_world_system, setup_main_menu_system).chain(),
        )
        .add_systems(OnExit(AppState::MainMenu), cleanup_main_menu_system)
        .add_systems(OnEnter(AppState::InGame), setup_in_game_system)
        .add_systems(OnEnter(AppState::Paused), setup_paused_system);
    }
}
