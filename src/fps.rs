use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::culling::VoxelRenderStats;
use crate::world::VoxelWorld;

/// Marker component for the debug text UI element located in the top-right corner.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FpsText;

/// Spawns the FPS and diagnostic statistics text element in the top-right corner of the window.
pub fn setup_fps_text_system(mut commands: Commands) {
    commands.spawn((
        Text::new("FPS: --"),
        TextFont {
            font_size: bevy::text::FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.2, 1.0, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(16.0),
            ..default()
        },
        FpsText,
    ));
}

/// Updates the FPS text UI with current smoothed FPS and voxel chunk render statistics.
pub fn update_fps_text_system(
    diagnostics: Option<Res<DiagnosticsStore>>,
    voxel_world: Option<Res<VoxelWorld>>,
    render_stats: Option<Res<VoxelRenderStats>>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };

    let fps_str = if let Some(diag_store) = diagnostics {
        if let Some(fps_diag) = diag_store.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(fps_val) = fps_diag.smoothed() {
                format!("{:.0}", fps_val)
            } else {
                "--".to_string()
            }
        } else {
            "--".to_string()
        }
    } else {
        "--".to_string()
    };

    let total_chunks = voxel_world.map(|w| w.len()).unwrap_or(0);
    let (vis, culled) = render_stats
        .map(|s| (s.visible_chunks, s.culled_chunks))
        .unwrap_or((0, 0));

    text.0 = format!(
        "FPS: {} | Chunks: {} (Vis: {}, Culled: {})",
        fps_str, total_chunks, vis, culled
    );
}

/// Plugin adding the top-right debug FPS counter and chunk statistics UI.
pub struct FpsOverlayPlugin;

impl Plugin for FpsOverlayPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        app.add_systems(Startup, setup_fps_text_system)
            .add_systems(Update, update_fps_text_system);
    }
}
