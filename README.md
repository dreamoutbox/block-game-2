# Block Game 2

A voxel block game written in Rust using the Bevy engine, targeting native desktop and WebAssembly (WASM).

## Features

- **Voxel Engine**: Chunk streaming, procedural terrain generation (noise-based dirt, grass, and stone), boundary face culling, and asynchronous chunk meshing.
- **Player Controller**: First-person 3D flying camera with AABB swept collision detection and toggleable noclip mode.
- **Block Editing**: Raycasting for block breaking and placing.
- **Hotbar & UI**: Hotbar HUD with block selection, in-game pause menu, and world management screens powered by `bevy_egui`.
- **World Persistence**: Save and load worlds and modified chunks using `bincode`.
- **Cross-Platform**: Runs natively on desktop and in modern web browsers via WebAssembly.

## Controls

| Action | Control |
| --- | --- |
| Move | `W` / `A` / `S` / `D` |
| Fly Up / Down | `Space` / `Left Shift` |
| Fast Fly (Speed Boost) | `Left Ctrl` |
| Toggle Collision (Noclip) | `C` |
| Look | Mouse (click window to lock cursor) |
| Break Block | Left Mouse Button |
| Place Block | Right Mouse Button |
| Select Hotbar Slot | Number keys `1` - `9` or Mouse Scroll Wheel |
| Pause / Menu / Release Cursor | `Escape` or `P` |

## Getting Started

### Prerequisites

- Rust 2024 edition (Rust 1.85+)
- For WebAssembly builds: `wasm32-unknown-unknown` target and `wasm-bindgen-cli`

### Running on Desktop

Run in release mode for optimal voxel meshing and frame rates:

```sh
cargo run --release
```

### Running in the Browser (WebAssembly)

Install the WASM target and wasm-bindgen CLI:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
```

Build the web bundle into `dist/` and start a local server:

```sh
./scripts/build-wasm.sh --serve
```

Then navigate to `http://localhost:8080`.

### Testing

Run the test suite using `cargo nextest`:

```sh
cargo nextest run --test-threads 1 --fail-fast
```

## Project Structure

- `src/` - Core engine, chunk streaming, voxel meshing, world generation, collision, save systems, and UI plugins.
- `assets/` - Essential texture assets (blocks and UI sprites).
- `scripts/` - Build scripts, including [build-wasm.sh](scripts/build-wasm.sh).
- `web/` - Web host template (`index.html`).
- `.github/workflows/` - CI/CD workflows, including GitHub Pages deployment.

