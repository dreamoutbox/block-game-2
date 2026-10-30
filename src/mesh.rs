use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::chunk::{Chunk, ChunkDirty, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_I32};
use crate::registry::{BlockFace, BlockRegistry, BlockType};
use crate::world::VoxelWorld;

pub const ATLAS_TILES_PER_ROW: u32 = 4;
pub const TILE_PIXEL_SIZE: u32 = 16;
pub const ATLAS_PIXEL_SIZE: u32 = ATLAS_TILES_PER_ROW * TILE_PIXEL_SIZE; // 64x64

/// Generates a procedural 64x64 RGBA texture atlas image with distinct patterns for standard blocks.
///
/// # Example
/// ```rust
/// use block_game_2::mesh::create_texture_atlas_image;
///
/// let image = create_texture_atlas_image();
/// assert_eq!(image.texture_descriptor.size.width, 64);
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
            if py < 4 {
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
