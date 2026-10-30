use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

/// Numeric identifier for a voxel block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct BlockId(pub u16);

/// Chunk grid coordinate in world space.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ChunkPos(pub IVec3);

impl ChunkPos {
    /// Constructs a new chunk position from 3D coordinates.
    ///
    /// # Arguments
    /// * `x` - Chunk X coordinate.
    /// * `y` - Chunk Y coordinate.
    /// * `z` - Chunk Z coordinate.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::chunk::ChunkPos;
    ///
    /// let pos = ChunkPos::new(0, 1, -2);
    /// assert_eq!(pos.0.y, 1);
    /// ```
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }
}

/// Converts local 3D chunk coordinates (0..16, 0..16, 0..16) into a 1D flat array index.
///
/// # Arguments
/// * `x` - Local X coordinate (0 <= x < 16).
/// * `y` - Local Y coordinate (0 <= y < 16).
/// * `z` - Local Z coordinate (0 <= z < 16).
///
/// # Example
/// ```rust
/// use block_game_2::chunk::coords_to_index;
///
/// let index = coords_to_index(1, 2, 3);
/// assert_eq!(index, 1 + 2 * 16 + 3 * 256);
/// ```
#[inline]
pub fn coords_to_index(x: usize, y: usize, z: usize) -> usize {
    debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
    x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
}

/// Converts a 1D flat array index (0..4096) into local 3D chunk coordinates (x, y, z).
///
/// # Arguments
/// * `index` - 1D array index (0 <= index < 4096).
///
/// # Example
/// ```rust
/// use block_game_2::chunk::{coords_to_index, index_to_coords};
///
/// let (x, y, z) = index_to_coords(coords_to_index(5, 7, 9));
/// assert_eq!((x, y, z), (5, 7, 9));
/// ```
#[inline]
pub fn index_to_coords(index: usize) -> (usize, usize, usize) {
    debug_assert!(index < CHUNK_VOLUME);
    let x = index % CHUNK_SIZE;
    let y = (index / CHUNK_SIZE) % CHUNK_SIZE;
    let z = index / (CHUNK_SIZE * CHUNK_SIZE);
    (x, y, z)
}

/// Converts global world block coordinates into chunk position and local block coordinates.
///
/// # Arguments
/// * `world_pos` - Global world block coordinate.
///
/// # Example
/// ```rust
/// use block_game_2::chunk::{world_to_chunk_and_local, ChunkPos};
/// use bevy::prelude::IVec3;
///
/// let (chunk_pos, (lx, ly, lz)) = world_to_chunk_and_local(IVec3::new(-1, 16, 31));
/// assert_eq!(chunk_pos, ChunkPos::new(-1, 1, 1));
/// assert_eq!((lx, ly, lz), (15, 0, 15));
/// ```
pub fn world_to_chunk_and_local(world_pos: IVec3) -> (ChunkPos, (usize, usize, usize)) {
    let chunk_x = world_pos.x.div_euclid(CHUNK_SIZE_I32);
    let chunk_y = world_pos.y.div_euclid(CHUNK_SIZE_I32);
    let chunk_z = world_pos.z.div_euclid(CHUNK_SIZE_I32);

    let local_x = world_pos.x.rem_euclid(CHUNK_SIZE_I32) as usize;
    let local_y = world_pos.y.rem_euclid(CHUNK_SIZE_I32) as usize;
    let local_z = world_pos.z.rem_euclid(CHUNK_SIZE_I32) as usize;

    (
        ChunkPos::new(chunk_x, chunk_y, chunk_z),
        (local_x, local_y, local_z),
    )
}

/// Converts chunk position and local block coordinates into a global world block coordinate.
///
/// # Arguments
/// * `chunk_pos` - Target chunk position.
/// * `local_x` - Local X coordinate within the chunk (0..16).
/// * `local_y` - Local Y coordinate within the chunk (0..16).
/// * `local_z` - Local Z coordinate within the chunk (0..16).
///
/// # Example
/// ```rust
/// use block_game_2::chunk::{chunk_and_local_to_world, ChunkPos};
/// use bevy::prelude::IVec3;
///
/// let world_pos = chunk_and_local_to_world(ChunkPos::new(-1, 1, 1), 15, 0, 15);
/// assert_eq!(world_pos, IVec3::new(-1, 16, 31));
/// ```
pub fn chunk_and_local_to_world(
    chunk_pos: ChunkPos,
    local_x: usize,
    local_y: usize,
    local_z: usize,
) -> IVec3 {
    chunk_pos.0 * CHUNK_SIZE_I32 + IVec3::new(local_x as i32, local_y as i32, local_z as i32)
}

/// Component storing voxel data for a 16x16x16 chunk.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    blocks: Vec<BlockId>,
}

impl Chunk {
    /// Creates a new chunk filled with the specified default block ID.
    ///
    /// # Arguments
    /// * `default_block` - Initial block ID for every block cell.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::chunk::{Chunk, BlockId};
    ///
    /// let chunk = Chunk::new(BlockId(0));
    /// assert_eq!(chunk.get_block(0, 0, 0), Some(BlockId(0)));
    /// ```
    pub fn new(default_block: BlockId) -> Self {
        Self {
            blocks: vec![default_block; CHUNK_VOLUME],
        }
    }

    /// Retrieves block ID at specified local chunk coordinates.
    /// Returns None if coordinates are out of bounds.
    ///
    /// # Arguments
    /// * `x` - Local X coordinate.
    /// * `y` - Local Y coordinate.
    /// * `z` - Local Z coordinate.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::chunk::{Chunk, BlockId};
    ///
    /// let chunk = Chunk::new(BlockId(2));
    /// assert_eq!(chunk.get_block(1, 2, 3), Some(BlockId(2)));
    /// assert_eq!(chunk.get_block(16, 0, 0), None);
    /// ```
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> Option<BlockId> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE {
            Some(self.blocks[coords_to_index(x, y, z)])
        } else {
            None
        }
    }

    /// Sets block ID at specified local chunk coordinates.
    /// Returns `true` if block was successfully set and differed from the prior value, `false` otherwise.
    ///
    /// # Arguments
    /// * `x` - Local X coordinate.
    /// * `y` - Local Y coordinate.
    /// * `z` - Local Z coordinate.
    /// * `block` - New block ID to store.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::chunk::{Chunk, BlockId};
    ///
    /// let mut chunk = Chunk::new(BlockId(0));
    /// assert!(chunk.set_block(0, 0, 0, BlockId(1)));
    /// assert_eq!(chunk.get_block(0, 0, 0), Some(BlockId(1)));
    /// ```
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockId) -> bool {
        if x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE {
            let index = coords_to_index(x, y, z);
            if self.blocks[index] != block {
                self.blocks[index] = block;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Returns a slice view of the underlying raw flat block storage.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::chunk::{Chunk, BlockId, CHUNK_VOLUME};
    ///
    /// let chunk = Chunk::new(BlockId(0));
    /// assert_eq!(chunk.raw_blocks().len(), CHUNK_VOLUME);
    /// ```
    pub fn raw_blocks(&self) -> &[BlockId] {
        &self.blocks
    }
}

/// Marker component added to a chunk when its block data changes and mesh needs remeshing.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChunkDirty;
