# PLAN: Minecraft textures for grass/dirt + Minecraft sky

Requests (deduped, dependency-ordered):
1. Atlas supports 64px tiles loaded from PNG files (foundation).
2. Dirt uses `assets/minecraft/textures/block/dirt.png`.
3. Grass block uses `grass_block_top.png`, `grass_block_side.png` (+ `grass_block_side_overlay.png`), dirt for bottom.
4. Sky is Minecraft light blue.

Context: atlas is procedural in [mesh.rs](src/mesh.rs) (`create_texture_atlas_image`, 4x4 tiles of 16px, nearest sampling set in [batching.rs](src/batching.rs)). Tile indices set in [registry.rs](src/registry.rs) (dirt = 2, grass side = 3, grass top = 4). Camera/light spawned in [main.rs](src/main.rs); no clear color set. Source PNGs are 64x64 (Faithful pack), indexed-color. `assets/` is already in the repo root.
Test command: `cargo nextest run --test-threads 1 --fail-fast`.

### Phase 1: 64px atlas built from PNG files [STATUS: DONE]
- Goal: atlas tiles are 64px and can be filled from a PNG on disk, with other blocks keeping their flat colors.
- Tasks:
  - [x] Set `TILE_PIXEL_SIZE` to 64 (atlas becomes 256x256); keep flat-color tiles filling the larger tile size
  - [x] Add a shared helper that decodes a PNG (path relative to `assets/`) to RGBA8 and blits it into a given tile index; return a typed error (`thiserror`) on missing file or wrong size, no `unwrap`
  - [x] Decide load mechanism (see risks): synchronous decode at startup so the atlas stays a single ready `Image`
  - [x] Update tests and doctests asserting 64x64 atlas size (`mesh.rs` doc, `lib.rs` atlas test) to 256x256
  - [x] Unit test: blitting a known PNG puts its first pixel at the tile's top-left atlas pixel
- Done when: build and all tests pass; game runs identically in layout (flat colors still shown, just a larger atlas).

### Phase 2: Dirt and grass textures [STATUS: DONE]
- Goal: dirt and grass blocks render with Minecraft textures.
- Tasks:
  - [x] Replace tile 2 with `dirt.png`
  - [x] Replace tile 4 (grass top) with `grass_block_top.png`
  - [x] Replace tile 3 (grass side) with `grass_block_side.png` composited with `grass_block_side_overlay.png` (alpha blend)
  - [x] Handle grass tint: top and overlay are grayscale in vanilla-style packs; verify, then multiply by a fixed Minecraft plains green (approx `#91BD59`) when blitting, not at runtime per-vertex
  - [x] Confirm `BlockType::Grass` face mapping in `registry.rs` still matches (sides 3, top 4, bottom 2 = dirt); no change expected
  - [x] Unit test: grass-top tile pixels are non-gray (tinted) and dirt tile equals `dirt.png` pixels
  - [x] Visual check: run game, inspect grass edges and dirt cliffs for correct orientation and no bleeding between tiles (nearest sampling already on)
- Done when: tests pass; in-game grass shows textured top/sides, dirt shows dirt texture.

### Phase 3: Minecraft sky color [STATUS: TODO]
- Goal: background sky is Minecraft's light blue.
- Tasks:
  - [ ] Add `ClearColor` resource in `main.rs` with Minecraft overworld sky `#78A7FF` (`rgb 120,167,255`), defined as a named constant
  - [ ] Optionally match fog later (out of scope unless asked)
- Done when: build passes; game window background is light blue.

## Unknowns, risks, assumptions
- Assumption: sky = plain `ClearColor` (no gradient, clouds, or fog). `#78A7FF` is the commonly cited vanilla daytime sky; tell me if you prefer another shade.
- Assumption: grass tint is baked as a fixed plains green; no per-biome coloring.
- Risk: runtime file loading via `AssetServer` is async and makes the atlas appear late; plan uses synchronous decode at startup instead. Needs the `png` decoder (Bevy's default `png` feature should cover it, or add the `image` crate) - verify in Phase 1.
- Risk: atlas grows 16x in memory (256x256 RGBA = 256 KB), negligible. Mipmaps are not generated, so distant textures may shimmer; out of scope.
- Risk: working dir must contain `assets/` at runtime (same as `cargo run` from repo root); WASM build in `TODO.md` would need embedding later.
