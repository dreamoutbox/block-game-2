use bevy::prelude::*;

/// Persistent UI state for main menu and pause menu forms.
#[derive(Resource, Debug, Clone)]
pub struct MenuState {
    /// Name input text field for world creation.
    pub new_world_name: String,
    /// Seed input text field for world creation.
    pub new_world_seed: String,
    /// Name of currently selected saved world in the list.
    pub selected_world: Option<String>,
    /// Informational message to display in the menu.
    pub status_message: Option<String>,
    /// Error message to display in the menu.
    pub error_message: Option<String>,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            new_world_name: "World_1".to_string(),
            new_world_seed: "42".to_string(),
            selected_world: None,
            status_message: None,
            error_message: None,
        }
    }
}
