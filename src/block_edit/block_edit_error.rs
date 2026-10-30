use bevy::prelude::IVec3;
use thiserror::Error;

/// Errors raised while editing blocks in the voxel world.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BlockEditError {
    /// The chunk containing the target block is not loaded.
    #[error("chunk containing block {0} is not loaded")]
    ChunkNotLoaded(IVec3),
}
