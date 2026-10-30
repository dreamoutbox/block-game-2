use bevy::prelude::*;

use crate::worldgen::{
    poll_chunk_generation_system, TerrainConfig, TerrainGenerator,
};

/// Plugin providing procedural terrain generation resources and background chunk generation polling.
pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<TerrainGenerator>() {
            app.insert_resource(TerrainGenerator::new(TerrainConfig::default()));
        }
        app.add_systems(Update, poll_chunk_generation_system);
    }
}
