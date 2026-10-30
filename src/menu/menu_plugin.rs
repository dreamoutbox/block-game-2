use bevy::prelude::*;

use crate::game_state::AppState;
use crate::menu::main_menu_ui::main_menu_ui_system;
use crate::menu::menu_state::MenuState;
use crate::menu::pause_menu_ui::pause_menu_ui_system;

/// Plugin registering UI systems for main menu and pause menu screens.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .add_systems(Update, main_menu_ui_system.run_if(in_state(AppState::MainMenu)))
            .add_systems(Update, pause_menu_ui_system.run_if(in_state(AppState::Paused)));
    }
}
