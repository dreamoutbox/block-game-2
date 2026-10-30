#!/bin/sh
# Builds the game for the browser (WASM) into ./dist.
# Usage: scripts/build-wasm.sh [--debug] [--serve]
#   --debug  build the dev profile instead of release (faster compile, slower game)
#   --serve  serve ./dist on http://localhost:8080 after building
set -eu
cd "$(dirname "$0")/.."

PROFILE=release
CARGO_FLAGS="--release"
SERVE=0
for arg in "$@"; do
  case "$arg" in
    --debug) PROFILE=debug; CARGO_FLAGS="" ;;
    --serve) SERVE=1 ;;
    *) echo "unknown argument: $arg" >&2; exit 1 ;;
  esac
done

TARGET=wasm32-unknown-unknown
command -v wasm-bindgen >/dev/null 2>&1 || {
  echo "wasm-bindgen not found. Install: cargo install wasm-bindgen-cli" >&2
  exit 1
}
rustup target list --installed | grep -qx "$TARGET" || {
  echo "missing target. Install: rustup target add $TARGET" >&2
  exit 1
}

# shellcheck disable=SC2086
cargo build $CARGO_FLAGS --target "$TARGET"
wasm-bindgen --target web --no-typescript --out-dir dist/pkg \
  "target/$TARGET/$PROFILE/block-game-2.wasm"
cp web/index.html dist/index.html
rm -rf dist/assets
cp -r assets dist/assets
echo "Built ./dist"

if [ "$SERVE" = 1 ]; then
  echo "Serving http://localhost:8080"
  exec python3 -m http.server 8080 --directory dist
fi
