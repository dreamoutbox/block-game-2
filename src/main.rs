use bevy::math::Vec3;
use bevy::prelude::*;
use block_game_2::streaming::{Player, VoxelWorldPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Voxel Engine (Minetest Techniques in Rust + Bevy)".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(VoxelWorldPlugin)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, move_player)
        .run();
}

/// Spawns player camera and directional sunlight.
fn setup_scene(mut commands: Commands) {
    // Camera representing the player
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 30.0, 30.0).looking_at(Vec3::new(0.0, 16.0, 0.0), Vec3::Y),
        Player,
    ));

    // Directional sunlight
    commands.spawn((
        DirectionalLight {
            illuminance: 15_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(50.0, 80.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Slowly moves player camera to demonstrate dynamic chunk streaming.
fn move_player(time: Res<Time>, mut player_query: Query<&mut Transform, With<Player>>) {
    for mut transform in player_query.iter_mut() {
        transform.translation.x += 4.0 * time.delta_secs();
    }
}
