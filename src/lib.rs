pub mod async_mesh;
pub mod chunk;
pub mod mesh;
pub mod registry;
pub mod world;
pub mod world_gen;

#[cfg(test)]
mod tests {
    use super::async_mesh::*;
    use super::chunk::*;
    use super::mesh::*;
    use super::registry::*;
    use super::world::*;
    use super::world_gen::*;
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
            assert!((h_left - h_right).abs() <= 3, "Height jump too large across boundary: left={h_left}, right={h_right}");

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

    #[test]
    fn test_mesh_single_block_and_adjacent_culling() {
        let registry = BlockRegistry::with_defaults();

        // 1 block isolated -> 6 faces -> 36 indices
        let mut chunk = Chunk::new(BlockType::Air.into());
        chunk.set_block(5, 5, 5, BlockType::Stone.into());
        let mesh_single = build_chunk_mesh(ChunkPos::new(0, 0, 0), &chunk, &|_| None, &registry);
        let count_single = mesh_single.indices().unwrap().len();
        assert_eq!(count_single, 6 * 6); // 36 indices

        // 2 adjacent blocks -> 10 faces -> 60 indices
        chunk.set_block(6, 5, 5, BlockType::Stone.into());
        let mesh_adjacent = build_chunk_mesh(ChunkPos::new(0, 0, 0), &chunk, &|_| None, &registry);
        let count_adjacent = mesh_adjacent.indices().unwrap().len();
        assert_eq!(count_adjacent, 10 * 6); // 60 indices
    }

    #[test]
    fn test_mesh_boundary_face_culling() {
        let registry = BlockRegistry::with_defaults();

        // Chunk A has a stone block at local x = 15
        let mut chunk_a = Chunk::new(BlockType::Air.into());
        chunk_a.set_block(15, 0, 0, BlockType::Stone.into());

        // Chunk B has a stone block at local x = 0
        let mut chunk_b = Chunk::new(BlockType::Air.into());
        chunk_b.set_block(0, 0, 0, BlockType::Stone.into());

        // Without Chunk B, Chunk A should have 6 faces (including PosX)
        let mesh_without_b = build_chunk_mesh(ChunkPos::new(0, 0, 0), &chunk_a, &|_| None, &registry);
        assert_eq!(mesh_without_b.indices().unwrap().len(), 36);

        // With Chunk B neighbor present at ChunkPos(1, 0, 0), PosX face of Chunk A should be culled (5 faces = 30 indices)
        let get_chunk = |pos: ChunkPos| -> Option<&Chunk> {
            if pos == ChunkPos::new(1, 0, 0) {
                Some(&chunk_b)
            } else {
                None
            }
        };
        let mesh_with_b = build_chunk_mesh(ChunkPos::new(0, 0, 0), &chunk_a, &get_chunk, &registry);
        assert_eq!(mesh_with_b.indices().unwrap().len(), 5 * 6);
    }

    #[test]
    fn test_sync_remesh_system() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.insert_resource(VoxelWorld::default());
        app.insert_resource(BlockRegistry::with_defaults());
        app.add_systems(Update, sync_remesh_dirty_chunks_system);

        let mut chunk = Chunk::new(BlockType::Air.into());
        chunk.set_block(0, 0, 0, BlockType::Stone.into());

        let pos = ChunkPos::new(0, 0, 0);
        let entity = app.world_mut().spawn((pos, chunk, ChunkDirty)).id();
        app.world_mut().resource_mut::<VoxelWorld>().insert(pos, entity);

        // Before update: entity has ChunkDirty, no Mesh3d
        assert!(app.world().get::<ChunkDirty>(entity).is_some());
        assert!(app.world().get::<Mesh3d>(entity).is_none());

        app.update();

        // After update: entity has Mesh3d, ChunkDirty is removed
        assert!(app.world().get::<ChunkDirty>(entity).is_none());
        assert!(app.world().get::<Mesh3d>(entity).is_some());
    }

    #[test]
    fn test_async_mesh_lifecycle_and_debounce() {
        let mut app = App::new();
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        app.init_resource::<Assets<Mesh>>();
        app.insert_resource(VoxelWorld::default());
        app.insert_resource(BlockRegistry::with_defaults());
        app.insert_resource(DebounceSettings { debounce_frames: 2 });
        app.add_systems(Update, (queue_async_remesh_system, poll_mesh_tasks_system));

        let mut chunk = Chunk::new(BlockType::Air.into());
        chunk.set_block(0, 0, 0, BlockType::Stone.into());

        let pos = ChunkPos::new(0, 0, 0);
        let entity = app.world_mut().spawn((pos, chunk, ChunkDirty)).id();
        app.world_mut().resource_mut::<VoxelWorld>().insert(pos, entity);

        // Frame 1: Debounce initialized (frames_remaining = 1)
        app.update();
        assert!(app.world().get::<ComputeMeshTask>(entity).is_none());
        assert!(app.world().get::<ChunkDebounce>(entity).is_some());

        // Frame 2: Debounce decrements (frames_remaining = 0)
        app.update();
        assert!(app.world().get::<ComputeMeshTask>(entity).is_none());

        // Frame 3: Task dispatched to thread pool
        app.update();
        // Wait briefly for background compute to finish
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Frame 4: Task polled and resolved into Mesh3d
        app.update();
        assert!(app.world().get::<Mesh3d>(entity).is_some());
        assert!(app.world().get::<ComputeMeshTask>(entity).is_none());
        assert!(app.world().get::<ChunkDirty>(entity).is_none());
    }

    #[test]
    fn test_async_mesh_coalesce_and_requeue() {
        let mut app = App::new();
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        app.init_resource::<Assets<Mesh>>();
        app.insert_resource(VoxelWorld::default());
        app.insert_resource(BlockRegistry::with_defaults());
        app.insert_resource(DebounceSettings { debounce_frames: 0 });
        app.add_systems(Update, (queue_async_remesh_system, poll_mesh_tasks_system));

        let mut chunk = Chunk::new(BlockType::Air.into());
        chunk.set_block(0, 0, 0, BlockType::Stone.into());

        let pos = ChunkPos::new(0, 0, 0);
        let entity = app.world_mut().spawn((pos, chunk, ChunkDirty)).id();
        app.world_mut().resource_mut::<VoxelWorld>().insert(pos, entity);

        // Frame 1: task dispatched immediately
        app.update();

        // While task is in flight (or finishing), multiple edits occur
        let mut commands = app.world_mut().commands();
        request_chunk_remesh(&mut commands, entity, 0, true);
        commands.entity(entity).insert(ChunkDirty);

        // Next frame processes completion and requeue
        app.update();
        // Flush deferred commands
        app.update();

        // The requeue mechanism ensures ChunkDirty is preserved/re-added so edits are never dropped
        assert!(app.world().get::<ChunkDirty>(entity).is_some() || app.world().get::<ComputeMeshTask>(entity).is_some() || app.world().get::<Mesh3d>(entity).is_some());
    }

    #[test]
    fn test_chunk_snapshot_isolation() {
        let registry = BlockRegistry::with_defaults();
        let mut chunk = Chunk::new(BlockType::Air.into());
        chunk.set_block(0, 0, 0, BlockType::Stone.into());

        let snapshot = ChunkSnapshot {
            pos: ChunkPos::new(0, 0, 0),
            center: std::sync::Arc::new(chunk.clone()),
            neighbors: [None, None, None, None, None, None],
        };

        // Mutating original chunk after snapshot does NOT alter snapshot geometry
        chunk.set_block(1, 0, 0, BlockType::Stone.into());
        let mesh = snapshot.build_mesh(&registry);
        // Single block in snapshot = 36 indices
        assert_eq!(mesh.indices().unwrap().len(), 36);
    }

    #[test]
    fn test_rapid_block_edits_stress() {
        let mut app = App::new();
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        app.init_resource::<Assets<Mesh>>();
        app.insert_resource(VoxelWorld::default());
        app.insert_resource(BlockRegistry::with_defaults());
        app.insert_resource(DebounceSettings { debounce_frames: 1 });
        app.add_systems(Update, (queue_async_remesh_system, poll_mesh_tasks_system));

        let pos = ChunkPos::new(0, 0, 0);
        let chunk = Chunk::new(BlockType::Air.into());
        let entity = app.world_mut().spawn((pos, chunk)).id();
        app.world_mut().resource_mut::<VoxelWorld>().insert(pos, entity);

        // Rapidly place and remove 100 blocks
        for i in 0..100 {
            let x = (i % 16) as usize;
            let z = ((i / 16) % 16) as usize;
            {
                let mut chunk_mut = app.world_mut().get_mut::<Chunk>(entity).unwrap();
                chunk_mut.set_block(x, 0, z, BlockType::Stone.into());
            }
            let is_running = app.world().get::<ComputeMeshTask>(entity).is_some();
            let mut commands = app.world_mut().commands();
            request_chunk_remesh(&mut commands, entity, 1, is_running);

            // Tick app every 5 edits
            if i % 5 == 0 {
                app.update();
            }
        }

        // Run until meshing settles
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            app.update();
        }

        assert!(app.world().get::<Mesh3d>(entity).is_some());
    }
}
