use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

use crate::block_edit::get_block_at;
use crate::chunk::Chunk;
use crate::collision::resolve_voxel_collision;
use crate::registry::BlockRegistry;
use crate::settings::CAMERA_COLLIDER_HALF_EXTENTS;
use crate::world::VoxelWorld;

/// First-person creative-mode flight camera controller.
#[derive(Component, Debug, Clone)]
pub struct FlyCamera {
    /// Base movement speed in blocks (meters) per second.
    pub speed: f32,
    /// Fast movement multiplier when holding sprint key (LeftControl).
    pub fast_multiplier: f32,
    /// Mouse look sensitivity in radians per pixel.
    pub sensitivity: f32,
    /// Current yaw rotation around the Y axis in radians.
    pub yaw: f32,
    /// Current pitch rotation around the local X axis in radians.
    pub pitch: f32,
    /// Whether passthrough (noclip) mode is toggled on.
    pub passthrough: bool,
}

impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            speed: 20.0,
            fast_multiplier: 2.5,
            sensitivity: 0.003,
            yaw: 0.0,
            pitch: 0.0,
            passthrough: false,
        }
    }
}

/// System managing cursor grabbing and release for first-person camera control.
pub fn cursor_grab_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut cursor_query: Query<&mut bevy::window::CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(mut cursor) = cursor_query.single_mut() else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Left) {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
    }

    if key_input.just_pressed(KeyCode::Escape) {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}

/// System driving Minecraft creative-mode flight and mouse look.
#[allow(clippy::too_many_arguments)]
pub fn fly_camera_controller_system(
    time: Res<Time>,
    key_input: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    cursor_query: Query<&bevy::window::CursorOptions, With<PrimaryWindow>>,
    game_settings: Option<Res<crate::settings::GameSettings>>,
    voxel_world: Option<Res<VoxelWorld>>,
    chunk_query: Query<&Chunk>,
    registry: Option<Res<BlockRegistry>>,
    mut camera_query: Query<(&mut Transform, &mut FlyCamera)>,
) {
    let Ok(cursor) = cursor_query.single() else {
        return;
    };

    let is_locked = cursor.grab_mode == CursorGrabMode::Locked;
    let (base_speed, fast_mult, sensitivity) = if let Some(ref gs) = game_settings {
        (gs.fly_speed, gs.fly_fast_multiplier, gs.mouse_sensitivity)
    } else {
        (20.0, 2.5, 0.003)
    };

    for (mut transform, mut fly_cam) in camera_query.iter_mut() {
        // Toggle passthrough mode when pressing 'C'
        if key_input.just_pressed(KeyCode::KeyC) {
            fly_cam.passthrough = !fly_cam.passthrough;
        }

        // Mouse look when cursor is grabbed
        if is_locked && mouse_motion.delta != Vec2::ZERO {
            fly_cam.yaw -= mouse_motion.delta.x * sensitivity;
            fly_cam.pitch = (fly_cam.pitch - mouse_motion.delta.y * sensitivity)
                .clamp(-1.55, 1.55); // Clamped to ~89 degrees

            transform.rotation =
                Quat::from_rotation_y(fly_cam.yaw) * Quat::from_rotation_x(fly_cam.pitch);
        }

        // Minecraft creative mode fly movement
        let speed_multiplier = if key_input.pressed(KeyCode::ControlLeft) {
            fast_mult
        } else {
            1.0
        };
        let current_speed = base_speed * speed_multiplier;

        let mut move_direction = Vec3::ZERO;

        // Flatten forward & right for horizontal movement
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();

        if key_input.pressed(KeyCode::KeyW) {
            move_direction += forward;
        }
        if key_input.pressed(KeyCode::KeyS) {
            move_direction -= forward;
        }
        if key_input.pressed(KeyCode::KeyD) {
            move_direction += right;
        }
        if key_input.pressed(KeyCode::KeyA) {
            move_direction -= right;
        }
        if key_input.pressed(KeyCode::Space) {
            move_direction += Vec3::Y;
        }
        if key_input.pressed(KeyCode::ShiftLeft) {
            move_direction -= Vec3::Y;
        }

        if move_direction != Vec3::ZERO {
            let delta = move_direction.normalize() * current_speed * time.delta_secs();

            // When passthrough is toggled on, bypass collision checks to allow passing through blocks.
            if !fly_cam.passthrough
                && let (Some(world), Some(reg)) = (voxel_world.as_deref(), registry.as_deref())
            {
                let is_solid_fn = |world_pos: IVec3| -> bool {
                    if let Some(block_id) = get_block_at(world, &chunk_query, world_pos) {
                        reg.is_solid(block_id)
                    } else {
                        false
                    }
                };
                transform.translation = resolve_voxel_collision(
                    transform.translation,
                    delta,
                    CAMERA_COLLIDER_HALF_EXTENTS,
                    is_solid_fn,
                );
            } else {
                transform.translation += delta;
            }
        }
    }
}

/// Plugin adding first person creative flight controls and cursor grab management.
pub struct FlyCameraPlugin;

impl Plugin for FlyCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (cursor_grab_system, fly_camera_controller_system),
        );
    }
}
