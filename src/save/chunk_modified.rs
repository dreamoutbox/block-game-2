use bevy::prelude::*;

/// Component marking that a chunk has been modified by player edits and requires persisting to disk.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChunkModified;
