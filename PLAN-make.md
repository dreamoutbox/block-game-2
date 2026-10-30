# PLAN: Flying camera block collision + `C` passthrough

Requests (deduped, dependency-ordered):
1. Prevent the flying camera from passing through solid blocks (foundation: AABB-voxel collision resolution).
2. Allow passing through blocks when `C` is pressed (input integration into `FlyCamera`).

Context (from repo):
- Camera movement is handled in `fly_camera_controller_system` ([fly_cam.rs](src/fly_cam.rs)), which directly sets `transform.translation += move_direction.normalize() * current_speed * time.delta_secs();`.
- World block lookup is in `get_block_at` ([world_edit.rs](src/block_edit/world_edit.rs)), taking `&VoxelWorld`, `&Query<&Chunk>`, and `IVec3`.
- Block solidity is checked via `BlockRegistry::is_solid(BlockId)` ([registry.rs](src/registry.rs)).
- Block placement in `block_edit_input_system` ([block_edit_plugin.rs](src/block_edit/block_edit_plugin.rs)) currently checks `place_pos == origin.floor().as_ivec3()` to avoid placing a block inside the camera.
- New types must live in their own snake_case file per `AGENTS.md`, with `mod.rs` used only for module declarations and re-exports.
- Test command: `cargo nextest run --test-threads 1 --fail-fast`.

### Phase 1: Voxel AABB collision resolver [STATUS: DONE]
- Goal: Pure, Bevy-ECS-independent function that resolves a moving AABB (centered at camera position with configurable half-extents) against solid unit-cube voxels with per-axis sliding.
- Tasks:
  - [x] Define camera collider half-extents constant in [settings.rs](src/settings.rs) (e.g., `CAMERA_COLLIDER_HALF_EXTENTS: Vec3 = Vec3::splat(0.3)` so the camera cannot clip into faces due to the near plane)
  - [x] Add a pure helper function (e.g., `resolve_voxel_collision(pos: Vec3, delta: Vec3, half_extents: Vec3, is_solid: impl Fn(IVec3) -> bool) -> Vec3`) that steps/sweeps axis-by-axis (X, Y, Z) so the camera slides smoothly along walls, floors, and ceilings without tunneling on large frame deltas
  - [x] Add a helper to check whether a candidate block `IVec3` overlaps the camera collider AABB at `pos`, to share between collision and block-placement checks
  - [x] Add doc comments describing function purpose, arguments, and example usage
  - [x] Unit tests (`_test.rs`): stopped by wall on each axis (+X, -X, +Y, -Y, +Z, -Z), diagonal movement slides along wall, corner stops movement on both blocked axes, negative world coordinates, and large delta step clamping (no tunneling through 1-block thick walls)
- Done when: `cargo nextest run --test-threads 1 --fail-fast` passes; resolver is pure and independent of ECS queries.

### Phase 2: Flying camera collision & `C` passthrough [STATUS: DONE]
- Goal: Integrate voxel collision into `fly_camera_controller_system` by default and bypass it when holding `C` (or toggling `C`, see assumptions).
- Tasks:
  - [x] Refactor `src/fly_cam.rs` if adding new types (following one-type-per-file rule in `AGENTS.md`), or keep `FlyCamera` and `FlyCameraPlugin` split cleanly
  - [x] Update `fly_camera_controller_system` to query `Option<Res<VoxelWorld>>`, `Query<&Chunk>`, and `Option<Res<BlockRegistry>>`
  - [x] When `KeyCode::KeyC` is not held (collision active) and world/registry resources are present, resolve `delta` through `resolve_voxel_collision` using `get_block_at` + `registry.is_solid`; when `KeyCode::KeyC` is held (noclip/passthrough active), apply `delta` directly
  - [x] Update block placement overlap check in [block_edit_plugin.rs](src/block_edit/block_edit_plugin.rs) to use the shared camera AABB overlap helper so the player cannot place a solid block intersecting their camera collider
  - [x] Unit/integration tests: camera moving into a solid block stops at the surface when `C` is not pressed; camera moves right through the solid block when `KeyCode::KeyC` is pressed; camera slides along a wall when moving at an angle
  - [x] Mark `- [x] make flying camera not passthrough block. unless press C.` in [TODO.md](TODO.md)
- Done when: `cargo nextest run --test-threads 1 --fail-fast` passes; in-game flying camera collides and slides against terrain blocks and passes through blocks while pressing `C`.

## Unknowns, risks, assumptions
- Assumption (`C` key behavior): Holding `C` (`key_input.pressed(KeyCode::KeyC)`) enables passthrough (noclip) while held, and releasing `C` re-enables block collision. Let me know if you prefer `C` to be a toggle (`just_pressed`) instead.
- Assumption (unloaded chunks): Unloaded chunks return `None` from `get_block_at` and are treated as non-solid (`false`) so the camera is not frozen at startup while initial chunks are still generating asynchronously.
- Assumption (stuck inside block): If the player releases `C` while already inside a solid block, only movement into new solid voxels is clamped, or if already overlapping, we allow moving out / wait until clear rather than NaN/jittering.
- Assumption (collider shape): Symmetric box AABB around the camera eye position (`half_extents = Vec3::splat(0.3)`) rather than a full 1.8m tall player body, since this is a free-flying camera.
