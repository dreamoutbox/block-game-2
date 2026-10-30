use bevy::math::Vec3;
use bevy::prelude::*;
use block_game_2::fly_cam::FlyCamera;
use block_game_2::settings::{
    MINECRAFT_SKY_COLOR, SUN_LIGHT_ILLUMINANCE, SUN_LIGHT_POSITION, SUN_LIGHT_TARGET,
};
use block_game_2::streaming::{Player, VoxelWorldPlugin};

fn main() {
    App::new()
        .insert_resource(ClearColor(MINECRAFT_SKY_COLOR))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Voxel Engine (Minecraft Creative Fly & Minetest Techniques)".into(),
                // Resize the canvas with the browser page (no-op on desktop).
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(VoxelWorldPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

/// Spawns first-person creative flight camera and directional sunlight.
fn setup_scene(mut commands: Commands) {
    // First-person creative-mode fly camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 32.0, 32.0).looking_at(Vec3::new(0.0, 16.0, 0.0), Vec3::Y),
        Player,
        FlyCamera::default(),
    ));

    // Directional sunlight configured via settings
    commands.spawn((
        DirectionalLight {
            illuminance: SUN_LIGHT_ILLUMINANCE,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(SUN_LIGHT_POSITION).looking_at(SUN_LIGHT_TARGET, Vec3::Y),
    ));
}
