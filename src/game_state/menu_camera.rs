use bevy::prelude::*;

/// Marker component for the 2D camera active during main menu navigation.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MenuCamera;
