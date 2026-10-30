use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

use crate::block_edit::{SelectedBlock, get_block_at, set_block_at, voxel_raycast};
use crate::chunk::{BlockId, Chunk};
use crate::collision::aabb_overlaps_block;
use crate::fly_cam::FlyCamera;
use crate::game_state::{in_game_or_no_state, InGameEntity};
use crate::registry::{BlockRegistry, BlockType};
use crate::settings::{BLOCK_REACH_DISTANCE, CAMERA_COLLIDER_HALF_EXTENTS};
use crate::world::VoxelWorld;

/// Marker component for the crosshair UI element.
#[derive(Component)]
pub struct CrosshairUi;

/// Plugin adding block breaking (left-click) and placing (right-click) via raycasting.
pub struct BlockEditPlugin;

impl Plugin for BlockEditPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedBlock>()
            .add_systems(Startup, setup_crosshair_system)
            .add_systems(Update, block_edit_input_system.run_if(in_game_or_no_state));
    }
}

/// Spawns a small centered crosshair on screen.
fn setup_crosshair_system(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(4.0),
            height: Val::Px(4.0),
            margin: UiRect {
                left: Val::Px(-2.0),
                top: Val::Px(-2.0),
                ..default()
            },
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        CrosshairUi,
        InGameEntity,
    ));
}

/// System handling left-click to break blocks and right-click to place blocks.
#[allow(clippy::too_many_arguments)]
pub fn block_edit_input_system(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    cursor_query: Query<&bevy::window::CursorOptions, With<PrimaryWindow>>,
    camera_query: Query<&Transform, With<FlyCamera>>,
    voxel_world: Res<VoxelWorld>,
    mut chunk_query: Query<&mut Chunk>,
    registry: Res<BlockRegistry>,
    selected_block: Res<SelectedBlock>,
) {
    let Ok(cursor) = cursor_query.single() else {
        return;
    };

    // Only process edits when the cursor is grabbed / locked to prevent clicking into the window from breaking blocks.
    if cursor.grab_mode != CursorGrabMode::Locked {
        return;
    }

    let break_pressed = mouse_button.just_pressed(MouseButton::Left);
    let place_pressed = mouse_button.just_pressed(MouseButton::Right);

    if !break_pressed && !place_pressed {
        return;
    }

    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let origin = camera_transform.translation;
    let forward = camera_transform.forward().as_vec3();

    let is_solid_fn = |world_pos: IVec3| -> bool {
        if let Some(block_id) = get_block_at(&voxel_world, &chunk_query.as_readonly(), world_pos) {
            registry.is_solid(block_id)
        } else {
            false
        }
    };

    let Some(hit) = voxel_raycast(origin, forward, BLOCK_REACH_DISTANCE, is_solid_fn) else {
        return;
    };

    if break_pressed {
        let _ = set_block_at(
            &mut commands,
            &voxel_world,
            &mut chunk_query,
            hit.block,
            BlockType::Air.into(),
        );
    } else if place_pressed {
        let place_pos = hit.block + hit.normal;

        // Prevent placing inside the player's camera collider AABB
        if aabb_overlaps_block(origin, CAMERA_COLLIDER_HALF_EXTENTS, place_pos) {
            return;
        }

        // Only place if the target voxel is currently empty/non-solid
        if let Some(existing) = get_block_at(&voxel_world, &chunk_query.as_readonly(), place_pos)
            && registry.is_solid(existing)
        {
            return;
        }

        let _ = set_block_at(
            &mut commands,
            &voxel_world,
            &mut chunk_query,
            place_pos,
            BlockId::from(selected_block.0),
        );
    }
}
