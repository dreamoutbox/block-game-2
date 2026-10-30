use bevy::prelude::*;
use block_game_2::settings::MINECRAFT_SKY_COLOR;
use block_game_2::streaming::VoxelWorldPlugin;

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
        .run();
}
