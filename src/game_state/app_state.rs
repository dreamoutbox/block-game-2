use bevy::prelude::*;

/// High-level application lifecycle state.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Main title, world selection, and world creation menu.
    #[default]
    MainMenu,
    /// Active in-world creative gameplay.
    InGame,
    /// In-game pause menu with camera frozen and cursor released.
    Paused,
}
