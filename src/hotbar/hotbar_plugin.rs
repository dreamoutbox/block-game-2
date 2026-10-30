use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;

use crate::block_edit::SelectedBlock;
use crate::hotbar::Hotbar;
use crate::mesh::VoxelMaterialHandle;
use crate::registry::BlockType;

/// Marker component for the hotbar selection box UI node.
#[derive(Component)]
pub struct HotbarSelectionBox;

/// Plugin managing hotbar state, UI rendering, slot switching, and selected block synchronization.
pub struct HotbarPlugin;

impl Plugin for HotbarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hotbar>()
            .init_resource::<AccumulatedMouseScroll>()
            .add_systems(Startup, setup_hotbar_ui_system)
            .add_systems(
                Update,
                (
                    hotbar_input_system,
                    sync_hotbar_selection_system,
                    update_hotbar_ui_system,
                )
                    .chain(),
            );
    }
}

/// Spawns the hotbar GUI at bottom-center of the screen.
fn setup_hotbar_ui_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    voxel_material: Option<Res<VoxelMaterialHandle>>,
) {
    let hotbar_texture = asset_server.load("minecraft/textures/gui/sprites/hud/hotbar.png");
    let selection_texture =
        asset_server.load("minecraft/textures/gui/sprites/hud/hotbar_selection.png");

    let atlas_image = voxel_material.map(|m| m.atlas_image.clone());

    // Root UI container positioned at bottom center
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Percent(50.0),
            // hotbar is 182x22 scaled 2x -> 364x44
            width: Val::Px(364.0),
            height: Val::Px(44.0),
            margin: UiRect {
                left: Val::Px(-182.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Background hotbar sprite
            parent.spawn((
                ImageNode {
                    image: hotbar_texture,
                    ..default()
                },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ));

            // Selection box (24x24 in 1x, so 48x48 in 2x scale)
            parent.spawn((
                ImageNode {
                    image: selection_texture,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(48.0),
                    height: Val::Px(48.0),
                    top: Val::Px(-2.0),
                    left: Val::Px(-2.0),
                    ..default()
                },
                HotbarSelectionBox,
            ));

            // Spawn icon swatches for slot 0 (Grass) and slot 1 (Dirt)
            // Each hotbar slot is 20px wide in 1x (40px in 2x). Slot 0 starts at x=3px (6px in 2x), spaced by 20px (40px).
            let slot_configs = [(0, BlockType::Grass, 4), (1, BlockType::Dirt, 2)];

            for (idx, _block_type, tile_idx) in slot_configs {
                let left_offset = 6.0 + (idx as f32) * 40.0;
                let mut node_cmds = parent.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(left_offset),
                    top: Val::Px(6.0),
                    width: Val::Px(32.0),
                    height: Val::Px(32.0),
                    ..default()
                });

                if let Some(ref atlas) = atlas_image {
                    let tile_x = (tile_idx % 4) as f32 * 64.0;
                    let tile_y = (tile_idx / 4) as f32 * 64.0;
                    node_cmds.insert(ImageNode {
                        image: atlas.clone(),
                        rect: Some(Rect::new(tile_x, tile_y, tile_x + 64.0, tile_y + 64.0)),
                        ..default()
                    });
                }
            }
        });
}

/// System processing number keys (1-9) and mouse wheel to cycle hotbar selection.
pub fn hotbar_input_system(
    key_input: Res<ButtonInput<KeyCode>>,
    mouse_scroll: Option<Res<AccumulatedMouseScroll>>,
    mut hotbar: ResMut<Hotbar>,
) {
    let key_mappings = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (key, index) in key_mappings {
        if key_input.just_pressed(key) && index < hotbar.slots.len() {
            hotbar.selected_index = index;
            break;
        }
    }

    if let Some(scroll) = mouse_scroll {
        if scroll.delta.y < 0.0 {
            hotbar.cycle_next();
        } else if scroll.delta.y > 0.0 {
            hotbar.cycle_prev();
        }
    }
}

/// Keeps the active `SelectedBlock` resource synchronized with `Hotbar`.
pub fn sync_hotbar_selection_system(
    hotbar: Res<Hotbar>,
    mut selected_block: ResMut<SelectedBlock>,
) {
    if hotbar.is_changed()
        && let Some(block) = hotbar.selected_block()
    {
        selected_block.0 = block;
    }
}

/// Updates the position of the hotbar selection box to highlight the chosen slot.
pub fn update_hotbar_ui_system(
    hotbar: Res<Hotbar>,
    mut selection_query: Query<&mut Node, With<HotbarSelectionBox>>,
) {
    if !hotbar.is_changed() {
        return;
    }

    let Ok(mut node) = selection_query.single_mut() else {
        return;
    };

    // Slot offset: base is -2px, each slot advances by 40px
    let left_offset = -2.0 + (hotbar.selected_index as f32) * 40.0;
    node.left = Val::Px(left_offset);
}
