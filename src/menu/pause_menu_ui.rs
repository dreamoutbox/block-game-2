use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

use crate::chunk::{Chunk, ChunkPos};
use crate::game_state::AppState;
use crate::menu::MenuState;
use crate::save::{save_active_world, ActiveWorld, ChunkModified};
use crate::streaming::Player;

/// System rendering the in-game pause menu UI using egui.
#[allow(clippy::too_many_arguments)]
pub fn pause_menu_ui_system(
    mut egui_query: Query<&mut EguiContext>,
    mut menu_state: ResMut<MenuState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    mut active_world: Option<ResMut<ActiveWorld>>,
    player_query: Query<&Transform, With<Player>>,
    chunks: Query<&Chunk>,
    modified_chunks: Query<(Entity, &ChunkPos), With<ChunkModified>>,
) {
    let Ok(mut egui_ctx) = egui_query.single_mut() else {
        return;
    };
    let ctx = egui_ctx.get_mut();

    egui::Window::new("Game Paused")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(260.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("PAUSED");

                if let Some(ref active) = active_world {
                    ui.label(format!("World: {}", active.name));
                } else {
                    ui.label("World: [In-Memory]");
                }
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

            if ui.button("Resume Game").clicked() {
                menu_state.status_message = None;
                menu_state.error_message = None;
                next_state.set(AppState::InGame);
            }

            ui.add_space(4.0);

            if ui.button("Save World").clicked() {
                if let Some(ref mut active) = active_world {
                    let player_pos = player_query
                        .single()
                        .ok()
                        .map(|t| t.translation.to_array());

                    match save_active_world(
                        active,
                        player_pos,
                        &chunks,
                        &modified_chunks,
                        &mut commands,
                    ) {
                        Ok(count) => {
                            menu_state.status_message =
                                Some(format!("Saved {count} chunk(s) successfully!"));
                            menu_state.error_message = None;
                        }
                        Err(err) => {
                            menu_state.error_message = Some(format!("Save error: {err}"));
                        }
                    }
                } else {
                    menu_state.status_message =
                        Some("In-memory world (no active save directory).".into());
                }
            }

            ui.add_space(4.0);

            if ui.button("Save & Quit to Title").clicked() {
                if let Some(ref mut active) = active_world {
                    let player_pos = player_query
                        .single()
                        .ok()
                        .map(|t| t.translation.to_array());
                    let _ = save_active_world(
                        active,
                        player_pos,
                        &chunks,
                        &modified_chunks,
                        &mut commands,
                    );
                }
                menu_state.status_message = None;
                menu_state.error_message = None;
                next_state.set(AppState::MainMenu);
            }

            ui.add_space(4.0);

            if ui.button("Quit Without Saving").clicked() {
                menu_state.status_message = None;
                menu_state.error_message = None;
                next_state.set(AppState::MainMenu);
            }
        });
}
