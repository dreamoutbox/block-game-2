use bevy::prelude::*;

use crate::async_mesh::{poll_mesh_tasks_system, queue_async_remesh_system, DebounceSettings};
use crate::batching::{setup_voxel_material_system, update_batching_stats_system, BatchingStats};
use crate::chunk::{world_to_chunk_and_local, ChunkPos};
use crate::culling::{update_render_stats_system, VoxelRenderStats};
use crate::occlusion::{occlusion_culling_system, OcclusionCullingEnabled};
use crate::registry::BlockRegistry;
use crate::world::VoxelWorld;
use crate::world_gen::{spawn_chunk, TerrainConfig, TerrainGenerator};

/// Settings controlling streaming radius and unloading hysteresis around the viewer.
#[derive(Resource, Clone, Copy, Debug)]
pub struct WorldStreamingSettings {
    /// Horizontal chunk radius to maintain loaded around the player.
    pub horizontal_render_distance: i32,
    /// Vertical chunk radius (up/down) to maintain loaded around the player.
    pub vertical_render_distance: i32,
    /// Extra buffer chunks before unloading to prevent rapid load/unload cycles at borders.
    pub unload_margin: i32,
}

impl Default for WorldStreamingSettings {
    fn default() -> Self {
        Self {
            horizontal_render_distance: 3,
            vertical_render_distance: 1,
            unload_margin: 1,
        }
    }
}

/// Marker component attached to the player or active camera controlling chunk streaming.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Player;

/// Streams chunks dynamically in and out of the world according to player movement.
pub fn stream_chunks_around_player_system(
    mut commands: Commands,
    mut voxel_world: ResMut<VoxelWorld>,
    terrain_gen: Res<TerrainGenerator>,
    streaming_settings: Res<WorldStreamingSettings>,
    player_query: Query<&Transform, With<Player>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let (player_chunk, _) = world_to_chunk_and_local(player_transform.translation.as_ivec3());
    let h_rad = streaming_settings.horizontal_render_distance;
    let v_rad = streaming_settings.vertical_render_distance;

    // Load newly in-range chunks
    for dy in -v_rad..=v_rad {
        for dz in -h_rad..=h_rad {
            for dx in -h_rad..=h_rad {
                // Spherical / circular horizontal culling
                if dx * dx + dz * dz <= h_rad * h_rad {
                    let target_pos = ChunkPos::new(
                        player_chunk.0.x + dx,
                        player_chunk.0.y + dy,
                        player_chunk.0.z + dz,
                    );
                    spawn_chunk(&mut commands, &mut voxel_world, &terrain_gen, target_pos);
                }
            }
        }
    }

    // Unload chunks exceeding unload threshold
    let max_h = h_rad + streaming_settings.unload_margin;
    let max_v = v_rad + streaming_settings.unload_margin;
    let max_h_sq = max_h * max_h;

    let mut to_unload = Vec::new();

    for (&pos, &entity) in voxel_world.iter() {
        let dx = pos.0.x - player_chunk.0.x;
        let dy = (pos.0.y - player_chunk.0.y).abs();
        let dz = pos.0.z - player_chunk.0.z;

        if dx * dx + dz * dz > max_h_sq || dy > max_v {
            to_unload.push((pos, entity));
        }
    }

    for (pos, entity) in to_unload {
        voxel_world.remove(&pos);
        commands.entity(entity).despawn();
    }
}

/// Core plugin configuring the complete voxel rendering, meshing, streaming, and culling pipeline.
pub struct VoxelWorldPlugin;

impl Plugin for VoxelWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelWorld>()
            .init_resource::<WorldStreamingSettings>()
            .init_resource::<DebounceSettings>()
            .init_resource::<VoxelRenderStats>()
            .init_resource::<BatchingStats>()
            .init_resource::<OcclusionCullingEnabled>()
            .insert_resource(BlockRegistry::with_defaults())
            .insert_resource(TerrainGenerator::new(TerrainConfig::default()))
            .add_systems(Startup, setup_voxel_material_system)
            .add_systems(
                Update,
                (
                    stream_chunks_around_player_system,
                    queue_async_remesh_system,
                    poll_mesh_tasks_system,
                    occlusion_culling_system,
                    update_render_stats_system,
                    update_batching_stats_system,
                )
                    .chain(),
            );
    }
}
