use bevy::prelude::*;
use std::collections::HashMap;

use crate::chunk::{ChunkDirty, ChunkPos};

/// Resource mapping chunk positions to their corresponding ECS entity.
#[derive(Resource, Debug, Clone, Default)]
pub struct VoxelWorld {
    chunks: HashMap<ChunkPos, Entity>,
}

impl VoxelWorld {
    /// Inserts a chunk entity into the world map.
    ///
    /// # Arguments
    /// * `pos` - Chunk grid coordinate.
    /// * `entity` - ECS entity owning the chunk.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world::VoxelWorld;
    /// use block_game_2::chunk::ChunkPos;
    /// use bevy::prelude::Entity;
    ///
    /// let mut world = VoxelWorld::default();
    /// world.insert(ChunkPos::new(0, 0, 0), Entity::from_bits(1));
    /// assert!(world.contains(&ChunkPos::new(0, 0, 0)));
    /// ```
    pub fn insert(&mut self, pos: ChunkPos, entity: Entity) -> Option<Entity> {
        self.chunks.insert(pos, entity)
    }

    /// Retrieves chunk entity at specified chunk position.
    ///
    /// # Arguments
    /// * `pos` - Chunk grid coordinate.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world::VoxelWorld;
    /// use block_game_2::chunk::ChunkPos;
    /// use bevy::prelude::Entity;
    ///
    /// let mut world = VoxelWorld::default();
    /// world.insert(ChunkPos::new(0, 0, 0), Entity::from_bits(42));
    /// assert_eq!(world.get(&ChunkPos::new(0, 0, 0)), Some(Entity::from_bits(42)));
    /// ```
    pub fn get(&self, pos: &ChunkPos) -> Option<Entity> {
        self.chunks.get(pos).copied()
    }

    /// Removes chunk entity mapping at specified chunk position.
    ///
    /// # Arguments
    /// * `pos` - Chunk grid coordinate to remove.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world::VoxelWorld;
    /// use block_game_2::chunk::ChunkPos;
    /// use bevy::prelude::Entity;
    ///
    /// let mut world = VoxelWorld::default();
    /// world.insert(ChunkPos::new(0, 0, 0), Entity::from_bits(1));
    /// assert!(world.remove(&ChunkPos::new(0, 0, 0)).is_some());
    /// assert!(!world.contains(&ChunkPos::new(0, 0, 0)));
    /// ```
    pub fn remove(&mut self, pos: &ChunkPos) -> Option<Entity> {
        self.chunks.remove(pos)
    }

    /// Returns true if a chunk is loaded at the given chunk position.
    ///
    /// # Arguments
    /// * `pos` - Chunk grid coordinate to query.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world::VoxelWorld;
    /// use block_game_2::chunk::ChunkPos;
    ///
    /// let world = VoxelWorld::default();
    /// assert!(!world.contains(&ChunkPos::new(0, 0, 0)));
    /// ```
    pub fn contains(&self, pos: &ChunkPos) -> bool {
        self.chunks.contains_key(pos)
    }

    /// Returns the number of loaded chunks.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world::VoxelWorld;
    ///
    /// let world = VoxelWorld::default();
    /// assert_eq!(world.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Returns true if no chunks are currently tracked.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world::VoxelWorld;
    ///
    /// let world = VoxelWorld::default();
    /// assert!(world.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Returns an iterator over all chunk positions and entities.
    pub fn iter(&self) -> impl Iterator<Item = (&ChunkPos, &Entity)> {
        self.chunks.iter()
    }

    /// Removes and returns all tracked chunk positions and their entities.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::world::VoxelWorld;
    /// use block_game_2::chunk::ChunkPos;
    /// use bevy::prelude::Entity;
    ///
    /// let mut world = VoxelWorld::default();
    /// world.insert(ChunkPos::new(0, 0, 0), Entity::from_bits(1));
    /// let drained: Vec<_> = world.drain().collect();
    /// assert_eq!(drained.len(), 1);
    /// assert!(world.is_empty());
    /// ```
    pub fn drain(&mut self) -> impl Iterator<Item = (ChunkPos, Entity)> {
        self.chunks.drain()
    }
}

/// Marks a chunk entity as dirty by adding the `ChunkDirty` component.
///
/// # Arguments
/// * `commands` - Entity commands to mutate ECS components.
/// * `entity` - Target chunk entity.
///
/// # Example
/// ```rust
/// use block_game_2::world::mark_chunk_dirty;
/// use bevy::prelude::*;
///
/// let mut app = App::new();
/// let entity = app.world_mut().spawn_empty().id();
/// mark_chunk_dirty(&mut app.world_mut().commands(), entity);
/// ```
pub fn mark_chunk_dirty(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).insert(ChunkDirty);
}

/// Clears the `ChunkDirty` marker from a chunk entity after its mesh has been updated.
///
/// # Arguments
/// * `commands` - Entity commands to mutate ECS components.
/// * `entity` - Target chunk entity.
///
/// # Example
/// ```rust
/// use block_game_2::world::{mark_chunk_dirty, clear_chunk_dirty};
/// use block_game_2::chunk::ChunkDirty;
/// use bevy::prelude::*;
///
/// let mut app = App::new();
/// let entity = app.world_mut().spawn(ChunkDirty).id();
/// clear_chunk_dirty(&mut app.world_mut().commands(), entity);
/// ```
pub fn clear_chunk_dirty(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).remove::<ChunkDirty>();
}
