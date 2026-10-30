use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during texture atlas construction and tile blitting.
#[derive(Debug, Error)]
pub enum AtlasError {
    /// Failed to open or read an asset file.
    #[error("Failed to read texture file '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Failed to decode an image buffer.
    #[error("Failed to decode image '{path}': {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
    /// Image width or height does not match expected tile pixel size.
    #[error("Invalid image dimensions for '{path}': expected {expected_w}x{expected_h}, got {actual_w}x{actual_h}")]
    InvalidDimensions {
        path: PathBuf,
        expected_w: u32,
        expected_h: u32,
        actual_w: u32,
        actual_h: u32,
    },
    /// Tile index exceeds total capacity of atlas.
    #[error("Tile index {tile_index} is out of bounds (max {max_tiles})")]
    TileIndexOutOfBounds {
        tile_index: u32,
        max_tiles: u32,
    },
}
