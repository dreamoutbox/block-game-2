use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during world creation, saving, or loading.
#[derive(Debug, Error)]
pub enum SaveError {
    /// The world name is invalid (contains invalid characters or path separators).
    #[error("Invalid world name '{0}': names cannot contain path separators, traversal elements, or be empty")]
    InvalidWorldName(String),

    /// A world with this name already exists.
    #[error("World '{0}' already exists")]
    WorldAlreadyExists(String),

    /// The requested world does not exist.
    #[error("World '{0}' does not exist")]
    WorldNotFound(String),

    /// File I/O error occurred.
    #[error("I/O error at '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Binary serialization or deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),
}
