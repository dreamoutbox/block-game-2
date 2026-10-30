use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use crate::asset_texture_loader::{
    setup_asset_texture_loader_system, update_atlas_from_asset_server_system,
};
use crate::async_mesh::{DebounceSettings, poll_mesh_tasks_system, queue_async_remesh_system};
use crate::batching::{BatchingStats, setup_voxel_material_system, update_batching_stats_system};
use crate::block_edit::BlockEditPlugin;
use crate::chunk::{Chunk, ChunkPos, world_to_chunk_and_local};
use crate::culling::{VoxelRenderStats, update_render_stats_system};
use crate::fly_cam::FlyCameraPlugin;
use crate::fps::FpsOverlayPlugin;
use crate::game_state::{GameStatePlugin, in_game_or_no_state};
use crate::hotbar::HotbarPlugin;
use crate::occlusion::{OcclusionCullingEnabled, OcclusionCullingTimer, occlusion_culling_system};
use crate::registry::BlockRegistry;
use crate::save::{ActiveWorld, ChunkModified, save_chunk};
use crate::settings::GameSettings;
use crate::world::VoxelWorld;
use crate::worldgen::{spawn_chunk_async_from_storage, TerrainGenerator, WorldGenPlugin};

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
#[allow(clippy::too_many_arguments)]
pub fn stream_chunks_around_player_system(
    mut commands: Commands,
    mut voxel_world: ResMut<VoxelWorld>,
    terrain_gen: Res<TerrainGenerator>,
    game_settings: Option<Res<GameSettings>>,
    streaming_settings: Option<Res<WorldStreamingSettings>>,
    player_query: Query<&Transform, With<Player>>,
    active_world: Option<Res<ActiveWorld>>,
    chunks_query: Query<&Chunk>,
    modified_query: Query<(), With<ChunkModified>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let (player_chunk, _) = world_to_chunk_and_local(player_transform.translation.as_ivec3());
    let (h_rad, v_rad, unload_margin) = if let Some(ref gs) = game_settings {
        (
            gs.horizontal_render_distance,
            gs.vertical_render_distance,
            gs.unload_margin,
        )
    } else if let Some(ref ss) = streaming_settings {
        (
            ss.horizontal_render_distance,
            ss.vertical_render_distance,
            ss.unload_margin,
        )
    } else {
        let def = GameSettings::default();
        (
            def.horizontal_render_distance,
            def.vertical_render_distance,
            def.unload_margin,
        )
    };

    // World directory handle passed to chunk spawning
    let world_dir = active_world.as_ref().map(|w| w.dir.clone());

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
                    spawn_chunk_async_from_storage(
                        &mut commands,
                        &mut voxel_world,
                        &terrain_gen,
                        world_dir.clone(),
                        target_pos,
                    );
                }
            }
        }
    }

    // Unload chunks exceeding unload threshold
    let max_h = h_rad + unload_margin;
    let max_v = v_rad + unload_margin;
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
        if let Some(ref active) = active_world
            && modified_query.contains(entity)
            && let Ok(chunk) = chunks_query.get(entity)
        {
            let _ = save_chunk(&active.dir, pos, chunk); // best-effort save on chunk unload
        }
        voxel_world.remove(&pos);
        commands.entity(entity).despawn();
    }
}

/// Core plugin configuring the complete voxel rendering, meshing, streaming, and culling pipeline.
pub struct VoxelWorldPlugin;

impl Plugin for VoxelWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelWorld>()
            .init_resource::<GameSettings>()
            .init_resource::<WorldStreamingSettings>()
            .init_resource::<DebounceSettings>()
            .init_resource::<VoxelRenderStats>()
            .init_resource::<BatchingStats>()
            .init_resource::<OcclusionCullingEnabled>()
            .init_resource::<OcclusionCullingTimer>()
            .insert_resource(BlockRegistry::with_defaults())
            .add_plugins(EguiPlugin::default())
            .add_plugins(GameStatePlugin)
            .add_plugins(WorldGenPlugin)
            .add_plugins(FpsOverlayPlugin)
            .add_plugins(FlyCameraPlugin)
            .add_plugins(BlockEditPlugin)
            .add_plugins(HotbarPlugin)
            .add_systems(
                Startup,
                (
                    setup_voxel_material_system,
                    setup_asset_texture_loader_system,
                ),
            )
            .add_systems(
                Update,
                (
                    stream_chunks_around_player_system.run_if(in_game_or_no_state),
                    queue_async_remesh_system.run_if(in_game_or_no_state),
                    poll_mesh_tasks_system,
                    occlusion_culling_system.run_if(in_game_or_no_state),
                    update_atlas_from_asset_server_system,
                    update_render_stats_system,
                    update_batching_stats_system,
                )
                    .chain(),
            );
    }
}
