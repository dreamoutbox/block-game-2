use std::path::PathBuf;
use tempfile::tempdir;

use crate::chunk::{BlockId, Chunk, ChunkPos};
use crate::save::save_error::SaveError;
use crate::save::world_meta::WorldMeta;
use crate::save::world_save::{
    chunk_file_path, create_world, delete_world, list_worlds, load_chunk, load_world_meta,
    save_chunk, save_world_meta, validate_world_name,
};
use crate::worldgen::{TerrainConfig, TerrainGenerator};

#[test]
fn test_validate_world_name() {
    assert!(validate_world_name("World1").is_ok());
    assert!(validate_world_name("my-cool_world 2").is_ok());

    assert!(matches!(validate_world_name(""), Err(SaveError::InvalidWorldName(_))));
    assert!(matches!(validate_world_name("   "), Err(SaveError::InvalidWorldName(_))));
    assert!(matches!(validate_world_name("foo/bar"), Err(SaveError::InvalidWorldName(_))));
    assert!(matches!(validate_world_name("foo\\bar"), Err(SaveError::InvalidWorldName(_))));
    assert!(matches!(validate_world_name(".."), Err(SaveError::InvalidWorldName(_))));
    assert!(matches!(validate_world_name("foo/../bar"), Err(SaveError::InvalidWorldName(_))));
}

#[test]
fn test_world_meta_save_load_round_trip() {
    let dir = tempdir().unwrap();
    let meta_path = dir.path().to_path_buf();

    let mut config = TerrainConfig::default();
    config.seed = 98765;
    config.base_height = 24.0;

    let original = WorldMeta::new("TestSave".into(), config, [12.0, 34.0, 56.0]);
    save_world_meta(&meta_path, &original).expect("save_world_meta should succeed");

    let loaded = load_world_meta(&meta_path).expect("load_world_meta should succeed");
    assert_eq!(loaded, original);
}

#[test]
fn test_create_world_and_duplicate_error() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let meta = create_world(root, "NewWorld", 42).expect("create_world should succeed");
    assert_eq!(meta.name, "NewWorld");
    assert_eq!(meta.terrain_config.seed, 42);

    let world_dir = root.join("NewWorld");
    assert!(world_dir.join("meta.bin").exists());
    assert!(world_dir.join("chunks").is_dir());

    // Duplicate creation should fail
    let duplicate = create_world(root, "NewWorld", 99);
    assert!(matches!(duplicate, Err(SaveError::WorldAlreadyExists(_))));
}

#[test]
fn test_chunk_file_path() {
    let path = chunk_file_path(&PathBuf::from("saves/MyWorld"), ChunkPos::new(-3, 0, 7));
    assert_eq!(path, PathBuf::from("saves/MyWorld/chunks/-3_0_7.bin"));
}

#[test]
fn test_chunk_save_and_load_round_trip() {
    let dir = tempdir().unwrap();
    let world_dir = dir.path().join("VoxelWorld");
    create_world(dir.path(), "VoxelWorld", 123).unwrap();

    let pos = ChunkPos::new(1, -2, 3);

    // Initial check: chunk should not exist yet
    assert!(load_chunk(&world_dir, pos).unwrap().is_none());

    // Create and save modified chunk
    let mut chunk = Chunk::new(BlockId(0));
    chunk.set_block(4, 5, 6, BlockId(42));
    chunk.set_block(15, 0, 15, BlockId(99));

    save_chunk(&world_dir, pos, &chunk).expect("save_chunk should succeed");

    // Load back and verify contents
    let loaded = load_chunk(&world_dir, pos)
        .expect("load_chunk should succeed")
        .expect("chunk should be present");

    assert_eq!(loaded.get_block(4, 5, 6), Some(BlockId(42)));
    assert_eq!(loaded.get_block(15, 0, 15), Some(BlockId(99)));
    assert_eq!(loaded.get_block(0, 0, 0), Some(BlockId(0)));
}

#[test]
fn test_list_and_delete_worlds() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    create_world(root, "World_B", 200).unwrap();
    create_world(root, "World_A", 100).unwrap();

    let list = list_worlds(root).expect("list_worlds should succeed");
    assert_eq!(list.len(), 2);
    // list_worlds is sorted alphabetically
    assert_eq!(list[0].name, "World_A");
    assert_eq!(list[1].name, "World_B");

    delete_world(root, "World_A").expect("delete_world should succeed");
    let list_after = list_worlds(root).expect("list_worlds should succeed");
    assert_eq!(list_after.len(), 1);
    assert_eq!(list_after[0].name, "World_B");
}

#[test]
fn test_load_edited_chunk_overrides_procedural_generation() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let world_meta = create_world(root, "PersistWorld", 777).unwrap();
    let world_dir = root.join("PersistWorld");

    let generator = TerrainGenerator::new(world_meta.terrain_config);
    let pos = ChunkPos::new(0, 1, 0);

    // Procedural generation gives initial block
    let mut chunk = generator.generate_chunk(pos);
    let original_block = chunk.get_block(0, 0, 0).unwrap();

    // Player edits block to something distinct
    let new_block = BlockId(original_block.0 + 100);
    chunk.set_block(0, 0, 0, new_block);

    // Persist to world save
    save_chunk(&world_dir, pos, &chunk).unwrap();

    // Check loading from disk
    let loaded_from_disk = load_chunk(&world_dir, pos).unwrap().unwrap();
    assert_eq!(loaded_from_disk.get_block(0, 0, 0), Some(new_block));
}
