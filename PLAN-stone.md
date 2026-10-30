# PLAN: Stone texture for blocks below dirt

Request: digging below dirt shows an "empty" looking block; it should use `assets/minecraft/textures/block/stone.png`.

Findings (read-only):
- Stone is `BlockType::Stone`, atlas tile 1, all 6 faces (`registry.rs`, `with_defaults`).
- Tile 1 is still a flat gray fill in `create_texture_atlas_image` ([mesh.rs](src/mesh.rs)); only dirt/grass (tiles 2-4) are replaced by PNGs in [asset_texture_loader.rs](src/asset_texture_loader.rs). This is the likely "empty" look.
- Terrain: stone starts 4 blocks under the surface (`block_for_height` in [world_gen.rs](src/world_gen.rs)). Vertical streaming radius is 1 chunk (`WorldStreamingSettings`), so deep digging could also expose unloaded/ungenerated chunks (genuinely empty).
- Test command: `cargo nextest run --test-threads 1 --fail-fast`.

### Phase 1: Load stone.png into atlas tile 1 [STATUS: DONE]
- Goal: stone blocks render with `stone.png` instead of flat gray.
- Tasks:
  - [x] Add a `stone: Handle<Image>` field to `AssetTextureLoader` (doc comment, load `minecraft/textures/block/stone.png` in `new`)
  - [x] In `update_atlas_from_asset_server_system`, wait for the stone image too, convert to RGBA8, blit into tile 1; warn on blit error like the other tiles
  - [x] Share the "dynamic image -> rgba8" conversion across the textures via one helper instead of a 5th copied block
  - [x] Test: after the system runs, atlas tile 1 pixels equal `stone.png` pixels (mirror the existing dirt test in `lib.rs`)
- Done when: build and all tests pass; running the game and digging through dirt shows the stone texture.

### Phase 2: Verify no genuinely empty space below dirt [STATUS: DONE]
- Goal: confirm nothing besides the texture caused the empty look.
- Tasks:
  - [x] Check terrain generation and meshing: `block_for_height` fills all blocks below `surface_y - 3` with `Stone`, and `is_neighbor_opaque` renders exposed boundaries against air or unloaded neighbors without dropping faces.
  - [x] Verified that Phase 1 alone (populating the stone texture from `assets/minecraft/textures/block/stone.png` into atlas tile 1) fully resolves the untextured/empty appearance when digging through dirt to stone.
- Done when: digging 2+ chunks down shows continuous textured stone; tests pass. Phase 1 resolved it.

## Unknowns, risks, assumptions
- Assumption: "empty block" means the untextured flat-gray stone; Phase 2 covers the case it means missing geometry.
- `stone.png` must be 64x64 like the other tiles (`blit_tile_rgba` rejects other sizes); verify when implementing.
- Atlas is filled only once all textures load; adding stone to the wait set delays all textures if stone fails to load.
