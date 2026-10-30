use std::path::PathBuf;
use bevy::prelude::*;

use crate::save::WorldMeta;

/// Resource tracking the currently active loaded world.
#[derive(Resource, Clone, Debug)]
pub struct ActiveWorld {
    /// World name identifier.
    pub name: String,
    /// Root directory path of the active world save on disk.
    pub dir: PathBuf,
    /// World metadata including generator configuration and player position.
    pub meta: WorldMeta,
}

impl ActiveWorld {
    /// Constructs a new active world handle.
    ///
    /// # Arguments
    /// * `name` - World name identifier.
    /// * `dir` - Directory path containing world saves and chunks.
    /// * `meta` - World metadata.
    ///
    /// # Example
    /// ```rust
    /// use std::path::PathBuf;
    /// use block_game_2::save::{ActiveWorld, WorldMeta};
    /// use block_game_2::worldgen::TerrainConfig;
    ///
    /// let meta = WorldMeta::new("DevWorld".into(), TerrainConfig::default(), [0.0, 32.0, 32.0]);
    /// let active = ActiveWorld::new("DevWorld".into(), PathBuf::from("saves/DevWorld"), meta);
    /// assert_eq!(active.name, "DevWorld");
    /// ```
    pub fn new(name: String, dir: PathBuf, meta: WorldMeta) -> Self {
        Self { name, dir, meta }
    }
}
