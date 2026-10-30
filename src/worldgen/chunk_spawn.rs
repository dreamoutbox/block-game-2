use std::path::{Path, PathBuf};
use bevy::prelude::*;
use bevy::tasks::futures_lite::future::block_on;
use bevy::tasks::AsyncComputeTaskPool;

use crate::chunk::{ChunkDirty, ChunkPos};
use crate::culling::chunk_local_aabb;
use crate::save::load_chunk;
use crate::world::VoxelWorld;
use crate::worldgen::{GenerateChunkTask, TerrainGenerator};

/// Spawns a chunk synchronously into the ECS world if not already present.
/// Loads from `world_dir` if present, falling back to procedural generation.
///
/// # Arguments
/// * `commands` - Entity commands to spawn entities and components.
/// * `world` - VoxelWorld resource tracking loaded chunks.
/// * `generator` - Procedural terrain generator.
/// * `world_dir` - Optional world save directory path to check for saved chunk files.
/// * `pos` - Chunk grid coordinate to spawn.
///
/// # Example
/// ```rust
/// use block_game_2::worldgen::{spawn_chunk_from_storage, TerrainConfig, TerrainGenerator};
/// use block_game_2::world::VoxelWorld;
/// use block_game_2::chunk::ChunkPos;
/// use bevy::prelude::*;
///
/// let mut app = App::new();
/// let mut world = VoxelWorld::default();
/// let generator = TerrainGenerator::new(TerrainConfig::default());
/// let entity = spawn_chunk_from_storage(&mut app.world_mut().commands(), &mut world, &generator, None, ChunkPos::new(0, 0, 0));
/// assert!(world.contains(&ChunkPos::new(0, 0, 0)));
/// ```
pub fn spawn_chunk_from_storage(
    commands: &mut Commands,
    world: &mut VoxelWorld,
    generator: &TerrainGenerator,
    world_dir: Option<&Path>,
    pos: ChunkPos,
) -> Entity {
    if let Some(existing) = world.get(&pos) {
        return existing;
    }

    let chunk = if let Some(dir) = world_dir {
        match load_chunk(dir, pos) {
            Ok(Some(saved)) => saved,
            _ => generator.generate_chunk(pos),
        }
    } else {
        generator.generate_chunk(pos)
    };

    let entity = commands
        .spawn((pos, chunk, ChunkDirty, chunk_local_aabb()))
        .id();
    world.insert(pos, entity);
    entity
}

/// Spawns a procedurally generated chunk synchronously into the ECS world if not already present.
/// Returns the existing or newly created Entity.
///
/// # Arguments
/// * `commands` - Entity commands to spawn entities and components.
/// * `world` - VoxelWorld resource tracking loaded chunks.
/// * `generator` - Procedural terrain generator.
/// * `pos` - Chunk grid coordinate to spawn.
///
/// # Example
/// ```rust
/// use block_game_2::worldgen::{spawn_chunk, TerrainConfig, TerrainGenerator};
/// use block_game_2::world::VoxelWorld;
/// use block_game_2::chunk::ChunkPos;
/// use bevy::prelude::*;
///
/// let mut app = App::new();
/// let mut world = VoxelWorld::default();
/// let generator = TerrainGenerator::new(TerrainConfig::default());
/// let entity = spawn_chunk(&mut app.world_mut().commands(), &mut world, &generator, ChunkPos::new(0, 0, 0));
/// assert!(world.contains(&ChunkPos::new(0, 0, 0)));
/// ```
pub fn spawn_chunk(
    commands: &mut Commands,
    world: &mut VoxelWorld,
    generator: &TerrainGenerator,
    pos: ChunkPos,
) -> Entity {
    spawn_chunk_from_storage(commands, world, generator, None, pos)
}

/// Dispatches asynchronous chunk loading or generation to the compute task pool.
///
/// # Arguments
/// * `commands` - Entity commands to spawn entities and components.
/// * `world` - VoxelWorld resource tracking loaded chunks.
/// * `generator` - Procedural terrain generator.
/// * `world_dir` - Optional world save directory path to check for saved chunk files.
/// * `pos` - Chunk grid coordinate to spawn.
///
/// # Example
/// ```rust
/// use block_game_2::worldgen::{spawn_chunk_async_from_storage, TerrainConfig, TerrainGenerator};
/// use block_game_2::world::VoxelWorld;
/// use block_game_2::chunk::ChunkPos;
/// use bevy::prelude::*;
///
/// let mut app = App::new();
/// bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
/// let mut world = VoxelWorld::default();
/// let generator = TerrainGenerator::new(TerrainConfig::default());
/// let entity = spawn_chunk_async_from_storage(&mut app.world_mut().commands(), &mut world, &generator, None, ChunkPos::new(0, 0, 0));
/// assert!(world.contains(&ChunkPos::new(0, 0, 0)));
/// ```
pub fn spawn_chunk_async_from_storage(
    commands: &mut Commands,
    world: &mut VoxelWorld,
    generator: &TerrainGenerator,
    world_dir: Option<PathBuf>,
    pos: ChunkPos,
) -> Entity {
    if let Some(existing) = world.get(&pos) {
        return existing;
    }

    let pool = AsyncComputeTaskPool::get();
    let generator_clone = generator.clone();
    let task = pool.spawn(async move {
        if let Some(ref dir) = world_dir
            && let Ok(Some(saved)) = load_chunk(dir, pos)
        {
            return saved;
        }
        generator_clone.generate_chunk(pos)
    });

    let entity = commands
        .spawn((
            pos,
            GenerateChunkTask(task),
            chunk_local_aabb(),
        ))
        .id();

    world.insert(pos, entity);
    entity
}

/// Dispatches asynchronous procedural chunk generation to the compute task pool.
///
/// # Arguments
/// * `commands` - Entity commands to spawn entities and components.
/// * `world` - VoxelWorld resource tracking loaded chunks.
/// * `generator` - Procedural terrain generator.
/// * `pos` - Chunk grid coordinate to spawn.
///
/// # Example
/// ```rust
/// use block_game_2::worldgen::{spawn_chunk_async, TerrainConfig, TerrainGenerator};
/// use block_game_2::world::VoxelWorld;
/// use block_game_2::chunk::ChunkPos;
/// use bevy::prelude::*;
///
/// let mut app = App::new();
/// bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
/// let mut world = VoxelWorld::default();
/// let generator = TerrainGenerator::new(TerrainConfig::default());
/// let entity = spawn_chunk_async(&mut app.world_mut().commands(), &mut world, &generator, ChunkPos::new(0, 0, 0));
/// assert!(world.contains(&ChunkPos::new(0, 0, 0)));
/// ```
pub fn spawn_chunk_async(
    commands: &mut Commands,
    world: &mut VoxelWorld,
    generator: &TerrainGenerator,
    pos: ChunkPos,
) -> Entity {
    spawn_chunk_async_from_storage(commands, world, generator, None, pos)
}

/// System polling in-flight background chunk generation tasks and adding chunk voxel data and ChunkDirty.
///
/// # Arguments
/// * `commands` - Commands used to attach completed chunk data and mark it dirty.
/// * `tasks` - Query of entities currently running background generation tasks.
///
/// # Example
/// ```rust
/// use bevy::prelude::*;
/// use block_game_2::worldgen::poll_chunk_generation_system;
///
/// let mut app = App::new();
/// app.add_systems(Update, poll_chunk_generation_system);
/// ```
pub fn poll_chunk_generation_system(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut GenerateChunkTask)>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if task.0.is_finished() {
            let chunk = block_on(&mut task.0);
            commands
                .entity(entity)
                .remove::<GenerateChunkTask>()
                .insert((chunk, ChunkDirty));
        }
    }
}
