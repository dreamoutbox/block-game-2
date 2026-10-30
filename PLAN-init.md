# PLAN: Voxel renderer (Minetest techniques in Rust + Bevy)

Scope: replicate Minetest's 6 perf techniques in Bevy — chunked mapblocks,
per-face culling, async meshing, frustum culling, occlusion culling, draw
batching. Greedy meshing intentionally excluded (per prior discussion: bad
fit for frequently-edited voxel worlds).

## Phase 1: Chunk data model [STATUS: DONE]
- Goal: in-memory voxel world, chunked like Minetest's 16³ MapBlocks.
- Tasks:
  - [x] Pin `bevy = "0.20.1"` in `Cargo.toml`
  - [x] `Chunk` component: `[[[BlockId; 16]; 16]; 16]` or flat `Vec<BlockId>` (flat + index fn, avoid 3D array cache-miss patterns)
  - [x] `ChunkPos` (i32x3) + `World` resource: `HashMap<ChunkPos, Entity>`
  - [x] Block registry (resource): id → `is_opaque`, `is_solid`, texture indices
  - [x] Dirty-flag component (`ChunkDirty`) set on block edit, cleared after remesh
  - [x] Unit tests: block get/set, chunk↔world coord conversion, dirty flag lifecycle
- Done when: chunks can be created, edited, and queried in isolation (no rendering yet); `cargo test` green.

## Phase 2: World generation [STATUS: DONE]
- Goal: procedurally populate chunks so later phases have a real world to render/cull, not hand-stubbed data.
- Tasks:
  - [x] Noise-based heightmap generator (e.g. `noise` crate, FBM/Perlin) → terrain surface per column
  - [x] Column → block-column fill rule (stone below surface, dirt/grass near surface, air above)
  - [x] Generator runs per-chunk, keyed by `ChunkPos`, pure function of world seed (deterministic, no shared mutable state — needed for later async generation)
  - [x] Wire into chunk load path: chunk spawn triggers generation if not already in world storage
  - [x] Unit tests: determinism (same seed+pos → same chunk), surface continuity across chunk boundaries (no seams)
- Done when: a multi-chunk area generates deterministically from a seed with continuous terrain across chunk borders; `cargo test` green.

## Phase 3: Naive mesher + static rendering [STATUS: DONE]
- Goal: get something on screen, correctness baseline before optimizing.
- Tasks:
  - [x] Per-chunk mesh builder: one quad per exposed block face (6 faces, no merging)
  - [x] Face culling rule: skip face if neighbor block (same chunk or adjacent chunk) is opaque — mirrors Minetest's "don't draw hidden faces"
  - [x] Handle chunk-boundary neighbor lookups (read into adjacent chunk's border layer)
  - [x] Spawn `PbrBundle`/`Mesh3d` per chunk entity, rebuild mesh synchronously on dirty flag (sync for now — async comes in Phase 3)
  - [x] Basic texture atlas + UV mapping per face
- Done when: a static multi-chunk world renders correctly with no missing/duplicate faces; editing a block updates its chunk mesh next frame.

## Phase 4: Async meshing [STATUS: DONE]
- Goal: move mesh (re)generation off the main thread — Minetest's queued background meshing, no frame spikes on edit.
- Tasks:
  - [x] Move mesh-building (Phase 3 algorithm) into `AsyncComputeTaskPool` tasks, keyed by `ChunkPos`
  - [x] `Task<Mesh>` component pattern: poll each frame, swap into `Mesh3d` handle when ready
  - [x] Dedup/coalesce: multiple dirty events for same chunk in one frame → single remesh task
  - [x] Debounce rapid edits (e.g. drag-placing blocks) so meshing isn't resubmitted every tick
  - [x] Border-read snapshot: task must read a copied/Arc'd slice of neighbor data, not live chunk refs (avoid borrow/send issues across threads)
- Done when: editing blocks under sustained load (e.g. scripted stress test placing/removing many blocks) shows no main-thread frame-time spike attributable to meshing; mesh correctness unchanged from Phase 2.

## Phase 5: Frustum culling [STATUS: TODO]
- Goal: skip rendering chunks outside the camera view, like Minetest's frustum fix.
- Tasks:
  - [ ] Verify Bevy's built-in `Frustum`/`VisibleEntities` culling is active on chunk entities (correct `Aabb`/bounding volume per chunk mesh)
  - [ ] Ensure chunk `Aabb` is set explicitly (don't rely on auto-calc being correct for sparse/empty chunks)
  - [ ] Sanity check: disable all solid chunks outside view, confirm draw call count via Bevy diagnostics/`RenderDiagnosticsPlugin`
- Done when: moving camera shows draw-call count tracking visible chunk count only, confirmed via diagnostics overlay.

## Phase 6: Occlusion culling [STATUS: TODO]
- Goal: skip chunks hidden behind other solid chunks, like Minetest's raycast occlusion check.
- Tasks:
  - [ ] Per-chunk occlusion test: sample ray from camera to chunk center (+ corners for accuracy), step through intervening chunks checking solidity
  - [ ] Run occlusion pass each frame only on frustum-visible chunks (cheap prefilter from Phase 5 first)
  - [ ] Randomize/jitter sample ray per Minetest's later fix (avoid systematic false negatives from fixed sampling)
  - [ ] Distance-based cheapening: coarser/fewer samples for far chunks
  - [ ] Toggle resource (`OcclusionCullingEnabled`) for A/B perf comparison
- Done when: a wall-occluded cave/room's chunks are measurably excluded from draw calls (diagnostics counter drops) with scene still rendering correctly when toggled off vs on.

## Phase 7: Draw batching [STATUS: TODO]
- Goal: reduce texture/material switches, like Minetest's texture-sorted draw list.
- Tasks:
  - [ ] Single shared texture atlas + one material for all chunk meshes (avoids per-chunk material swaps entirely — stronger than Minetest's sort-by-texture, since Bevy batches same-material meshes automatically)
  - [ ] Confirm Bevy's automatic instancing/batching is kicking in for same-material chunk meshes (check render stats)
  - [ ] If using `bevy_render` custom pipeline for voxel chunks (optional), batch draw calls by material/pipeline key explicitly
- Done when: diagnostics show draw calls roughly proportional to visible chunks, not visible faces/blocks, with a single material bound across the world.

## Phase 8: Integration + load/stream loop [STATUS: TODO]
- Goal: tie chunking + generation + meshing + culling together into a working streaming world (load/unload chunks around player).
- Tasks:
  - [ ] Chunk load/unload system based on player position + render distance
  - [ ] Wire generate-on-load (Phase 2) → dirty → async mesh (Phase 4) → cull (Phase 5/6) → batched draw (Phase 7)
  - [ ] Stress test: fly through generated terrain, confirm stable frame time and bounded chunk entity count
- Done when: player can move through a procedurally generated world with chunks streaming in/out, stable FPS, no leaked entities (chunk count matches render-distance expectation after movement).

## Unknowns, risks, assumptions
- `bevy = "0.20.1"` pinned (per instruction); APIs referenced (`AsyncComputeTaskPool`, `Frustum`/`Aabb`, diagnostics) assumed stable at that version — verify against 0.20.1 docs/changelog at Phase 1 start, not from training knowledge.
- World generation (Phase 2) is synchronous in this plan; if generation cost becomes a bottleneck at larger render distances, it can move onto `AsyncComputeTaskPool` alongside meshing (Phase 4) — not included as a separate phase unless profiling shows it's needed.
- Occlusion culling (Phase 6) is the highest-risk phase for false negatives (pop-in); may need a tuning pass after Phase 8 stress test.
- No networking/multiplayer — single-process world only, per instruction.

