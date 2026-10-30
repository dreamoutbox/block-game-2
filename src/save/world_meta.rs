use serde::{Deserialize, Serialize};

use crate::worldgen::TerrainConfig;

/// Metadata stored for a saved world, including generator configuration and player position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldMeta {
    /// Human-readable world name.
    pub name: String,
    /// Procedural terrain generation configuration.
    pub terrain_config: TerrainConfig,
    /// Last recorded player world position `[x, y, z]`.
    pub player_position: [f32; 3],
}

impl WorldMeta {
    /// Constructs a new world metadata record.
    ///
    /// # Arguments
    /// * `name` - Display name of the world.
    /// * `terrain_config` - Terrain generator settings.
    /// * `player_position` - Initial or saved player position coordinates.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::save::WorldMeta;
    /// use block_game_2::worldgen::TerrainConfig;
    ///
    /// let meta = WorldMeta::new("MyWorld".into(), TerrainConfig::default(), [0.0, 32.0, 32.0]);
    /// assert_eq!(meta.name, "MyWorld");
    /// ```
    pub fn new(name: String, terrain_config: TerrainConfig, player_position: [f32; 3]) -> Self {
        Self {
            name,
            terrain_config,
            player_position,
        }
    }
}
