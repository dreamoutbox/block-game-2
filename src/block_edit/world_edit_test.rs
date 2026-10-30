use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use crate::block_edit::{BlockEditError, get_block_at, set_block_at};
use crate::chunk::{BlockId, Chunk, ChunkDirty, ChunkPos};
use crate::world::VoxelWorld;

/// Spawns loaded empty chunks at the given positions and returns the world plus entities.
fn setup(positions: &[ChunkPos]) -> (World, Vec<Entity>) {
    let mut world = World::new();
    let mut voxel = VoxelWorld::default();
    let mut entities = Vec::new();
    for &pos in positions {
        let e = world.spawn((pos, Chunk::new(BlockId(0)))).id();
        voxel.insert(pos, e);
        entities.push(e);
    }
    world.insert_resource(voxel);
    (world, entities)
}

fn edit(world: &mut World, pos: IVec3, block: BlockId) -> Result<bool, BlockEditError> {
    world
        .run_system_once(
            move |mut commands: Commands, vw: Res<VoxelWorld>, mut q: Query<&mut Chunk>| {
                set_block_at(&mut commands, &vw, &mut q, pos, block)
            },
        )
        .expect("system runs")
}

#[test]
fn edit_inside_chunk_marks_only_that_chunk_dirty() {
    let (mut world, e) = setup(&[ChunkPos::new(0, 0, 0), ChunkPos::new(1, 0, 0)]);
    assert_eq!(edit(&mut world, IVec3::new(5, 5, 5), BlockId(2)), Ok(true));
    assert!(world.entity(e[0]).contains::<ChunkDirty>());
    assert!(!world.entity(e[1]).contains::<ChunkDirty>());

    let got = world
        .run_system_once(|vw: Res<VoxelWorld>, q: Query<&Chunk>| {
            get_block_at(&vw, &q, IVec3::new(5, 5, 5))
        })
        .expect("system runs");
    assert_eq!(got, Some(BlockId(2)));
}

#[test]
fn edit_on_border_marks_neighbor_dirty() {
    let (mut world, e) = setup(&[ChunkPos::new(0, 0, 0), ChunkPos::new(1, 0, 0)]);
    assert_eq!(edit(&mut world, IVec3::new(15, 5, 5), BlockId(2)), Ok(true));
    assert!(world.entity(e[0]).contains::<ChunkDirty>());
    assert!(world.entity(e[1]).contains::<ChunkDirty>());
}

#[test]
fn same_block_is_noop() {
    let (mut world, e) = setup(&[ChunkPos::new(0, 0, 0)]);
    assert_eq!(edit(&mut world, IVec3::new(1, 1, 1), BlockId(0)), Ok(false));
    assert!(!world.entity(e[0]).contains::<ChunkDirty>());
}

#[test]
fn unloaded_chunk_errors() {
    let (mut world, _) = setup(&[ChunkPos::new(0, 0, 0)]);
    let pos = IVec3::new(100, 0, 0);
    assert_eq!(
        edit(&mut world, pos, BlockId(1)),
        Err(BlockEditError::ChunkNotLoaded(pos))
    );
}
