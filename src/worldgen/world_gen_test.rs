use bevy::prelude::*;

use crate::chunk::{chunk_and_local_to_world, Chunk, ChunkDirty, ChunkPos, CHUNK_SIZE};
use crate::world::VoxelWorld;
use crate::worldgen::{
    spawn_chunk, TerrainConfig, TerrainGenerator,
};

#[test]
fn test_terrain_generator_determinism() {
    let config = TerrainConfig {
        seed: 12345,
        base_height: 16.0,
        scale: 0.05,
        amplitude: 10.0,
    };
    let generator1 = TerrainGenerator::new(config.clone());
    let generator2 = TerrainGenerator::new(config);

    let pos = ChunkPos::new(2, 0, -3);
    let chunk1 = generator1.generate_chunk(pos);
    let chunk2 = generator2.generate_chunk(pos);

    assert_eq!(chunk1.raw_blocks(), chunk2.raw_blocks());
}

#[test]
fn test_terrain_surface_continuity_across_boundaries() {
    let generator = TerrainGenerator::new(TerrainConfig::default());

    // Check continuity across X boundary between chunk (0, 1, 0) and (1, 1, 0)
    let chunk_left = generator.generate_chunk(ChunkPos::new(0, 1, 0));
    let chunk_right = generator.generate_chunk(ChunkPos::new(1, 1, 0));

    for z in 0..CHUNK_SIZE {
        let left_world = chunk_and_local_to_world(ChunkPos::new(0, 1, 0), 15, 0, z);
        let right_world = chunk_and_local_to_world(ChunkPos::new(1, 1, 0), 0, 0, z);
        assert_eq!(left_world.x + 1, right_world.x);

        let h_left = generator.height_at(left_world.x, left_world.z);
        let h_right = generator.height_at(right_world.x, right_world.z);

        // Perlin noise with continuous sampling guarantees adjacent integer columns differ by at most a small amount
        assert!(
            (h_left - h_right).abs() <= 3,
            "Height jump too large across boundary: left={h_left}, right={h_right}"
        );

        // Verify the generated blocks match the height function across boundaries
        for y in 0..CHUNK_SIZE {
            let left_by = chunk_and_local_to_world(ChunkPos::new(0, 1, 0), 15, y, z).y;
            let right_by = chunk_and_local_to_world(ChunkPos::new(1, 1, 0), 0, y, z).y;

            let expected_left = TerrainGenerator::block_for_height(left_by, h_left);
            let expected_right = TerrainGenerator::block_for_height(right_by, h_right);

            assert_eq!(chunk_left.get_block(15, y, z), Some(expected_left));
            assert_eq!(chunk_right.get_block(0, y, z), Some(expected_right));
        }
    }
}

#[test]
fn test_spawn_chunk_integration() {
    let mut app = App::new();
    let mut world = VoxelWorld::default();
    let generator = TerrainGenerator::new(TerrainConfig::default());
    let pos = ChunkPos::new(0, 0, 0);

    let entity1 = spawn_chunk(&mut app.world_mut().commands(), &mut world, &generator, pos);
    app.update();

    assert!(world.contains(&pos));
    assert!(app.world().get::<Chunk>(entity1).is_some());
    assert!(app.world().get::<ChunkDirty>(entity1).is_some());

    // Calling spawn_chunk again for same pos returns the existing entity without duplicating
    let entity2 = spawn_chunk(&mut app.world_mut().commands(), &mut world, &generator, pos);
    assert_eq!(entity1, entity2);
    assert_eq!(world.len(), 1);
}
