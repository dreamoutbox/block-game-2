use std::path::{Path, PathBuf};
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::game_state::AppState;
use crate::menu::MenuState;
use crate::save::{create_world, delete_world, list_worlds, ActiveWorld, WorldMeta};
use crate::worldgen::{TerrainConfig, TerrainGenerator};

/// Base directory path where world save folders are stored.
pub const SAVES_DIR: &str = "saves";

/// System rendering the main menu UI using egui.
#[allow(clippy::too_many_arguments)]
pub fn main_menu_ui_system(
    mut egui_query: Query<&mut EguiContext>,
    mut menu_state: ResMut<MenuState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    #[allow(unused_mut, unused_variables)] mut app_exit: Option<MessageWriter<AppExit>>,
) {
    let Ok(mut egui_ctx) = egui_query.single_mut() else {
        return;
    };
    let ctx = egui_ctx.get_mut();

    egui::Window::new("Block Game 2 - Main Menu")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("BLOCK GAME 2");
                ui.label("Voxel Engine with Save & Load");
                #[cfg(target_arch = "wasm32")]
                ui.colored_label(
                    egui::Color32::from_rgb(200, 200, 100),
                    "WebAssembly mode: file persistence is unavailable.",
                );
            });

            ui.add_space(8.0);

            // Display status or error messages
            if let Some(ref status) = menu_state.status_message {
                ui.colored_label(egui::Color32::from_rgb(100, 220, 100), status);
            }
            if let Some(ref err) = menu_state.error_message {
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
            }

            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new("Saved Worlds").strong());

            let saves_path = Path::new(SAVES_DIR);
            let worlds = list_worlds(saves_path).unwrap_or_default();

            if worlds.is_empty() {
                ui.label("No saved worlds found.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(120.0)
                    .show(ui, |ui| {
                        for meta in &worlds {
                            let is_selected = menu_state
                                .selected_world
                                .as_ref()
                                .is_some_and(|s| s == &meta.name);

                            let label = format!(
                                "{} (Seed: {}, Pos: [{:.0}, {:.0}, {:.0}])",
                                meta.name,
                                meta.terrain_config.seed,
                                meta.player_position[0],
                                meta.player_position[1],
                                meta.player_position[2]
                            );

                            if ui.selectable_label(is_selected, label).clicked() {
                                menu_state.selected_world = Some(meta.name.clone());
                            }
                        }
                    });

                ui.horizontal(|ui| {
                    let has_selection = menu_state.selected_world.is_some();

                    if ui
                        .add_enabled(has_selection, egui::Button::new("Play Selected"))
                        .clicked()
                        && let Some(ref name) = menu_state.selected_world
                        && let Some(meta) = worlds.iter().find(|w| &w.name == name)
                    {
                        start_game_with_world(&mut commands, &mut next_state, meta.clone());
                    }

                    if ui
                        .add_enabled(has_selection, egui::Button::new("Delete"))
                        .clicked()
                        && let Some(name) = menu_state.selected_world.clone()
                    {
                        match delete_world(saves_path, &name) {
                            Ok(()) => {
                                menu_state.status_message =
                                    Some(format!("Deleted world '{name}'."));
                                menu_state.error_message = None;
                                menu_state.selected_world = None;
                            }
                            Err(err) => {
                                menu_state.error_message = Some(err.to_string());
                            }
                        }
                    }
                });
            }

            ui.add_space(10.0);
            ui.separator();
            ui.label(egui::RichText::new("Create New World").strong());

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut menu_state.new_world_name);
            });

            ui.horizontal(|ui| {
                ui.label("Seed:");
                ui.text_edit_singleline(&mut menu_state.new_world_seed);
            });

            if ui.button("Create & Play World").clicked() {
                let name = menu_state.new_world_name.trim().to_string();
                let seed = menu_state
                    .new_world_seed
                    .trim()
                    .parse::<u32>()
                    .unwrap_or(42);

                match create_world(saves_path, &name, seed) {
                    Ok(meta) => {
                        menu_state.error_message = None;
                        menu_state.status_message = None;
                        start_game_with_world(&mut commands, &mut next_state, meta);
                    }
                    Err(err) => {
                        menu_state.error_message = Some(err.to_string());
                    }
                }
            }

            ui.add_space(8.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Quick Play (Default)").clicked() {
                    let config = TerrainConfig {
                        seed: 42,
                        ..Default::default()
                    };
                    let meta = WorldMeta::new("QuickWorld".into(), config, [0.0, 32.0, 32.0]);
                    start_game_with_world(&mut commands, &mut next_state, meta);
                }

                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Quit Game").clicked()
                    && let Some(ref mut exit) = app_exit
                {
                    exit.write(AppExit::Success);
                }
            });
        });
}

/// Helper function to configure active world resources and switch application state to `InGame`.
pub fn start_game_with_world(
    commands: &mut Commands,
    next_state: &mut NextState<AppState>,
    meta: WorldMeta,
) {
    let dir = PathBuf::from(SAVES_DIR).join(&meta.name);
    let terrain_config = meta.terrain_config.clone();
    commands.insert_resource(ActiveWorld::new(meta.name.clone(), dir, meta));
    commands.insert_resource(TerrainGenerator::new(terrain_config));
    next_state.set(AppState::InGame);
}
