pub mod chunk;
pub mod registry;
pub mod world;

#[cfg(test)]
mod tests {
    use super::chunk::*;
    use super::registry::*;
    use super::world::*;
    use bevy::prelude::*;

    #[test]
    fn test_chunk_get_set_and_bounds() {
        let mut chunk = Chunk::new(BlockId(0));
        assert_eq!(chunk.get_block(0, 0, 0), Some(BlockId(0)));

        // Out of bounds checks
        assert_eq!(chunk.get_block(16, 0, 0), None);
        assert_eq!(chunk.get_block(0, 16, 0), None);
        assert_eq!(chunk.get_block(0, 0, 16), None);
        assert!(!chunk.set_block(16, 0, 0, BlockId(1)));

        // Setting a new block
        assert!(chunk.set_block(5, 6, 7, BlockId(1)));
        assert_eq!(chunk.get_block(5, 6, 7), Some(BlockId(1)));

        // Setting to the same block returns false
        assert!(!chunk.set_block(5, 6, 7, BlockId(1)));
    }

    #[test]
    fn test_coord_conversions() {
        // Test index <-> coords roundtrip
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    let idx = coords_to_index(x, y, z);
                    let (rx, ry, rz) = index_to_coords(idx);
                    assert_eq!((x, y, z), (rx, ry, rz));
                }
            }
        }

        // Test world <-> chunk and local conversions including negatives
        let test_positions = [
            IVec3::new(0, 0, 0),
            IVec3::new(15, 15, 15),
            IVec3::new(16, 32, 48),
            IVec3::new(-1, -1, -1),
            IVec3::new(-16, -32, -48),
            IVec3::new(-17, 33, -1),
        ];

        for &world_pos in &test_positions {
            let (chunk_pos, (lx, ly, lz)) = world_to_chunk_and_local(world_pos);
            let roundtrip = chunk_and_local_to_world(chunk_pos, lx, ly, lz);
            assert_eq!(world_pos, roundtrip);
            assert!(lx < CHUNK_SIZE);
            assert!(ly < CHUNK_SIZE);
            assert!(lz < CHUNK_SIZE);
        }

        // Specifically test -1 boundary
        let (neg_chunk, (nlx, nly, nlz)) = world_to_chunk_and_local(IVec3::new(-1, -1, -1));
        assert_eq!(neg_chunk, ChunkPos::new(-1, -1, -1));
        assert_eq!((nlx, nly, nlz), (15, 15, 15));
    }

    #[test]
    fn test_registry_properties() {
        let registry = BlockRegistry::with_defaults();
        assert!(!registry.is_opaque(BlockType::Air.into()));
        assert!(!registry.is_solid(BlockType::Air.into()));

        assert!(registry.is_opaque(BlockType::Stone.into()));
        assert!(registry.is_solid(BlockType::Stone.into()));

        assert!(registry.is_opaque(BlockType::Dirt.into()));
        assert!(registry.is_opaque(BlockType::Grass.into()));

        // Non-opaque but solid leaves
        assert!(!registry.is_opaque(BlockType::Leaves.into()));
        assert!(registry.is_solid(BlockType::Leaves.into()));
    }

    #[test]
    fn test_dirty_flag_lifecycle() {
        let mut app = App::new();
        let entity = app.world_mut().spawn((Chunk::new(BlockId(0)), ChunkPos::new(0, 0, 0))).id();

        // Initially no dirty flag
        assert!(app.world().get::<ChunkDirty>(entity).is_none());

        // Mark dirty
        let mut commands = app.world_mut().commands();
        mark_chunk_dirty(&mut commands, entity);
        app.update();

        assert!(app.world().get::<ChunkDirty>(entity).is_some());

        // Clear dirty
        let mut commands = app.world_mut().commands();
        clear_chunk_dirty(&mut commands, entity);
        app.update();

        assert!(app.world().get::<ChunkDirty>(entity).is_none());
    }

    #[test]
    fn test_voxel_world_resource() {
        let mut app = App::new();
        let mut voxel_world = VoxelWorld::default();

        let e1 = app.world_mut().spawn_empty().id();
        let e2 = app.world_mut().spawn_empty().id();

        voxel_world.insert(ChunkPos::new(0, 0, 0), e1);
        voxel_world.insert(ChunkPos::new(1, 0, 0), e2);

        assert_eq!(voxel_world.len(), 2);
        assert_eq!(voxel_world.get(&ChunkPos::new(0, 0, 0)), Some(e1));
        assert_eq!(voxel_world.get(&ChunkPos::new(1, 0, 0)), Some(e2));
        assert_eq!(voxel_world.get(&ChunkPos::new(2, 0, 0)), None);

        assert_eq!(voxel_world.remove(&ChunkPos::new(0, 0, 0)), Some(e1));
        assert_eq!(voxel_world.len(), 1);
        assert!(!voxel_world.contains(&ChunkPos::new(0, 0, 0)));
    }
}
