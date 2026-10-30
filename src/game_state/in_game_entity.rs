use bevy::prelude::*;

/// Marker component for entities that only exist while an active world is loaded.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InGameEntity;
