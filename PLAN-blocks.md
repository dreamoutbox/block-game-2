# PLAN: Block place/break + hotbar

Requests (deduped, dependency-ordered):
1. Break and place blocks (foundation: world edit API, then raycast, then input).
2. Player tool bar (hotbar) GUI showing grass block and dirt block; selection drives which block is placed.

Context (from repo):
- Chunks are ECS entities with `Chunk` + `ChunkPos` ([chunk.rs](src/chunk.rs)); `world_to_chunk_and_local`, `Chunk::set_block/get_block` exist. `VoxelWorld` maps `ChunkPos` to `Entity` ([world.rs](src/world.rs)).
- Remesh: `request_chunk_remesh` ([async_mesh.rs](src/async_mesh.rs)) with debounce and requeue. Neighbor chunks are snapshotted, so edits on chunk borders must also remesh the adjacent chunk.
- Camera = `Player` + `FlyCamera` ([fly_cam.rs](src/fly_cam.rs)); left click currently grabs the cursor (`cursor_grab_system`), so edit input must only fire while the cursor is locked.
- `BlockType` / `BlockRegistry` in [registry.rs](src/registry.rs) (`is_solid`, `is_opaque`).
- Plugin wiring in [streaming.rs](src/streaming.rs) (`VoxelWorldPlugin`); `main.rs` only spawns scene.
- GUI sprites available: `assets/minecraft/textures/gui/sprites/hud/hotbar.png`, `hotbar_selection.png`. Block icons are not available as 2D files; use the atlas tiles (top/side) or a flat-color/face-texture swatch.
- Test command: `cargo nextest run --test-threads 1 --fail-fast`.
- New types go in their own file (AGENTS.md); `mod.rs` only wiring.

### Phase 1: World block edit API [STATUS: DONE]
- Goal: set/get a block by world coordinate and trigger correct remeshing, without any input yet.
- Tasks:
  - [x] Add a shared helper (new module, e.g. `block_edit`) to read a block at `IVec3` world pos across chunks (via `VoxelWorld` + `Query<&Chunk>`), returning `None` for unloaded chunks
  - [x] Add a helper to write a block at world pos: mutate `Chunk`, call `request_chunk_remesh` for the chunk
  - [x] When the local coord is on a chunk border (0 or max), also remesh the adjacent chunk(s) on that axis if loaded
  - [x] Typed error (`thiserror`) for unloaded chunk / out-of-range, no `unwrap`
  - [x] Doc comments with args and example usage
  - [x] Unit tests (`_test.rs`): edit inside a chunk marks it dirty; edit on border marks neighbor dirty; unloaded chunk returns error
- Done when: build and all tests pass; no gameplay change yet.

### Phase 2: Voxel raycast [STATUS: DONE]
- Goal: given a ray, find the first solid block hit and the face entered (for place position).
- Tasks:
  - [x] Pure function: Amanatides-Woo voxel DDA taking origin, direction, max distance, and a `Fn(IVec3) -> bool` solidity lookup; returns hit block pos + face normal
  - [x] Reach distance as a named constant (default 5.0) in [settings.rs](src/settings.rs)
  - [x] Unit tests: hit straight on each axis, hit from inside gap, miss beyond reach, normal correctness, negative coordinates
- Done when: tests pass; function is pure and independent of Bevy ECS.

### Phase 3: Break and place input [STATUS: DONE]
- Goal: left click breaks the targeted block, right click places the selected block on the hit face.
- Tasks:
  - [x] Add `BlockEditPlugin` (new file) with a system using camera `Transform` forward ray, Phase 2 raycast, Phase 1 edit API
  - [x] Only act when cursor grab mode is `Locked`; the click that grabs the cursor must not also break a block (order system vs `cursor_grab_system`, or check state before grab)
  - [x] Break: set target to `Air`
  - [x] Place: set `hit + normal` to selected block; skip if that cell is non-air; skip if it intersects the player camera position (simple AABB/point check)
  - [x] Selected block as a `Resource` (`SelectedBlock`, defaulting to `Grass`) so Phase 4 can drive it
  - [x] Click rate limiting (act on `just_pressed`; hold-repeat optional, out of scope)
  - [x] Register plugin in `VoxelWorldPlugin`; no changes to `main.rs` beyond that if possible
  - [x] Add a tiny crosshair UI node so the target point is visible
  - [x] Manual check: break a block, place on all 6 faces, edit at chunk borders with no seams or stale faces
- Done when: build and tests pass; in game blocks can be broken and placed, including across chunk borders.

### Phase 4: Hotbar GUI [STATUS: DONE]
- Goal: bottom-center hotbar showing grass and dirt; selection controls the placed block.
- Tasks:
  - [x] `Hotbar` resource: ordered slots as `Vec<BlockType>` (initially Grass, Dirt) and selected index; enum-based, no string literals
  - [x] Bevy UI: bottom-center container using `hotbar.png` background (scaled with nearest filtering) and `hotbar_selection.png` highlight node
  - [x] Slot icons: image node per slot using the block's atlas tile (top texture) cropped via `ImageNode` rect, or a colored swatch fallback
  - [x] Input: keys `1`-`9` and mouse wheel change selection (clamped to slot count); keep selection in sync with `SelectedBlock`
  - [x] Update system moves the selection highlight when selection changes
  - [x] Unit tests: wheel/key selection clamps and wraps as chosen; `SelectedBlock` follows hotbar
  - [x] Mark both TODO.md items as done
- Done when: build and tests pass; hotbar visible with grass and dirt, selecting a slot changes which block is placed.

## Unknowns, risks, assumptions
- Assumption: no collision/physics exists (fly camera), so placing only blocks the camera cell itself.
- Assumption: slot icons use the top atlas tile (flat 2D), not an isometric 3D render. Say so if you want pseudo-3D icons.
- Assumption: hotbar `hotbar.png` is 182x22 (vanilla); scale factor chosen at implementation.
- Risk: `ChunkSnapshot::from_world` clones whole chunks per remesh, so rapid edits may be costly; debounce already exists. Measure before optimizing.
- Risk: WASM build shares input code; pointer-lock click handling may differ in browser, verify.
- Mouse wheel wrap vs clamp: defaulting to wrap (Minecraft behavior).
