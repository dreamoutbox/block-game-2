use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

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
}

impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            speed: 20.0,
            fast_multiplier: 2.5,
            sensitivity: 0.003,
            yaw: 0.0,
            pitch: 0.0,
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
pub fn fly_camera_controller_system(
    time: Res<Time>,
    key_input: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    cursor_query: Query<&bevy::window::CursorOptions, With<PrimaryWindow>>,
    mut camera_query: Query<(&mut Transform, &mut FlyCamera)>,
) {
    let Ok(cursor) = cursor_query.single() else {
        return;
    };

    let is_locked = cursor.grab_mode == CursorGrabMode::Locked;

    for (mut transform, mut fly_cam) in camera_query.iter_mut() {
        // Mouse look when cursor is grabbed
        if is_locked && mouse_motion.delta != Vec2::ZERO {
            fly_cam.yaw -= mouse_motion.delta.x * fly_cam.sensitivity;
            fly_cam.pitch = (fly_cam.pitch - mouse_motion.delta.y * fly_cam.sensitivity)
                .clamp(-1.55, 1.55); // Clamped to ~89 degrees

            transform.rotation =
                Quat::from_rotation_y(fly_cam.yaw) * Quat::from_rotation_x(fly_cam.pitch);
        }

        // Minecraft creative mode fly movement
        let speed_multiplier = if key_input.pressed(KeyCode::ControlLeft) {
            fly_cam.fast_multiplier
        } else {
            1.0
        };
        let current_speed = fly_cam.speed * speed_multiplier;

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
            transform.translation +=
                move_direction.normalize() * current_speed * time.delta_secs();
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
