use crate::registry::BlockType;

/// A slot within the player's hotbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotbarSlot {
    /// The block type assigned to this slot.
    pub block_type: BlockType,
}

impl HotbarSlot {
    /// Creates a new hotbar slot with the given block type.
    pub const fn new(block_type: BlockType) -> Self {
        Self { block_type }
    }
}
