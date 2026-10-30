use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::path::{Path, PathBuf};

use crate::chunk::{Chunk, ChunkDirty, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_I32};
use crate::error::AtlasError;
use crate::registry::{BlockFace, BlockRegistry, BlockType};
use crate::world::VoxelWorld;

pub const ATLAS_TILES_PER_ROW: u32 = 4;
pub const TILE_PIXEL_SIZE: u32 = 64;
pub const ATLAS_PIXEL_SIZE: u32 = ATLAS_TILES_PER_ROW * TILE_PIXEL_SIZE; // 256x256

/// Fixed grass tint matching vanilla Minecraft plains biome.
pub const MINECRAFT_GRASS_TINT: [u8; 3] = [144, 233, 84];

/// Applies an RGB tint to an RGBA image by multiplying its color channels.
///
/// # Arguments
/// * `img` - Mutable reference to the RGBA image to tint in-place.
/// * `tint` - Color multiplier in RGB (`[r, g, b]` in 0..=255).
///
/// # Example
/// ```rust
/// use block_game_2::mesh::{tint_rgba_image, MINECRAFT_GRASS_TINT};
/// use image::RgbaImage;
///
/// let mut img = RgbaImage::from_pixel(1, 1, image::Rgba([156, 156, 156, 255]));
/// tint_rgba_image(&mut img, MINECRAFT_GRASS_TINT);
/// assert!(img.get_pixel(0, 0).0[1] > img.get_pixel(0, 0).0[0]);
/// ```
pub fn tint_rgba_image(img: &mut image::RgbaImage, tint: [u8; 3]) {
    for pixel in img.pixels_mut() {
        pixel.0[0] = ((pixel.0[0] as u16 * tint[0] as u16) / 255) as u8;
        pixel.0[1] = ((pixel.0[1] as u16 * tint[1] as u16) / 255) as u8;
        pixel.0[2] = ((pixel.0[2] as u16 * tint[2] as u16) / 255) as u8;
    }
}

/// Composites an RGBA overlay image onto a base image using standard alpha blending.
///
/// # Arguments
/// * `base` - Destination base image modified in-place.
/// * `overlay` - Source overlay image with alpha transparency.
///
/// # Example
/// ```rust
/// use block_game_2::mesh::composite_rgba_image;
/// use image::RgbaImage;
///
/// let mut base = RgbaImage::from_pixel(1, 1, image::Rgba([100, 100, 100, 255]));
/// let overlay = RgbaImage::from_pixel(1, 1, image::Rgba([0, 200, 0, 128]));
/// composite_rgba_image(&mut base, &overlay);
/// assert!(base.get_pixel(0, 0).0[1] > 100);
/// ```
pub fn composite_rgba_image(base: &mut image::RgbaImage, overlay: &image::RgbaImage) {
    for (x, y, base_pixel) in base.enumerate_pixels_mut() {
        if x < overlay.width() && y < overlay.height() {
            let ov = overlay.get_pixel(x, y);
            let alpha = ov.0[3] as u32;
            if alpha == 0 {
                continue;
            } else if alpha == 255 {
                *base_pixel = *ov;
            } else {
                let inv_alpha = 255 - alpha;
                let r = (ov.0[0] as u32 * alpha + base_pixel.0[0] as u32 * inv_alpha) / 255;
                let g = (ov.0[1] as u32 * alpha + base_pixel.0[1] as u32 * inv_alpha) / 255;
                let b = (ov.0[2] as u32 * alpha + base_pixel.0[2] as u32 * inv_alpha) / 255;
                base_pixel.0 = [r as u8, g as u8, b as u8, 255];
            }
        }
    }
}

/// Blits a `TILE_PIXEL_SIZE x TILE_PIXEL_SIZE` RGBA pixel buffer into the atlas at the specified tile index.
///
/// # Arguments
/// * `atlas_data` - Target atlas raw RGBA buffer of length `(ATLAS_PIXEL_SIZE * ATLAS_PIXEL_SIZE * 4)`.
/// * `tile_index` - Target tile index in the atlas grid.
/// * `tile_rgba` - Source RGBA byte slice of length `(TILE_PIXEL_SIZE * TILE_PIXEL_SIZE * 4)`.
///
/// # Example
/// ```rust
/// use block_game_2::mesh::{blit_tile_rgba, ATLAS_PIXEL_SIZE, TILE_PIXEL_SIZE};
///
/// let mut atlas = vec![0u8; (ATLAS_PIXEL_SIZE * ATLAS_PIXEL_SIZE * 4) as usize];
/// let tile = vec![255u8; (TILE_PIXEL_SIZE * TILE_PIXEL_SIZE * 4) as usize];
/// blit_tile_rgba(&mut atlas, 0, &tile).unwrap();
/// assert_eq!(&atlas[0..4], &[255, 255, 255, 255]);
/// ```
pub fn blit_tile_rgba(
    atlas_data: &mut [u8],
    tile_index: u32,
    tile_rgba: &[u8],
) -> Result<(), AtlasError> {
    let max_tiles = ATLAS_TILES_PER_ROW * ATLAS_TILES_PER_ROW;
    if tile_index >= max_tiles {
        return Err(AtlasError::TileIndexOutOfBounds {
            tile_index,
            max_tiles,
        });
    }

    let expected_tile_bytes = (TILE_PIXEL_SIZE * TILE_PIXEL_SIZE * 4) as usize;
    if tile_rgba.len() != expected_tile_bytes {
        return Err(AtlasError::InvalidDimensions {
            path: PathBuf::from(format!("tile_{tile_index}")),
            expected_w: TILE_PIXEL_SIZE,
            expected_h: TILE_PIXEL_SIZE,
            actual_w: 0,
            actual_h: 0,
        });
    }

    let col = tile_index % ATLAS_TILES_PER_ROW;
    let row = tile_index / ATLAS_TILES_PER_ROW;
    let start_x = col * TILE_PIXEL_SIZE;
    let start_y = row * TILE_PIXEL_SIZE;

    for py in 0..TILE_PIXEL_SIZE {
        let src_row_start = (py * TILE_PIXEL_SIZE * 4) as usize;
        let src_row = &tile_rgba[src_row_start..src_row_start + (TILE_PIXEL_SIZE * 4) as usize];

        let dst_y = start_y + py;
        let dst_row_start = ((dst_y * ATLAS_PIXEL_SIZE + start_x) * 4) as usize;
        atlas_data[dst_row_start..dst_row_start + (TILE_PIXEL_SIZE * 4) as usize]
            .copy_from_slice(src_row);
    }

    Ok(())
}

/// Decodes PNG image bytes in memory into an RGBA8 image.
///
/// # Arguments
/// * `bytes` - In-memory PNG encoded bytes.
///
/// # Example
/// ```rust
/// use block_game_2::mesh::load_png_from_memory;
///
/// const DIRT_BYTES: &[u8] = include_bytes!("../assets/minecraft/textures/block/dirt.png");
/// let img = load_png_from_memory(DIRT_BYTES).unwrap();
/// assert_eq!(img.width(), 64);
/// assert_eq!(img.height(), 64);
/// ```
pub fn load_png_from_memory(bytes: &[u8]) -> Result<image::RgbaImage, AtlasError> {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| AtlasError::Decode {
            path: PathBuf::from("embedded"),
            source: e,
        })?;
    Ok(img.to_rgba8())
}

/// Retrieves embedded raw PNG bytes for standard Minecraft block textures.
///
/// Returns `Some(&'static [u8])` if the requested relative path corresponds to one of the built-in textures,
/// or `None` if it must be loaded from external storage.
///
/// # Arguments
/// * `relative_path` - Path identifying the texture.
///
/// # Example
/// ```rust
/// use block_game_2::mesh::get_embedded_texture_bytes;
///
/// assert!(get_embedded_texture_bytes("minecraft/textures/block/dirt.png").is_some());
/// ```
pub fn get_embedded_texture_bytes(relative_path: &str) -> Option<&'static [u8]> {
    let normalized = relative_path.strip_prefix("assets/").unwrap_or(relative_path);
    match normalized {
        "minecraft/textures/block/dirt.png" => {
            Some(include_bytes!("../assets/minecraft/textures/block/dirt.png"))
        }
        "minecraft/textures/block/grass_block_side.png" => {
            Some(include_bytes!("../assets/minecraft/textures/block/grass_block_side.png"))
        }
        "minecraft/textures/block/grass_block_side_overlay.png" => {
            Some(include_bytes!("../assets/minecraft/textures/block/grass_block_side_overlay.png"))
        }
        "minecraft/textures/block/grass_block_top.png" => {
            Some(include_bytes!("../assets/minecraft/textures/block/grass_block_top.png"))
        }
        _ => None,
    }
}

/// Loads a PNG image from a path relative to the workspace or assets directory and decodes it to RGBA8.
///
/// Checks embedded asset buffers first, making it work in WASM/browser environments where local filesystem
/// access is unavailable, before attempting to load from the filesystem.
///
/// # Arguments
/// * `relative_path` - Path relative to the workspace `assets` directory or project root.
///
/// # Example
/// ```rust
/// use block_game_2::mesh::load_png_rgba;
///
/// let img = load_png_rgba("minecraft/textures/block/dirt.png").unwrap();
/// assert_eq!(img.width(), 64);
/// assert_eq!(img.height(), 64);
/// ```
pub fn load_png_rgba(relative_path: impl AsRef<Path>) -> Result<image::RgbaImage, AtlasError> {
    let rel = relative_path.as_ref();
    let rel_str = rel.to_string_lossy();

    // Check embedded textures first (essential for WASM / web where no filesystem exists)
    if let Some(bytes) = get_embedded_texture_bytes(&rel_str) {
        return load_png_from_memory(bytes);
    }

    let full_path = if rel.starts_with("assets") {
        rel.to_path_buf()
    } else {
        Path::new("assets").join(rel)
    };

    let img = image::ImageReader::open(&full_path)
        .map_err(|e| AtlasError::Io {
            path: full_path.clone(),
            source: e,
        })?
        .decode()
        .map_err(|e| AtlasError::Decode {
            path: full_path.clone(),
            source: e,
        })?;

    Ok(img.to_rgba8())
}

/// Loads a PNG image file from the assets directory and blits it into the specified atlas tile index.
///
/// # Arguments
/// * `atlas_data` - Target atlas raw RGBA buffer of length `(ATLAS_PIXEL_SIZE * ATLAS_PIXEL_SIZE * 4)`.
/// * `tile_index` - Target tile index in the atlas grid.
/// * `relative_path` - Path relative to the workspace `assets` directory or project root.
///
/// # Example
/// ```rust
/// use block_game_2::mesh::{load_and_blit_png, ATLAS_PIXEL_SIZE};
///
/// let mut atlas = vec![0u8; (ATLAS_PIXEL_SIZE * ATLAS_PIXEL_SIZE * 4) as usize];
/// load_and_blit_png(&mut atlas, 2, "minecraft/textures/block/dirt.png").unwrap();
/// ```
pub fn load_and_blit_png(
    atlas_data: &mut [u8],
    tile_index: u32,
    relative_path: impl AsRef<Path>,
) -> Result<(), AtlasError> {
    let rel = relative_path.as_ref();
    let img = load_png_rgba(rel)?;

    if img.width() != TILE_PIXEL_SIZE || img.height() != TILE_PIXEL_SIZE {
        let full_path = if rel.starts_with("assets") {
            rel.to_path_buf()
        } else {
            Path::new("assets").join(rel)
        };
        return Err(AtlasError::InvalidDimensions {
            path: full_path,
            expected_w: TILE_PIXEL_SIZE,
            expected_h: TILE_PIXEL_SIZE,
            actual_w: img.width(),
            actual_h: img.height(),
        });
    }

    blit_tile_rgba(atlas_data, tile_index, img.as_raw())
}

/// Generates a procedural 256x256 RGBA texture atlas image with distinct patterns for standard blocks.
///
/// # Example
/// ```rust
/// use block_game_2::mesh::create_texture_atlas_image;
///
/// let image = create_texture_atlas_image();
/// assert_eq!(image.texture_descriptor.size.width, 256);
/// ```
pub fn create_texture_atlas_image() -> Image {
    let mut data = vec![255u8; (ATLAS_PIXEL_SIZE * ATLAS_PIXEL_SIZE * 4) as usize];

    // Helper closure to color a tile
    let color_tile = |data: &mut [u8], tile_idx: u32, rgba: [u8; 4]| {
        let col = tile_idx % ATLAS_TILES_PER_ROW;
        let row = tile_idx / ATLAS_TILES_PER_ROW;
        let start_x = col * TILE_PIXEL_SIZE;
        let start_y = row * TILE_PIXEL_SIZE;

        for py in 0..TILE_PIXEL_SIZE {
            for px in 0..TILE_PIXEL_SIZE {
                let x = start_x + px;
                let y = start_y + py;
                let idx = ((y * ATLAS_PIXEL_SIZE + x) * 4) as usize;
                data[idx..idx + 4].copy_from_slice(&rgba);
            }
        }
    };

    // Tile 0: Air / Transparent
    color_tile(&mut data, 0, [0, 0, 0, 0]);
    // Tile 1: Stone (Gray)
    color_tile(&mut data, 1, [128, 128, 128, 255]);
    // Tile 2: Dirt (Brown)
    color_tile(&mut data, 2, [139, 69, 19, 255]);
    // Tile 3: Grass Side (Brown bottom, green top)
    let col = 3 % ATLAS_TILES_PER_ROW;
    let row = 3 / ATLAS_TILES_PER_ROW;
    let start_x = col * TILE_PIXEL_SIZE;
    let start_y = row * TILE_PIXEL_SIZE;
    for py in 0..TILE_PIXEL_SIZE {
        for px in 0..TILE_PIXEL_SIZE {
            let x = start_x + px;
            let y = start_y + py;
            let idx = ((y * ATLAS_PIXEL_SIZE + x) * 4) as usize;
            if py < 16 {
                data[idx..idx + 4].copy_from_slice(&[34, 139, 34, 255]); // Green top border
            } else {
                data[idx..idx + 4].copy_from_slice(&[139, 69, 19, 255]); // Dirt base
            }
        }
    }
    // Tile 4: Grass Top (Vibrant Green)
    color_tile(&mut data, 4, [50, 205, 50, 255]);
    // Tile 5: Wood (Dark Brown)
    color_tile(&mut data, 5, [101, 67, 33, 255]);
    // Tile 6: Leaves (Green Foliage)
    color_tile(&mut data, 6, [46, 125, 50, 240]);

    // Attempt loading Minecraft textures from assets, falling back gracefully to procedural patterns
    if let Err(err) = load_and_blit_png(&mut data, 2, "minecraft/textures/block/dirt.png") {
        warn!(?err, "Failed to load dirt block texture; falling back to procedural color");
    }

    match load_png_rgba("minecraft/textures/block/grass_block_side.png") {
        Ok(mut side_img) => {
            if let Ok(mut overlay_img) =
                load_png_rgba("minecraft/textures/block/grass_block_side_overlay.png")
            {
                tint_rgba_image(&mut overlay_img, MINECRAFT_GRASS_TINT);
                composite_rgba_image(&mut side_img, &overlay_img);
            }
            if let Err(err) = blit_tile_rgba(&mut data, 3, side_img.as_raw()) {
                warn!(?err, "Failed to blit grass side texture");
            }
        }
        Err(err) => {
            warn!(
                ?err,
                "Failed to load grass block side texture; falling back to procedural color"
            );
        }
    }

    match load_png_rgba("minecraft/textures/block/grass_block_top.png") {
        Ok(mut top_img) => {
            tint_rgba_image(&mut top_img, MINECRAFT_GRASS_TINT);
            if let Err(err) = blit_tile_rgba(&mut data, 4, top_img.as_raw()) {
                warn!(?err, "Failed to blit grass top texture");
            }
        }
        Err(err) => {
            warn!(
                ?err,
                "Failed to load grass block top texture; falling back to procedural color"
            );
        }
    }

    Image::new(
        Extent3d {
            width: ATLAS_PIXEL_SIZE,
            height: ATLAS_PIXEL_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Resource caching the shared voxel texture atlas and standard material.
#[derive(Resource, Clone)]
pub struct VoxelMaterialHandle {
    pub material: Handle<StandardMaterial>,
    pub atlas_image: Handle<Image>,
}

/// System building chunk meshes synchronously for all entities marked with `ChunkDirty`.
pub fn sync_remesh_dirty_chunks_system(
    mut commands: Commands,
    dirty_chunks: Query<(Entity, &ChunkPos, &Chunk), With<ChunkDirty>>,
    all_chunks: Query<&Chunk>,
    voxel_world: Res<VoxelWorld>,
    registry: Res<BlockRegistry>,
    material_res: Option<Res<VoxelMaterialHandle>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, &pos, chunk) in dirty_chunks.iter() {
        let get_chunk = |neighbor_pos: ChunkPos| -> Option<&Chunk> {
            let neighbor_entity = voxel_world.get(&neighbor_pos)?;
            all_chunks.get(neighbor_entity).ok()
        };

        let mesh_opt = build_chunk_mesh(pos, chunk, &get_chunk, &registry);

        let translation = Vec3::new(
            (pos.0.x * CHUNK_SIZE_I32) as f32,
            (pos.0.y * CHUNK_SIZE_I32) as f32,
            (pos.0.z * CHUNK_SIZE_I32) as f32,
        );

        let mut entity_cmds = commands.entity(entity);
        entity_cmds.insert((
            Transform::from_translation(translation),
            Visibility::default(),
            crate::culling::chunk_local_aabb(),
        ));

        if let Some(mesh) = mesh_opt {
            let mesh_handle = meshes.add(mesh);
            entity_cmds.insert(Mesh3d(mesh_handle));
            if let Some(ref mat) = material_res {
                entity_cmds.insert(MeshMaterial3d(mat.material.clone()));
            }
        } else {
            entity_cmds.remove::<Mesh3d>();
            entity_cmds.remove::<MeshMaterial3d<StandardMaterial>>();
        }

        if crate::occlusion::check_chunk_solid(chunk, &registry) {
            entity_cmds.insert(crate::occlusion::SolidChunk);
        } else {
            entity_cmds.remove::<crate::occlusion::SolidChunk>();
        }

        entity_cmds.remove::<ChunkDirty>();
    }
}

/// Generates UV coordinates for a single quad corresponding to `tile_index` in a grid atlas.
///
/// # Arguments
/// * `tile_index` - Index of the texture tile in the atlas.
///
/// # Example
/// ```rust
/// use block_game_2::mesh::tile_uvs;
///
/// let uvs = tile_uvs(0);
/// assert_eq!(uvs.len(), 4);
/// ```
pub fn tile_uvs(tile_index: u32) -> [[f32; 2]; 4] {
    let tiles_per_row = ATLAS_TILES_PER_ROW as f32;
    let tile_size = 1.0 / tiles_per_row;
    let col = (tile_index % ATLAS_TILES_PER_ROW) as f32;
    let row = (tile_index / ATLAS_TILES_PER_ROW) as f32;

    let u_min = col * tile_size;
    let u_max = u_min + tile_size;
    let v_min = row * tile_size;
    let v_max = v_min + tile_size;

    [
        [u_min, v_max],
        [u_max, v_max],
        [u_max, v_min],
        [u_min, v_min],
    ]
}

/// Geometry buffers accumulating vertices, normals, UVs, and indices for a chunk mesh.
#[derive(Default)]
pub struct ChunkMeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl ChunkMeshBuilder {
    /// Appends a single quad face to the mesh builder.
    ///
    /// # Arguments
    /// * `face` - Cardinal face orientation.
    /// * `lx` - Local X coordinate (0..16).
    /// * `ly` - Local Y coordinate (0..16).
    /// * `lz` - Local Z coordinate (0..16).
    /// * `tile_index` - Texture tile index in atlas.
    pub fn add_face(&mut self, face: BlockFace, lx: usize, ly: usize, lz: usize, tile_index: u32) {
        let x = lx as f32;
        let y = ly as f32;
        let z = lz as f32;

        let face_positions: [[f32; 3]; 4] = match face {
            BlockFace::PosY => [
                [x, y + 1.0, z + 1.0],
                [x + 1.0, y + 1.0, z + 1.0],
                [x + 1.0, y + 1.0, z],
                [x, y + 1.0, z],
            ],
            BlockFace::NegY => [
                [x, y, z],
                [x + 1.0, y, z],
                [x + 1.0, y, z + 1.0],
                [x, y, z + 1.0],
            ],
            BlockFace::PosX => [
                [x + 1.0, y, z + 1.0],
                [x + 1.0, y, z],
                [x + 1.0, y + 1.0, z],
                [x + 1.0, y + 1.0, z + 1.0],
            ],
            BlockFace::NegX => [
                [x, y, z],
                [x, y, z + 1.0],
                [x, y + 1.0, z + 1.0],
                [x, y + 1.0, z],
            ],
            BlockFace::PosZ => [
                [x, y, z + 1.0],
                [x + 1.0, y, z + 1.0],
                [x + 1.0, y + 1.0, z + 1.0],
                [x, y + 1.0, z + 1.0],
            ],
            BlockFace::NegZ => [
                [x + 1.0, y, z],
                [x, y, z],
                [x, y + 1.0, z],
                [x + 1.0, y + 1.0, z],
            ],
        };

        let normal_ivec = face.normal();
        let normal = [
            normal_ivec.x as f32,
            normal_ivec.y as f32,
            normal_ivec.z as f32,
        ];

        let face_uvs = tile_uvs(tile_index);
        let base_idx = self.positions.len() as u32;

        self.positions.extend_from_slice(&face_positions);
        self.normals.extend_from_slice(&[normal; 4]);
        self.uvs.extend_from_slice(&face_uvs);

        self.indices.extend_from_slice(&[
            base_idx,
            base_idx + 1,
            base_idx + 2,
            base_idx,
            base_idx + 2,
            base_idx + 3,
        ]);
    }

    /// Converts accumulated geometry into a Bevy `Mesh`, returning None if no faces were added.
    pub fn build(self) -> Option<Mesh> {
        if self.positions.is_empty() {
            return None;
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));
        Some(mesh)
    }

    /// Returns the number of quad faces currently queued in the builder.
    pub fn quad_count(&self) -> usize {
        self.indices.len() / 6
    }
}

/// Evaluates if neighbor block at given offset relative to `chunk` is opaque.
///
/// # Arguments
/// * `pos` - Target chunk position.
/// * `nx` - Neighbor X block index (-1..=16).
/// * `ny` - Neighbor Y block index (-1..=16).
/// * `nz` - Neighbor Z block index (-1..=16).
/// * `chunk` - Center chunk voxel storage.
/// * `get_chunk` - Closure querying adjacent chunk by ChunkPos.
/// * `registry` - Block descriptor registry.
pub fn is_neighbor_opaque<'a, F>(
    pos: ChunkPos,
    nx: i32,
    ny: i32,
    nz: i32,
    chunk: &Chunk,
    get_chunk: &F,
    registry: &BlockRegistry,
) -> bool
where
    F: Fn(ChunkPos) -> Option<&'a Chunk>,
{
    if nx >= 0 && nx < CHUNK_SIZE as i32 && ny >= 0 && ny < CHUNK_SIZE as i32 && nz >= 0 && nz < CHUNK_SIZE as i32 {
        let block_id = chunk.get_block(nx as usize, ny as usize, nz as usize).unwrap_or(BlockType::Air.into());
        registry.is_opaque(block_id)
    } else {
        let chunk_dx = nx.div_euclid(CHUNK_SIZE as i32);
        let chunk_dy = ny.div_euclid(CHUNK_SIZE as i32);
        let chunk_dz = nz.div_euclid(CHUNK_SIZE as i32);

        let neighbor_pos = ChunkPos::new(
            pos.0.x + chunk_dx,
            pos.0.y + chunk_dy,
            pos.0.z + chunk_dz,
        );
        let lx = nx.rem_euclid(CHUNK_SIZE as i32) as usize;
        let ly = ny.rem_euclid(CHUNK_SIZE as i32) as usize;
        let lz = nz.rem_euclid(CHUNK_SIZE as i32) as usize;

        if let Some(neighbor_chunk) = get_chunk(neighbor_pos) {
            let block_id = neighbor_chunk.get_block(lx, ly, lz).unwrap_or(BlockType::Air.into());
            registry.is_opaque(block_id)
        } else {
            false
        }
    }
}

/// Builds a 3D mesh for a single chunk, culling faces against opaque neighbors.
/// Returns None if the chunk has no exposed faces (e.g. completely air or fully buried underground).
///
/// # Arguments
/// * `pos` - Position of the chunk.
/// * `chunk` - Voxel block data for the chunk.
/// * `get_chunk` - Callback returning adjacent chunks if available.
/// * `registry` - Block registry for opacity and texture mapping.
///
/// # Example
/// ```rust
/// use block_game_2::mesh::build_chunk_mesh;
/// use block_game_2::chunk::{Chunk, ChunkPos, BlockId};
/// use block_game_2::registry::BlockRegistry;
///
/// let mut chunk = Chunk::new(BlockId(0));
/// chunk.set_block(0, 0, 0, BlockId(1));
/// let registry = BlockRegistry::with_defaults();
/// let mesh = build_chunk_mesh(ChunkPos::new(0, 0, 0), &chunk, &|_| None, &registry);
/// assert!(mesh.is_some());
/// ```
pub fn build_chunk_mesh<'a, F>(
    pos: ChunkPos,
    chunk: &Chunk,
    get_chunk: &F,
    registry: &BlockRegistry,
) -> Option<Mesh>
where
    F: Fn(ChunkPos) -> Option<&'a Chunk>,
{
    let mut builder = ChunkMeshBuilder::default();

    for lz in 0..CHUNK_SIZE {
        for ly in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let Some(block_id) = chunk.get_block(lx, ly, lz) else { continue; };
                if block_id == BlockType::Air.into() {
                    continue;
                }
                let Some(desc) = registry.get(block_id) else { continue; };

                for (face_idx, &face) in BlockFace::ALL.iter().enumerate() {
                    let normal = face.normal();
                    let nx = lx as i32 + normal.x;
                    let ny = ly as i32 + normal.y;
                    let nz = lz as i32 + normal.z;

                    let neighbor_opaque = is_neighbor_opaque(pos, nx, ny, nz, chunk, get_chunk, registry);
                    if !neighbor_opaque {
                        let tile_index = desc.texture_indices[face_idx];
                        builder.add_face(face, lx, ly, lz, tile_index);
                    }
                }
            }
        }
    }

    builder.build()
}
