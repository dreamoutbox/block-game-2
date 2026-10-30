use bevy::prelude::*;

/// Minecraft overworld daytime sky color (#78A7FF: rgb(120, 167, 255)).
pub const MINECRAFT_SKY_COLOR: Color = Color::srgb(120.0 / 255.0, 167.0 / 255.0, 1.0);

/// Directional sunlight illuminance in lux.
/// Kept moderate (approx. 9,000 - 10,000 lux) so voxels are clearly visible without overexposure/glare.
pub const SUN_LIGHT_ILLUMINANCE: f32 = 9_000.0;

/// Directional sunlight transform position and target looking position.
pub const SUN_LIGHT_POSITION: Vec3 = Vec3::new(50.0, 80.0, 50.0);
pub const SUN_LIGHT_TARGET: Vec3 = Vec3::ZERO;

/// Central user-adjustable configuration for voxel world streaming, controls, and performance.
#[derive(Resource, Clone, Copy, Debug)]
pub struct GameSettings {
    /// Horizontal render distance in chunks around the player.
    /// Lower values (e.g. 2-3) improve FPS on low-end hardware; higher values (e.g. 5-8) extend view range.
    pub horizontal_render_distance: i32,
    /// Vertical render distance in chunks above and below the player.
    pub vertical_render_distance: i32,
    /// Chunk margin beyond render distance before unloading (hysteresis buffer to prevent border thrashing).
    pub unload_margin: i32,
    /// Movement speed for the first-person flying camera in blocks per second.
    pub fly_speed: f32,
    /// Speed multiplier when holding sprint key (LeftControl).
    pub fly_fast_multiplier: f32,
    /// Sensitivity factor for mouse look rotation in radians per pixel.
    pub mouse_sensitivity: f32,
    /// Whether directional sunlight casts real-time shadow maps.
    pub shadows_enabled: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            // Adjust this value to increase or decrease render distance
            horizontal_render_distance: 8,
            vertical_render_distance: 6,
            unload_margin: 1,
            fly_speed: 30.0,
            fly_fast_multiplier: 2.5,
            mouse_sensitivity: 0.003,
            shadows_enabled: true,
        }
    }
}

impl GameSettings {
    /// Creates a custom settings configuration.
    ///
    /// # Arguments
    /// * `horizontal_render_distance` - Horizontal chunk radius.
    /// * `vertical_render_distance` - Vertical chunk radius.
    /// * `unload_margin` - Extra chunk distance before unloading.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::settings::GameSettings;
    ///
    /// let settings = GameSettings::new(3, 1, 1);
    /// assert_eq!(settings.horizontal_render_distance, 3);
    /// ```
    pub fn new(
        horizontal_render_distance: i32,
        vertical_render_distance: i32,
        unload_margin: i32,
    ) -> Self {
        Self {
            horizontal_render_distance,
            vertical_render_distance,
            unload_margin,
            ..default()
        }
    }
}

/// Maximum distance in blocks at which the player can break or place blocks.
pub const BLOCK_REACH_DISTANCE: f32 = 5.0;
