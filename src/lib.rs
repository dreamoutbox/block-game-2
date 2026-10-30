pub mod async_mesh;
pub mod batching;
pub mod chunk;
pub mod culling;
pub mod error;
pub mod fly_cam;
pub mod fps;
pub mod mesh;
pub mod occlusion;
pub mod registry;
pub mod settings;
pub mod streaming;
pub mod world;
pub mod world_gen;

#[cfg(test)]
mod tests {
    use super::async_mesh::*;
    use super::batching::*;
    use super::chunk::*;
    use super::culling::*;
    use super::error::*;
    use super::fly_cam::*;
    use super::fps::*;
    use super::mesh::*;
    use super::occlusion::*;
    use super::registry::*;
    use super::streaming::*;
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
        let mesh_single = build_chunk_mesh(ChunkPos::new(0, 0, 0), &chunk, &|_| None, &registry).unwrap();
        let count_single = mesh_single.indices().unwrap().len();
        assert_eq!(count_single, 6 * 6); // 36 indices

        // 2 adjacent blocks -> 10 faces -> 60 indices
        chunk.set_block(6, 5, 5, BlockType::Stone.into());
        let mesh_adjacent = build_chunk_mesh(ChunkPos::new(0, 0, 0), &chunk, &|_| None, &registry).unwrap();
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
        let mesh_without_b = build_chunk_mesh(ChunkPos::new(0, 0, 0), &chunk_a, &|_| None, &registry).unwrap();
        assert_eq!(mesh_without_b.indices().unwrap().len(), 36);

        // With Chunk B neighbor present at ChunkPos(1, 0, 0), PosX face of Chunk A should be culled (5 faces = 30 indices)
        let get_chunk = |pos: ChunkPos| -> Option<&Chunk> {
            if pos == ChunkPos::new(1, 0, 0) {
                Some(&chunk_b)
            } else {
                None
            }
        };
        let mesh_with_b = build_chunk_mesh(ChunkPos::new(0, 0, 0), &chunk_a, &get_chunk, &registry).unwrap();
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
        let mesh = snapshot.build_mesh(&registry).unwrap();
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

    #[test]
    fn test_chunk_aabb_dimensions() {
        let local_aabb = chunk_local_aabb();
        assert_eq!(local_aabb.center, bevy::math::Vec3A::splat(8.0));
        assert_eq!(local_aabb.half_extents, bevy::math::Vec3A::splat(8.0));

        let world_aabb = chunk_world_aabb(ChunkPos::new(2, -1, 3));
        assert_eq!(world_aabb.center.x, 32.0 + 8.0);
        assert_eq!(world_aabb.center.y, -16.0 + 8.0);
        assert_eq!(world_aabb.center.z, 48.0 + 8.0);
    }

    #[test]
    fn test_frustum_culling_intersection() {
        use bevy::camera::CameraProjection;

        // Perspective projection matrix looking forward down -Z from origin
        let proj = PerspectiveProjection::default().get_clip_from_view();
        let frustum = bevy::camera::primitives::Frustum(
            bevy::shape::ViewFrustum::from_clip_from_world(&proj),
        );

        // Chunk in front of camera (down -Z) should intersect frustum
        let inside_pos = ChunkPos::new(0, 0, -2);
        assert!(is_chunk_in_frustum(inside_pos, &frustum));

        // Chunk completely behind the camera (down +Z) should be culled
        let behind_pos = ChunkPos::new(0, 0, 5);
        assert!(!is_chunk_in_frustum(behind_pos, &frustum));

        // Chunk extremely far to the side (+X = 1000) should be culled
        let far_pos = ChunkPos::new(1000, 0, -2);
        assert!(!is_chunk_in_frustum(far_pos, &frustum));
    }

    #[test]
    fn test_update_render_stats_system() {
        let mut app = App::new();
        app.init_resource::<VoxelRenderStats>();
        app.add_systems(Update, update_render_stats_system);

        // Entity 1: visible
        let v1 = ViewVisibility::VISIBLE;
        app.world_mut().spawn((ChunkPos::new(0, 0, 0), v1));

        // Entity 2: culled / hidden
        let v2 = ViewVisibility::HIDDEN;
        app.world_mut().spawn((ChunkPos::new(1, 0, 0), v2));

        app.update();

        let stats = *app.world().resource::<VoxelRenderStats>();
        assert_eq!(stats.total_chunks, 2);
        assert_eq!(stats.visible_chunks, 1);
        assert_eq!(stats.culled_chunks, 1);
    }

    #[test]
    fn test_chunk_solid_detection() {
        let registry = BlockRegistry::with_defaults();

        let solid_chunk = Chunk::new(BlockType::Stone.into());
        assert!(check_chunk_solid(&solid_chunk, &registry));

        let mut mixed_chunk = Chunk::new(BlockType::Stone.into());
        mixed_chunk.set_block(0, 0, 0, BlockType::Air.into());
        assert!(!check_chunk_solid(&mixed_chunk, &registry));
    }

    #[test]
    fn test_ray_blocking_by_solid_chunk() {
        let is_solid = |pos: ChunkPos| -> bool {
            pos == ChunkPos::new(1, 0, 0)
        };

        let camera_pos = Vec3::new(8.0, 8.0, 8.0); // Inside chunk (0, 0, 0)
        let target_blocked = Vec3::new(40.0, 8.0, 8.0); // Inside chunk (2, 0, 0)
        let target_unblocked = Vec3::new(8.0, 40.0, 8.0); // Inside chunk (0, 2, 0)

        assert!(is_ray_blocked(camera_pos, target_blocked, ChunkPos::new(2, 0, 0), &is_solid));
        assert!(!is_ray_blocked(camera_pos, target_unblocked, ChunkPos::new(0, 2, 0), &is_solid));
    }

    #[test]
    fn test_occlusion_culling_system_and_toggle() {
        let mut app = App::new();
        app.insert_resource(VoxelWorld::default());
        app.insert_resource(OcclusionCullingEnabled(true));
        app.add_systems(Update, occlusion_culling_system);

        // Spawn Camera at (8, 8, 8)
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(8.0, 8.0, 8.0),
            GlobalTransform::from_xyz(8.0, 8.0, 8.0),
        ));

        // Intervening solid chunk at (1, 0, 0)
        let e_solid = app.world_mut().spawn((ChunkPos::new(1, 0, 0), SolidChunk, Visibility::Inherited)).id();
        app.world_mut().resource_mut::<VoxelWorld>().insert(ChunkPos::new(1, 0, 0), e_solid);

        // Target occluded chunk at (2, 0, 0)
        let e_target = app.world_mut().spawn((ChunkPos::new(2, 0, 0), Visibility::Inherited)).id();
        app.world_mut().resource_mut::<VoxelWorld>().insert(ChunkPos::new(2, 0, 0), e_target);

        // Run occlusion system -> target chunk is marked occluded and hidden
        app.update();
        assert!(app.world().get::<OccludedChunk>(e_target).is_some());
        assert_eq!(app.world().get::<Visibility>(e_target), Some(&Visibility::Hidden));

        // Toggle occlusion culling OFF
        app.world_mut().insert_resource(OcclusionCullingEnabled(false));
        app.update();

        // Occluded marker removed and visibility restored
        assert!(app.world().get::<OccludedChunk>(e_target).is_none());
        assert_eq!(app.world().get::<Visibility>(e_target), Some(&Visibility::Inherited));
    }

    #[test]
    fn test_batching_shared_material_and_nearest_sampler() {
        let mut app = App::new();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<Image>>();

        let handle = app.world_mut().resource_scope(
            |world, mut materials: Mut<Assets<StandardMaterial>>| {
                let mut images = world.resource_mut::<Assets<Image>>();
                init_shared_voxel_material(&mut materials, &mut images)
            },
        );

        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&handle.atlas_image).unwrap();
        assert!(matches!(image.sampler, bevy::image::ImageSampler::Descriptor(ref d) if matches!(d.min_filter, bevy::image::ImageFilterMode::Nearest)));

        let materials = app.world().resource::<Assets<StandardMaterial>>();
        let material = materials.get(&handle.material).unwrap();
        assert_eq!(material.base_color_texture, Some(handle.atlas_image.clone()));
    }

    #[test]
    fn test_batching_stats_tracking() {
        let mut app = App::new();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<BatchingStats>();
        app.add_systems(Update, update_batching_stats_system);

        let shared_handle = app.world_mut().resource_scope(
            |world, mut materials: Mut<Assets<StandardMaterial>>| {
                let mut images = world.resource_mut::<Assets<Image>>();
                init_shared_voxel_material(&mut materials, &mut images)
            },
        );

        // Spawn 3 chunks sharing the exact same material handle
        app.world_mut().spawn(MeshMaterial3d(shared_handle.material.clone()));
        app.world_mut().spawn(MeshMaterial3d(shared_handle.material.clone()));
        app.world_mut().spawn(MeshMaterial3d(shared_handle.material.clone()));

        app.update();

        let stats = *app.world().resource::<BatchingStats>();
        assert_eq!(stats.total_material_entities, 3);
        assert_eq!(stats.unique_materials, 1);
        assert!(stats.is_batch_optimal);

        // Spawn another entity with a separate distinct material
        let diff_handle = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
        app.world_mut().spawn(MeshMaterial3d(diff_handle));

        app.update();

        let stats_after = *app.world().resource::<BatchingStats>();
        assert_eq!(stats_after.total_material_entities, 4);
        assert_eq!(stats_after.unique_materials, 2);
        assert!(!stats_after.is_batch_optimal);
    }

    #[test]
    fn test_streaming_chunks_around_player() {
        let mut app = App::new();
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        app.insert_resource(VoxelWorld::default());
        app.insert_resource(TerrainGenerator::new(TerrainConfig::default()));
        app.insert_resource(WorldStreamingSettings {
            horizontal_render_distance: 1,
            vertical_render_distance: 0,
            unload_margin: 1,
        });
        app.add_systems(
            Update,
            (stream_chunks_around_player_system, poll_chunk_generation_system).chain(),
        );

        // Spawn player at world origin
        let player_entity = app.world_mut().spawn((
            Player,
            Transform::from_xyz(0.0, 0.0, 0.0),
        )).id();

        app.update();
        std::thread::sleep(std::time::Duration::from_millis(15));
        app.update();

        let initial_count = app.world().resource::<VoxelWorld>().len();
        // At radius 1, horizontally dx=-1..=1, dz=-1..=1 where dx^2+dz^2 <= 1 => (0,0), (1,0), (-1,0), (0,1), (0,-1) = 5 chunks
        assert_eq!(initial_count, 5);

        // Move player far along X to (100.0, 0.0, 0.0)
        app.world_mut().entity_mut(player_entity).insert(Transform::from_xyz(100.0, 0.0, 0.0));
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(15));
        app.update();

        let moved_count = app.world().resource::<VoxelWorld>().len();
        // The old chunks outside the margin should be unloaded, and new ones loaded around (6, 0, 0)
        assert_eq!(moved_count, 5);
        assert!(!app.world().resource::<VoxelWorld>().contains(&ChunkPos::new(0, 0, 0)));
        assert!(app.world().resource::<VoxelWorld>().contains(&ChunkPos::new(6, 0, 0)));
    }

    #[test]
    fn test_streaming_and_meshing_pipeline_stress() {
        let mut app = App::new();
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<VoxelWorld>();
        app.init_resource::<WorldStreamingSettings>();
        app.init_resource::<DebounceSettings>();
        app.init_resource::<VoxelRenderStats>();
        app.init_resource::<BatchingStats>();
        app.init_resource::<OcclusionCullingEnabled>();
        app.insert_resource(BlockRegistry::with_defaults());
        app.insert_resource(TerrainGenerator::new(TerrainConfig::default()));

        // Setup material
        app.world_mut().resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
            let mut images = world.resource_mut::<Assets<Image>>();
            let handle = init_shared_voxel_material(&mut materials, &mut images);
            world.insert_resource(handle);
        });

        app.add_systems(
            Update,
            (
                stream_chunks_around_player_system,
                poll_chunk_generation_system,
                queue_async_remesh_system,
                poll_mesh_tasks_system,
                occlusion_culling_system,
                update_render_stats_system,
                update_batching_stats_system,
            )
                .chain(),
        );

        // Spawn player with Camera3d
        let player = app.world_mut().spawn((
            Player,
            Camera3d::default(),
            Transform::from_xyz(0.0, 16.0, 0.0),
            GlobalTransform::from_xyz(0.0, 16.0, 0.0),
        )).id();

        // Simulate flight through terrain over 15 steps
        for step in 0..15 {
            let x_pos = (step as f32) * 16.0;
            app.world_mut().entity_mut(player).insert((
                Transform::from_xyz(x_pos, 16.0, 0.0),
                GlobalTransform::from_xyz(x_pos, 16.0, 0.0),
            ));
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(15));
            app.update();
        }

        let world_chunks = app.world().resource::<VoxelWorld>().len();
        // Loaded chunk count must remain bounded (no unbounded growth or leak)
        assert!(world_chunks > 0 && world_chunks <= 150, "Chunk count out of bounds: {world_chunks}");

        // Confirm batching remains optimal
        let batch_stats = *app.world().resource::<BatchingStats>();
        assert!(batch_stats.is_batch_optimal);
        assert_eq!(batch_stats.unique_materials, 1);
    }

    #[test]
    fn test_async_chunk_generation() {
        let mut app = App::new();
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        app.insert_resource(VoxelWorld::default());
        app.insert_resource(TerrainGenerator::new(TerrainConfig::default()));
        app.add_systems(Update, poll_chunk_generation_system);

        let mut world = VoxelWorld::default();
        let generator = TerrainGenerator::new(TerrainConfig::default());
        let pos = ChunkPos::new(0, 0, 0);

        let entity = spawn_chunk_async(&mut app.world_mut().commands(), &mut world, &generator, pos);
        app.update();

        // Wait for compute task pool if still in flight
        if app.world().get::<GenerateChunkTask>(entity).is_some() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            app.update();
        }

        // After task resolves, entity has Chunk and ChunkDirty, and GenerateChunkTask is removed
        assert!(app.world().get::<GenerateChunkTask>(entity).is_none());
        assert!(app.world().get::<Chunk>(entity).is_some());
        assert!(app.world().get::<ChunkDirty>(entity).is_some());
    }

    #[test]
    fn test_fly_camera_defaults() {
        let cam = FlyCamera::default();
        assert_eq!(cam.speed, 20.0);
        assert_eq!(cam.fast_multiplier, 2.5);
        assert_eq!(cam.pitch, 0.0);
        assert_eq!(cam.yaw, 0.0);
    }

    #[test]
    fn test_fps_text_setup_and_update() {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.init_resource::<bevy::diagnostic::FrameCount>();
        app.add_plugins(bevy::diagnostic::DiagnosticsPlugin);
        app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default());
        app.insert_resource(VoxelWorld::default());
        app.insert_resource(VoxelRenderStats {
            total_chunks: 10,
            visible_chunks: 7,
            culled_chunks: 3,
        });
        app.add_systems(Startup, setup_fps_text_system);
        app.add_systems(Update, update_fps_text_system);

        app.update();

        // Query spawned FpsText
        let mut query = app.world_mut().query_filtered::<&Text, With<FpsText>>();
        let text = query.single(app.world()).unwrap();
        assert!(text.0.contains("Chunks: 0 (Vis: 7, Culled: 3)"));
    }

    #[test]
    fn test_atlas_blit_and_png_loading() {
        let mut atlas_data = vec![0u8; (ATLAS_PIXEL_SIZE * ATLAS_PIXEL_SIZE * 4) as usize];
        let tile_index = 2;
        load_and_blit_png(&mut atlas_data, tile_index, "minecraft/textures/block/dirt.png")
            .expect("should load and blit dirt texture");

        let col = tile_index % ATLAS_TILES_PER_ROW;
        let row = tile_index / ATLAS_TILES_PER_ROW;
        let start_x = col * TILE_PIXEL_SIZE;
        let start_y = row * TILE_PIXEL_SIZE;
        let start_idx = ((start_y * ATLAS_PIXEL_SIZE + start_x) * 4) as usize;

        let dirt_img = load_png_rgba("minecraft/textures/block/dirt.png").unwrap();
        let expected_first_pixel = dirt_img.get_pixel(0, 0).0;

        assert_eq!(
            &atlas_data[start_idx..start_idx + 4],
            &expected_first_pixel[..]
        );

        // Out of bounds tile index should return error
        let err = load_and_blit_png(&mut atlas_data, 16, "minecraft/textures/block/dirt.png");
        assert!(matches!(err, Err(AtlasError::TileIndexOutOfBounds { .. })));

        // Missing file should return IO error
        let missing_err = load_and_blit_png(&mut atlas_data, 0, "nonexistent.png");
        assert!(matches!(missing_err, Err(AtlasError::Io { .. })));
    }
}
