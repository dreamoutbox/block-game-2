use bevy::prelude::*;
use crate::registry::BlockType;

/// Resource tracking which block type is currently selected for placement.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedBlock(pub BlockType);

impl Default for SelectedBlock {
    fn default() -> Self {
        Self(BlockType::Grass)
    }
}
