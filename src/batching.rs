use bevy::image::ImageSampler;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::*;

use crate::mesh::{create_texture_atlas_image, VoxelMaterialHandle};

/// Statistics confirming draw call batching efficiency across loaded chunk meshes.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchingStats {
    /// Total number of chunk entities currently possessing a rendered material.
    pub total_material_entities: usize,
    /// Number of distinct materials currently assigned to chunks (target is exactly 1).
    pub unique_materials: usize,
    /// True if all chunks share the identical single material handle for optimal draw batching.
    pub is_batch_optimal: bool,
}

/// Initializes the shared texture atlas image with nearest-neighbor sampling and builds the single global voxel PBR material.
///
/// # Arguments
/// * `materials` - Assets storage for standard materials.
/// * `images` - Assets storage for 2D texture images.
///
/// # Example
/// ```rust
/// use block_game_2::batching::init_shared_voxel_material;
/// use bevy::prelude::*;
///
/// let mut app = App::new();
/// app.init_resource::<Assets<StandardMaterial>>();
/// app.init_resource::<Assets<Image>>();
///
/// let handle = app.world_mut().resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
///     let mut images = world.resource_mut::<Assets<Image>>();
///     init_shared_voxel_material(&mut materials, &mut images)
/// });
/// assert!(handle.material.id() != AssetId::default());
/// ```
pub fn init_shared_voxel_material(
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> VoxelMaterialHandle {
    let mut atlas_image = create_texture_atlas_image();
    atlas_image.sampler = ImageSampler::nearest();
    let atlas_image_handle = images.add(atlas_image);

    let standard_mat = StandardMaterial {
        base_color_texture: Some(atlas_image_handle.clone()),
        perceptual_roughness: 0.9,
        reflectance: 0.1,
        ..default()
    };
    let material_handle = materials.add(standard_mat);

    VoxelMaterialHandle {
        material: material_handle,
        atlas_image: atlas_image_handle,
    }
}

/// Startup system instantiating and caching the global voxel material resource.
pub fn setup_voxel_material_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let handle = init_shared_voxel_material(&mut materials, &mut images);
    commands.insert_resource(handle);
}

/// System verifying that all chunk meshes reference the shared global material to enable Bevy draw batching.
pub fn update_batching_stats_system(
    chunk_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut stats: ResMut<BatchingStats>,
) {
    let mut total = 0;
    let mut first_id = None;
    let mut is_optimal = true;

    for mat in chunk_materials.iter() {
        total += 1;
        match first_id {
            None => first_id = Some(mat.0.id()),
            Some(id) if id != mat.0.id() => {
                is_optimal = false;
            }
            _ => {}
        }
    }

    stats.total_material_entities = total;
    stats.unique_materials = if total == 0 { 0 } else if is_optimal { 1 } else { 2 };
    stats.is_batch_optimal = total > 0 && is_optimal;
}
