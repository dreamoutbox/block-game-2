use bevy::prelude::*;
use std::collections::HashMap;

use crate::chunk::BlockId;

/// Six cardinal directions / faces of a cubic voxel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockFace {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl BlockFace {
    pub const ALL: [BlockFace; 6] = [
        BlockFace::PosX,
        BlockFace::NegX,
        BlockFace::PosY,
        BlockFace::NegY,
        BlockFace::PosZ,
        BlockFace::NegZ,
    ];

    /// Returns normal offset (dx, dy, dz) for this face.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::registry::BlockFace;
    /// use bevy::prelude::IVec3;
    ///
    /// assert_eq!(BlockFace::PosY.normal(), IVec3::new(0, 1, 0));
    /// ```
    pub fn normal(&self) -> IVec3 {
        match self {
            BlockFace::PosX => IVec3::new(1, 0, 0),
            BlockFace::NegX => IVec3::new(-1, 0, 0),
            BlockFace::PosY => IVec3::new(0, 1, 0),
            BlockFace::NegY => IVec3::new(0, -1, 0),
            BlockFace::PosZ => IVec3::new(0, 0, 1),
            BlockFace::NegZ => IVec3::new(0, 0, -1),
        }
    }
}

/// Standard predefined block types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u16)]
pub enum BlockType {
    #[default]
    Air = 0,
    Stone = 1,
    Dirt = 2,
    Grass = 3,
    Wood = 4,
    Leaves = 5,
}

impl From<BlockType> for BlockId {
    fn from(block_type: BlockType) -> Self {
        BlockId(block_type as u16)
    }
}

/// Physical and visual descriptor for a block type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDescriptor {
    pub is_opaque: bool,
    pub is_solid: bool,
    pub texture_indices: [u32; 6],
}

impl BlockDescriptor {
    /// Creates a block descriptor.
    ///
    /// # Arguments
    /// * `is_opaque` - Whether this block fully occludes light and neighbor faces.
    /// * `is_solid` - Whether this block has collision geometry.
    /// * `texture_indices` - Atlas texture index for each face in order of `BlockFace::ALL`.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::registry::BlockDescriptor;
    ///
    /// let descriptor = BlockDescriptor::new(true, true, [0; 6]);
    /// assert!(descriptor.is_opaque);
    /// ```
    pub fn new(is_opaque: bool, is_solid: bool, texture_indices: [u32; 6]) -> Self {
        Self {
            is_opaque,
            is_solid,
            texture_indices,
        }
    }
}

/// Resource mapping block identifiers to their physical and visual properties.
#[derive(Resource, Debug, Clone, Default)]
pub struct BlockRegistry {
    descriptors: HashMap<BlockId, BlockDescriptor>,
}

impl BlockRegistry {
    /// Registers a descriptor for a given block ID.
    ///
    /// # Arguments
    /// * `id` - Block identifier.
    /// * `descriptor` - Associated physical and visual properties.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::registry::{BlockRegistry, BlockDescriptor};
    /// use block_game_2::chunk::BlockId;
    ///
    /// let mut registry = BlockRegistry::default();
    /// registry.register(BlockId(1), BlockDescriptor::new(true, true, [1; 6]));
    /// assert!(registry.is_opaque(BlockId(1)));
    /// ```
    pub fn register(&mut self, id: BlockId, descriptor: BlockDescriptor) {
        self.descriptors.insert(id, descriptor);
    }

    /// Retrieves descriptor for a block ID if registered.
    ///
    /// # Arguments
    /// * `id` - Block identifier to look up.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::registry::{BlockRegistry, BlockDescriptor};
    /// use block_game_2::chunk::BlockId;
    ///
    /// let mut registry = BlockRegistry::default();
    /// registry.register(BlockId(1), BlockDescriptor::new(true, true, [0; 6]));
    /// assert!(registry.get(BlockId(1)).is_some());
    /// ```
    pub fn get(&self, id: BlockId) -> Option<&BlockDescriptor> {
        self.descriptors.get(&id)
    }

    /// Returns whether the block ID is opaque, defaulting to false if unknown or air.
    ///
    /// # Arguments
    /// * `id` - Block identifier to check.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::registry::{BlockRegistry, BlockDescriptor};
    /// use block_game_2::chunk::BlockId;
    ///
    /// let mut registry = BlockRegistry::default();
    /// registry.register(BlockId(1), BlockDescriptor::new(true, true, [0; 6]));
    /// assert!(registry.is_opaque(BlockId(1)));
    /// assert!(!registry.is_opaque(BlockId(0)));
    /// ```
    pub fn is_opaque(&self, id: BlockId) -> bool {
        self.descriptors.get(&id).map(|d| d.is_opaque).unwrap_or(false)
    }

    /// Returns whether the block ID is solid, defaulting to false if unknown or air.
    ///
    /// # Arguments
    /// * `id` - Block identifier to check.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::registry::{BlockRegistry, BlockDescriptor};
    /// use block_game_2::chunk::BlockId;
    ///
    /// let mut registry = BlockRegistry::default();
    /// registry.register(BlockId(1), BlockDescriptor::new(true, true, [0; 6]));
    /// assert!(registry.is_solid(BlockId(1)));
    /// assert!(!registry.is_solid(BlockId(0)));
    /// ```
    pub fn is_solid(&self, id: BlockId) -> bool {
        self.descriptors.get(&id).map(|d| d.is_solid).unwrap_or(false)
    }

    /// Builds a default registry populated with common block types.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::registry::{BlockRegistry, BlockType};
    ///
    /// let registry = BlockRegistry::with_defaults();
    /// assert!(registry.is_opaque(BlockType::Stone.into()));
    /// assert!(!registry.is_opaque(BlockType::Air.into()));
    /// ```
    pub fn with_defaults() -> Self {
        let mut registry = Self::default();
        registry.register(
            BlockType::Air.into(),
            BlockDescriptor::new(false, false, [0; 6]),
        );
        registry.register(
            BlockType::Stone.into(),
            BlockDescriptor::new(true, true, [1; 6]),
        );
        registry.register(
            BlockType::Dirt.into(),
            BlockDescriptor::new(true, true, [2; 6]),
        );
        registry.register(
            BlockType::Grass.into(),
            BlockDescriptor::new(true, true, [3, 3, 4, 2, 3, 3]), // Top: 4 (grass), Bottom: 2 (dirt), Sides: 3 (grass side)
        );
        registry.register(
            BlockType::Wood.into(),
            BlockDescriptor::new(true, true, [5; 6]),
        );
        registry.register(
            BlockType::Leaves.into(),
            BlockDescriptor::new(false, true, [6; 6]),
        );
        registry
    }
}
