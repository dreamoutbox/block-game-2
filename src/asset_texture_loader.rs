use bevy::prelude::*;
use crate::mesh::{
    blit_tile_rgba, composite_rgba_image, tint_rgba_image, VoxelMaterialHandle,
    MINECRAFT_GRASS_TINT,
};

/// Resource managing asynchronous loading of block textures via Bevy's [`AssetServer`].
#[derive(Resource, Debug, Clone)]
pub struct AssetTextureLoader {
    /// Handle for the base dirt block texture.
    pub dirt: Handle<Image>,
    /// Handle for the grass block side texture.
    pub grass_side: Handle<Image>,
    /// Handle for the grass block side foliage overlay texture.
    pub grass_side_overlay: Handle<Image>,
    /// Handle for the grass block top texture.
    pub grass_top: Handle<Image>,
    /// Flag indicating whether textures have been loaded and blitted into the atlas.
    pub is_loaded: bool,
}

impl AssetTextureLoader {
    /// Creates a new texture loader and initiates asynchronous asset requests.
    ///
    /// # Arguments
    /// * `asset_server` - Reference to Bevy's asset server used to request textures.
    ///
    /// # Example
    /// ```rust,no_run
    /// use bevy::prelude::*;
    /// use block_game_2::asset_texture_loader::AssetTextureLoader;
    ///
    /// fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    ///     commands.insert_resource(AssetTextureLoader::new(&asset_server));
    /// }
    /// ```
    pub fn new(asset_server: &AssetServer) -> Self {
        Self {
            dirt: asset_server.load("minecraft/textures/block/dirt.png"),
            grass_side: asset_server.load("minecraft/textures/block/grass_block_side.png"),
            grass_side_overlay: asset_server
                .load("minecraft/textures/block/grass_block_side_overlay.png"),
            grass_top: asset_server.load("minecraft/textures/block/grass_block_top.png"),
            is_loaded: false,
        }
    }
}

/// System initializing texture loading during startup.
///
/// # Arguments
/// * `commands` - Entity and resource commands.
/// * `asset_server` - Bevy asset server resource.
pub fn setup_asset_texture_loader_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(AssetTextureLoader::new(&asset_server));
}

/// System checking whether all block textures have loaded from the asset server,
/// then blitting them into the shared voxel texture atlas.
///
/// # Arguments
/// * `loader` - Optional texture loader resource.
/// * `asset_server` - Asset server resource checking load state.
/// * `material_handle` - Optional handle to the shared atlas material and image.
/// * `images` - Image asset storage.
pub fn update_atlas_from_asset_server_system(
    loader: Option<ResMut<AssetTextureLoader>>,
    material_handle: Option<Res<VoxelMaterialHandle>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut loader) = loader else { return; };
    if loader.is_loaded {
        return;
    }
    let Some(material_handle) = material_handle else { return; };

    // Wait until all 4 textures are loaded and available in Assets<Image>
    let (Some(dirt_img), Some(side_img), Some(overlay_img), Some(top_img)) = (
        images.get(&loader.dirt),
        images.get(&loader.grass_side),
        images.get(&loader.grass_side_overlay),
        images.get(&loader.grass_top),
    ) else {
        return;
    };

    let Ok(dirt_dyn) = dirt_img.clone().try_into_dynamic() else {
        warn!("Failed to convert dirt image into dynamic image");
        return;
    };
    let Ok(side_dyn) = side_img.clone().try_into_dynamic() else {
        warn!("Failed to convert grass side image into dynamic image");
        return;
    };
    let Ok(overlay_dyn) = overlay_img.clone().try_into_dynamic() else {
        warn!("Failed to convert grass side overlay image into dynamic image");
        return;
    };
    let Ok(top_dyn) = top_img.clone().try_into_dynamic() else {
        warn!("Failed to convert grass top image into dynamic image");
        return;
    };

    let dirt_rgba = dirt_dyn.to_rgba8();
    let mut side_rgba = side_dyn.to_rgba8();
    let mut overlay_rgba = overlay_dyn.to_rgba8();
    let mut top_rgba = top_dyn.to_rgba8();

    // Apply Minecraft plains grass tint and composite overlay
    tint_rgba_image(&mut overlay_rgba, MINECRAFT_GRASS_TINT);
    composite_rgba_image(&mut side_rgba, &overlay_rgba);
    tint_rgba_image(&mut top_rgba, MINECRAFT_GRASS_TINT);

    if let Some(atlas_image) = images.get_mut(&material_handle.atlas_image) {
        if let Some(atlas_data) = atlas_image.into_inner().data.as_mut() {
            if let Err(err) = blit_tile_rgba(atlas_data, 2, dirt_rgba.as_raw()) {
                warn!(?err, "Failed to blit dirt texture to atlas");
            }
            if let Err(err) = blit_tile_rgba(atlas_data, 3, side_rgba.as_raw()) {
                warn!(?err, "Failed to blit grass side texture to atlas");
            }
            if let Err(err) = blit_tile_rgba(atlas_data, 4, top_rgba.as_raw()) {
                warn!(?err, "Failed to blit grass top texture to atlas");
            }

            loader.is_loaded = true;
            info!("Successfully populated voxel texture atlas from AssetServer textures");
        }
    }
}
