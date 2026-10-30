# PLAN: World gen module, persistence, egui menus

Requests (from TODO.md, deduped, dependency order): (1) move worldgen to its own module, stop regenerating every launch; (2) world create + `bincode` save; (3) `bevy_egui` main menu, pause menu, create/load/save GUI (dev-simple).

Findings (read-only):
- Worldgen lives in [world_gen.rs](src/world_gen.rs): `TerrainConfig`, `TerrainGenerator`, `spawn_chunk`, `spawn_chunk_async`, `poll_chunk_generation_system`.
- [streaming.rs](src/streaming.rs): `VoxelWorldPlugin` inserts `TerrainGenerator` at plugin build and streams chunks every frame; unloaded chunks are despawned and lost (edits not kept).
- `Chunk` ([chunk.rs](src/chunk.rs)) holds private `Vec<BlockId>`; `BlockId`/`ChunkPos` have no serde derives.
- [fly_cam.rs](src/fly_cam.rs) `cursor_grab_system` already uses Esc to release cursor and left-click to grab; this conflicts with a pause menu.
- [main.rs](src/main.rs) spawns camera and light in `Startup`; the game starts immediately.
- Errors: lib uses `thiserror` ([error.rs](src/error.rs)); bin uses `anyhow`. The crate also builds for WASM ([scripts/build-wasm.sh](scripts/build-wasm.sh)), where there is no filesystem.
- Tests: `cargo nextest run --test-threads 1 --fail-fast`. Style: one type per file, `mod.rs` re-exports only.

### Phase 1: Extract world generation into `worldgen/` module [STATUS: DONE]
- Goal: terrain generation is an isolated module with no behavior change.
- Tasks:
  - [x] Create `src/worldgen/` (`mod.rs` re-exports only); one type per file: `terrain_config.rs`, `terrain_generator.rs`, `generate_chunk_task.rs`; spawn and poll helpers in `chunk_spawn.rs`
  - [x] Remove `world_gen.rs`; update `lib.rs` and imports in `streaming.rs` and the `lib.rs` tests
  - [x] Move the existing worldgen tests into `*_test.rs` files in the new module
  - [x] `VoxelWorldPlugin` stops hard-inserting `TerrainGenerator`; it is provided by a `WorldGenPlugin` (still auto-inserted with defaults for now)
- Done when: build and all existing tests pass; game looks and plays identically.

### Phase 2: World persistence with `bincode` (no GUI) [STATUS: DONE]
- Goal: a world can be created, saved, and loaded from disk; loaded chunks are not regenerated.
- Tasks:
  - [x] Add deps: `bincode` (with `serde` feature) and `serde` (derive); add serde derives to `BlockId`, `ChunkPos`, `Chunk`
  - [x] New `src/save/` module: `world_meta.rs` (name, `TerrainConfig` incl. seed, player position), `world_save.rs` (create/save/load fns), `save_error.rs` (`thiserror`)
  - [x] Layout: `saves/<world_name>/meta.bin` plus `saves/<world_name>/chunks/<x>_<y>_<z>.bin` (one chunk per file keeps saves incremental)
  - [x] `create_world(name, seed)`: writes `meta.bin`, errors if the name exists; validate names (no path separators)
  - [x] Streaming: on chunk request, load from save if the file exists, else generate; mark generated chunks as unsaved
  - [x] Track modified chunks (`ChunkModified` marker set by block edit); `save_world` writes modified chunks and writes on unload so edits survive streaming
  - [x] Gate filesystem code with `#[cfg(not(target_arch = "wasm32"))]` so the WASM build still compiles (saving is a no-op there)
  - [x] Tests (using a temp dir): chunk round-trip, meta round-trip, create-existing errors, load of an edited chunk returns the edit instead of regenerated terrain
- Done when: tests pass; a unit/integration test creates a world, edits a block, saves, reloads, and sees the edit; `cargo build --target wasm32-unknown-unknown` still compiles.

### Phase 3: Game state machine and `bevy_egui` shell [STATUS: DONE]
- Goal: the app starts in a main menu; the world only exists once a world is opened.
- Tasks:
  - [x] Add `bevy_egui` (version compatible with bevy 0.20; verify on crates.io, check it supports bevy 0.20 before adding)
  - [x] `AppState` enum: `MainMenu`, `InGame`, `Paused`; gate streaming, fly cam, block edit, hotbar systems on `InGame` via `run_if(in_state(..))`
  - [x] `ActiveWorld` resource (name, generator, save path), inserted on open and removed on exit; despawn all chunk entities and clear `VoxelWorld` on exit
  - [x] Move camera/light spawn from `main.rs` `Startup` to world-open (`OnEnter(InGame)`), and cleanup on exit
  - [x] Replace Esc handling in `cursor_grab_system` so Esc toggles `InGame` and `Paused` (cursor released while paused)
- Done when: build and tests pass; app launches into an empty main menu state (stub screen) and a temporary dev trigger can enter and leave `InGame` without leaking chunk entities.

### Phase 4: Menus and world create/load/save GUI [STATUS: TODO]
- Goal: all world management is usable from egui (minimal dev UI).
- Tasks:
  - [ ] Main menu: list worlds from `saves/`; buttons Load, Delete (optional skip), "New world" with name and seed text fields; Quit
  - [ ] Pause menu (Esc): Resume, Save, Save and Quit to Menu; show a status line for save success or error
  - [ ] Surface `SaveError` in the UI instead of panicking
  - [ ] Ensure egui consuming pointer/keyboard input does not trigger block edit, hotbar, or camera while a menu is open (`wants_pointer_input` check or state gating)
  - [ ] Tests: world list function (temp dir) and state transitions via a headless `App` with `StatesPlugin`
- Done when: tests pass; manual run: create world, break a block, Esc, Save, quit to menu, load world, see the same terrain and the edit without regeneration.

### Phase 5: Cleanup [STATUS: TODO]
- Goal: docs and repo hygiene.
- Tasks:
  - [ ] Add `saves/` to `.gitignore`
  - [ ] Tick the three items in [TODO.md](TODO.md)
  - [ ] WASM: show "saving unavailable" in the UI (or hide Save/Load) via the cfg gate
- Done when: build, tests, and wasm check pass.

## Unknowns, risks, assumptions
- `bevy_egui` compatibility with bevy 0.20 is unverified; if there is no release, pin a git branch or fall back to waiting. Check before Phase 3.
- Assumption: saves use one file per chunk, not a single world file; revisit if file count becomes a problem.
- Assumption: web/WASM has no persistence for now (cfg-gated no-op). Say so if you want IndexedDB or localStorage.
- Assumption: only the seed is saved plus chunks visited; terrain is deterministic, so unvisited chunks are generated on demand.
- Changing `TerrainConfig` between versions would make old saves inconsistent at chunk borders; meta stores the config to avoid this.
- Existing Esc behavior (release cursor) is replaced by the pause menu.
