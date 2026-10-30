use bevy::prelude::*;
use bevy::tasks::Task;

use crate::chunk::Chunk;

/// Component holding an in-flight background procedural chunk generation task.
#[derive(Component)]
pub struct GenerateChunkTask(pub Task<Chunk>);
