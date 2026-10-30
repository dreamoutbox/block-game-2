use bevy::prelude::*;

/// Resource tracking the player hotbar state and active selection index.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct Hotbar {
    /// Available hotbar slots.
    pub slots: Vec<crate::hotbar::HotbarSlot>,
    /// Currently selected slot index.
    pub selected_index: usize,
}

impl Default for Hotbar {
    fn default() -> Self {
        use crate::hotbar::HotbarSlot;
        use crate::registry::BlockType;
        Self {
            slots: vec![
                HotbarSlot::new(BlockType::Grass),
                HotbarSlot::new(BlockType::Dirt),
            ],
            selected_index: 0,
        }
    }
}

impl Hotbar {
    /// Returns the currently selected block type if slots are available.
    pub fn selected_block(&self) -> Option<crate::registry::BlockType> {
        self.slots.get(self.selected_index).map(|s| s.block_type)
    }

    /// Selects a slot index with wrapping.
    pub fn select_slot(&mut self, index: usize) {
        if !self.slots.is_empty() {
            self.selected_index = index % self.slots.len();
        }
    }

    /// Cycles the selection forward by one slot with wrapping.
    pub fn cycle_next(&mut self) {
        if !self.slots.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.slots.len();
        }
    }

    /// Cycles the selection backward by one slot with wrapping.
    pub fn cycle_prev(&mut self) {
        if !self.slots.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.slots.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }
}
