use std::path::{Path, PathBuf};
use bevy::prelude::*;

use crate::chunk::{Chunk, ChunkPos};
use crate::save::active_world::ActiveWorld;
use crate::save::chunk_modified::ChunkModified;
use crate::save::save_error::SaveError;
use crate::save::world_meta::WorldMeta;
use crate::worldgen::TerrainConfig;

/// Validates that a world name is non-empty, contains no path separators or directory traversal symbols.
///
/// # Arguments
/// * `name` - Candidate world identifier string.
///
/// # Example
/// ```rust
/// use block_game_2::save::validate_world_name;
///
/// assert!(validate_world_name("World_1").is_ok());
/// assert!(validate_world_name("").is_err());
/// assert!(validate_world_name("foo/bar").is_err());
/// assert!(validate_world_name("..").is_err());
/// ```
pub fn validate_world_name(name: &str) -> Result<(), SaveError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(SaveError::InvalidWorldName(name.to_string()));
    }
    if trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('\0')
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains("..")
    {
        return Err(SaveError::InvalidWorldName(name.to_string()));
    }
    Ok(())
}

/// Computes the disk file path for a chunk binary file within a world directory.
///
/// # Arguments
/// * `world_dir` - Base path of the world save directory.
/// * `pos` - Chunk grid coordinate.
///
/// # Example
/// ```rust
/// use std::path::Path;
/// use block_game_2::chunk::ChunkPos;
/// use block_game_2::save::chunk_file_path;
///
/// let path = chunk_file_path(Path::new("saves/TestWorld"), ChunkPos::new(1, -2, 3));
/// assert_eq!(path, Path::new("saves/TestWorld/chunks/1_-2_3.bin"));
/// ```
pub fn chunk_file_path(world_dir: &Path, pos: ChunkPos) -> PathBuf {
    world_dir
        .join("chunks")
        .join(format!("{}_{}_{}.bin", pos.0.x, pos.0.y, pos.0.z))
}

#[cfg(not(target_arch = "wasm32"))]
/// Creates a new world directory structure and writes its initial `meta.bin`.
///
/// # Arguments
/// * `saves_root` - Root directory containing all world saves (e.g. `saves/`).
/// * `name` - Name of the new world.
/// * `seed` - Seed for terrain generation.
///
/// # Example
/// ```rust
/// use tempfile::tempdir;
/// use block_game_2::save::create_world;
///
/// let dir = tempdir().unwrap();
/// let meta = create_world(dir.path(), "Alpha", 12345).unwrap();
/// assert_eq!(meta.name, "Alpha");
/// ```
pub fn create_world(saves_root: &Path, name: &str, seed: u32) -> Result<WorldMeta, SaveError> {
    validate_world_name(name)?;
    let world_dir = saves_root.join(name);
    if world_dir.exists() {
        return Err(SaveError::WorldAlreadyExists(name.to_string()));
    }

    let chunks_dir = world_dir.join("chunks");
    std::fs::create_dir_all(&chunks_dir).map_err(|source| SaveError::Io {
        path: chunks_dir.clone(),
        source,
    })?;

    let config = TerrainConfig {
        seed,
        ..Default::default()
    };
    let meta = WorldMeta::new(name.to_string(), config, [0.0, 32.0, 32.0]);
    save_world_meta(&world_dir, &meta)?;

    Ok(meta)
}

#[cfg(not(target_arch = "wasm32"))]
/// Saves world metadata to `meta.bin` inside the specified world directory.
///
/// # Arguments
/// * `world_dir` - Directory path of the world save.
/// * `meta` - World metadata to serialize.
///
/// # Example
/// ```rust
/// use tempfile::tempdir;
/// use block_game_2::save::{create_world, save_world_meta, WorldMeta};
///
/// let dir = tempdir().unwrap();
/// let mut meta = create_world(dir.path(), "Beta", 42).unwrap();
/// meta.player_position = [10.0, 20.0, 30.0];
/// save_world_meta(&dir.path().join("Beta"), &meta).unwrap();
/// ```
pub fn save_world_meta(world_dir: &Path, meta: &WorldMeta) -> Result<(), SaveError> {
    let meta_path = world_dir.join("meta.bin");
    let bytes = bincode::serialize(meta)
        .map_err(|err| SaveError::Serialization(err.to_string()))?;
    std::fs::write(&meta_path, bytes).map_err(|source| SaveError::Io {
        path: meta_path,
        source,
    })?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
/// Loads world metadata from `meta.bin` inside the specified world directory.
///
/// # Arguments
/// * `world_dir` - Directory path of the world save.
///
/// # Example
/// ```rust
/// use tempfile::tempdir;
/// use block_game_2::save::{create_world, load_world_meta};
///
/// let dir = tempdir().unwrap();
/// create_world(dir.path(), "Gamma", 999).unwrap();
/// let meta = load_world_meta(&dir.path().join("Gamma")).unwrap();
/// assert_eq!(meta.name, "Gamma");
/// ```
pub fn load_world_meta(world_dir: &Path) -> Result<WorldMeta, SaveError> {
    let meta_path = world_dir.join("meta.bin");
    if !meta_path.exists() {
        return Err(SaveError::WorldNotFound(
            world_dir.display().to_string(),
        ));
    }
    let bytes = std::fs::read(&meta_path).map_err(|source| SaveError::Io {
        path: meta_path,
        source,
    })?;
    let meta: WorldMeta = bincode::deserialize(&bytes)
        .map_err(|err| SaveError::Serialization(err.to_string()))?;
    Ok(meta)
}

#[cfg(not(target_arch = "wasm32"))]
/// Saves a chunk to its binary file inside the world's chunks directory.
///
/// # Arguments
/// * `world_dir` - Directory path of the world save.
/// * `pos` - Chunk grid coordinate.
/// * `chunk` - Chunk voxel data to persist.
///
/// # Example
/// ```rust
/// use tempfile::tempdir;
/// use block_game_2::chunk::{Chunk, ChunkPos, BlockId};
/// use block_game_2::save::{create_world, save_chunk, load_chunk};
///
/// let dir = tempdir().unwrap();
/// let world_dir = dir.path().join("World");
/// create_world(dir.path(), "World", 1).unwrap();
/// let chunk = Chunk::new(BlockId(1));
/// save_chunk(&world_dir, ChunkPos::new(0, 0, 0), &chunk).unwrap();
/// let loaded = load_chunk(&world_dir, ChunkPos::new(0, 0, 0)).unwrap().unwrap();
/// assert_eq!(loaded.get_block(0, 0, 0), Some(BlockId(1)));
/// ```
pub fn save_chunk(world_dir: &Path, pos: ChunkPos, chunk: &Chunk) -> Result<(), SaveError> {
    let file_path = chunk_file_path(world_dir, pos);
    let bytes = bincode::serialize(chunk)
        .map_err(|err| SaveError::Serialization(err.to_string()))?;
    std::fs::write(&file_path, bytes).map_err(|source| SaveError::Io {
        path: file_path,
        source,
    })?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
/// Loads a chunk from its binary file if present, returning `None` if the chunk has not been persisted.
///
/// # Arguments
/// * `world_dir` - Directory path of the world save.
/// * `pos` - Chunk grid coordinate.
///
/// # Example
/// ```rust
/// use tempfile::tempdir;
/// use block_game_2::chunk::ChunkPos;
/// use block_game_2::save::{create_world, load_chunk};
///
/// let dir = tempdir().unwrap();
/// let world_dir = dir.path().join("World");
/// create_world(dir.path(), "World", 1).unwrap();
/// assert!(load_chunk(&world_dir, ChunkPos::new(99, 0, 99)).unwrap().is_none());
/// ```
pub fn load_chunk(world_dir: &Path, pos: ChunkPos) -> Result<Option<Chunk>, SaveError> {
    let file_path = chunk_file_path(world_dir, pos);
    if !file_path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&file_path).map_err(|source| SaveError::Io {
        path: file_path,
        source,
    })?;
    let chunk: Chunk = bincode::deserialize(&bytes)
        .map_err(|err| SaveError::Serialization(err.to_string()))?;
    Ok(Some(chunk))
}

#[cfg(not(target_arch = "wasm32"))]
/// Enumerates all saved worlds in the given saves directory root.
///
/// # Arguments
/// * `saves_root` - Directory path containing world folders.
///
/// # Example
/// ```rust
/// use tempfile::tempdir;
/// use block_game_2::save::{create_world, list_worlds};
///
/// let dir = tempdir().unwrap();
/// create_world(dir.path(), "WorldA", 1).unwrap();
/// create_world(dir.path(), "WorldB", 2).unwrap();
/// let list = list_worlds(dir.path()).unwrap();
/// assert_eq!(list.len(), 2);
/// ```
pub fn list_worlds(saves_root: &Path) -> Result<Vec<WorldMeta>, SaveError> {
    if !saves_root.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(saves_root).map_err(|source| SaveError::Io {
        path: saves_root.to_path_buf(),
        source,
    })?;

    let mut worlds = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| SaveError::Io {
            path: saves_root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir()
            && path.join("meta.bin").exists()
            && let Ok(meta) = load_world_meta(&path)
        {
            worlds.push(meta);
        }
    }
    worlds.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(worlds)
}

#[cfg(not(target_arch = "wasm32"))]
/// Persists the active world metadata and all loaded modified chunks to disk.
/// Removes the `ChunkModified` marker from successfully persisted chunks.
///
/// # Arguments
/// * `active_world` - Mutable handle to the currently active world resource.
/// * `player_position` - Optional current player world position to record in metadata.
/// * `chunks` - ECS query reading chunk voxel data.
/// * `modified_query` - ECS query targeting chunks marked with `ChunkModified`.
/// * `commands` - Entity commands to remove `ChunkModified` component once written.
///
/// # Example
/// ```ignore
/// save_active_world(&mut active_world, Some([0.0, 32.0, 0.0]), &chunks, &modified_query, &mut commands)?;
/// ```
pub fn save_active_world(
    active_world: &mut ActiveWorld,
    player_position: Option<[f32; 3]>,
    chunks: &Query<&Chunk>,
    modified_query: &Query<(Entity, &ChunkPos), With<ChunkModified>>,
    commands: &mut Commands,
) -> Result<usize, SaveError> {
    if let Some(pos) = player_position {
        active_world.meta.player_position = pos;
    }
    save_world_meta(&active_world.dir, &active_world.meta)?;

    let mut saved_count = 0;
    for (entity, pos) in modified_query.iter() {
        if let Ok(chunk) = chunks.get(entity) {
            save_chunk(&active_world.dir, *pos, chunk)?;
            commands.entity(entity).remove::<ChunkModified>();
            saved_count += 1;
        }
    }

    Ok(saved_count)
}

#[cfg(not(target_arch = "wasm32"))]
/// Deletes a saved world directory and all its chunks.
///
/// # Arguments
/// * `saves_root` - Directory path containing world folders.
/// * `name` - World name to delete.
///
/// # Example
/// ```rust
/// use tempfile::tempdir;
/// use block_game_2::save::{create_world, delete_world, list_worlds};
///
/// let dir = tempdir().unwrap();
/// create_world(dir.path(), "ToDelete", 1).unwrap();
/// delete_world(dir.path(), "ToDelete").unwrap();
/// assert_eq!(list_worlds(dir.path()).unwrap().len(), 0);
/// ```
pub fn delete_world(saves_root: &Path, name: &str) -> Result<(), SaveError> {
    validate_world_name(name)?;
    let world_dir = saves_root.join(name);
    if world_dir.exists() {
        std::fs::remove_dir_all(&world_dir).map_err(|source| SaveError::Io {
            path: world_dir,
            source,
        })?;
    }
    Ok(())
}

// Fallback no-op implementations for wasm32 targets
#[cfg(target_arch = "wasm32")]
pub fn create_world(_saves_root: &Path, name: &str, seed: u32) -> Result<WorldMeta, SaveError> {
    validate_world_name(name)?;
    let mut config = TerrainConfig::default();
    config.seed = seed;
    Ok(WorldMeta::new(name.to_string(), config, [0.0, 32.0, 32.0]))
}

#[cfg(target_arch = "wasm32")]
pub fn save_world_meta(_world_dir: &Path, _meta: &WorldMeta) -> Result<(), SaveError> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn load_world_meta(_world_dir: &Path) -> Result<WorldMeta, SaveError> {
    Err(SaveError::WorldNotFound("wasm32 persistence not supported".into()))
}

#[cfg(target_arch = "wasm32")]
pub fn save_chunk(_world_dir: &Path, _pos: ChunkPos, _chunk: &Chunk) -> Result<(), SaveError> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn load_chunk(_world_dir: &Path, _pos: ChunkPos) -> Result<Option<Chunk>, SaveError> {
    Ok(None)
}

#[cfg(target_arch = "wasm32")]
pub fn list_worlds(_saves_root: &Path) -> Result<Vec<WorldMeta>, SaveError> {
    Ok(Vec::new())
}

#[cfg(target_arch = "wasm32")]
pub fn save_active_world(
    _active_world: &mut ActiveWorld,
    _player_position: Option<[f32; 3]>,
    _chunks: &Query<&Chunk>,
    _modified_query: &Query<(Entity, &ChunkPos), With<ChunkModified>>,
    _commands: &mut Commands,
) -> Result<usize, SaveError> {
    Ok(0)
}

#[cfg(target_arch = "wasm32")]
pub fn delete_world(_saves_root: &Path, _name: &str) -> Result<(), SaveError> {
    Ok(())
}
