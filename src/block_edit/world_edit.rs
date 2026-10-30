use bevy::prelude::*;

use crate::async_mesh::request_chunk_remesh;
use crate::block_edit::BlockEditError;
use crate::chunk::{CHUNK_SIZE, Chunk, BlockId, ChunkPos, world_to_chunk_and_local};
use crate::world::VoxelWorld;

/// Debounce frames for player edits; 0 remeshes on the next meshing pass for responsiveness.
const EDIT_DEBOUNCE_FRAMES: u32 = 0;

/// Reads the block at a world coordinate, or `None` if its chunk is not loaded.
///
/// # Arguments
/// * `voxel_world` - Map from chunk position to chunk entity.
/// * `chunks` - Query over chunk voxel data.
/// * `world_pos` - Global block coordinate.
///
/// # Example
/// ```ignore
/// let block = get_block_at(&voxel_world, &chunks, IVec3::new(0, 10, 0));
/// ```
pub fn get_block_at(
    voxel_world: &VoxelWorld,
    chunks: &Query<&Chunk>,
    world_pos: IVec3,
) -> Option<BlockId> {
    let (chunk_pos, (x, y, z)) = world_to_chunk_and_local(world_pos);
    let entity = voxel_world.get(&chunk_pos)?;
    chunks.get(entity).ok()?.get_block(x, y, z)
}

/// Writes a block at a world coordinate and schedules remeshing of the edited chunk.
/// Loaded neighbor chunks are also remeshed when the edit lies on a shared border,
/// since their meshes depend on this chunk's boundary blocks.
///
/// Returns `Ok(true)` if the block changed, `Ok(false)` if it already held `block`.
///
/// # Arguments
/// * `commands` - Used to flag chunks dirty.
/// * `voxel_world` - Map from chunk position to chunk entity.
/// * `chunks` - Mutable query over chunk voxel data.
/// * `world_pos` - Global block coordinate.
/// * `block` - Block to write.
///
/// # Example
/// ```ignore
/// set_block_at(&mut commands, &voxel_world, &mut chunks, IVec3::new(0, 10, 0), BlockType::Air.into())?;
/// ```
pub fn set_block_at(
    commands: &mut Commands,
    voxel_world: &VoxelWorld,
    chunks: &mut Query<&mut Chunk>,
    world_pos: IVec3,
    block: BlockId,
) -> Result<bool, BlockEditError> {
    let (chunk_pos, (x, y, z)) = world_to_chunk_and_local(world_pos);
    let entity = voxel_world
        .get(&chunk_pos)
        .ok_or(BlockEditError::ChunkNotLoaded(world_pos))?;
    let mut chunk = chunks
        .get_mut(entity)
        .map_err(|_| BlockEditError::ChunkNotLoaded(world_pos))?;

    if !chunk.set_block(x, y, z, block) {
        return Ok(false);
    }

    commands.entity(entity).insert(crate::save::ChunkModified);
    request_chunk_remesh(commands, entity, EDIT_DEBOUNCE_FRAMES, false);

    for (axis, local) in [x, y, z].into_iter().enumerate() {
        let step = if local == 0 {
            -1
        } else if local == CHUNK_SIZE - 1 {
            1
        } else {
            continue;
        };
        let mut offset = IVec3::ZERO;
        offset[axis] = step;
        let neighbor = ChunkPos(chunk_pos.0 + offset);
        if let Some(neighbor_entity) = voxel_world.get(&neighbor) {
            request_chunk_remesh(commands, neighbor_entity, EDIT_DEBOUNCE_FRAMES, false);
        }
    }
    Ok(true)
}
