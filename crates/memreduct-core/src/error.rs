use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("I/O error accessing {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse field '{field}' in {path}: {detail}")]
    Parse {
        path: PathBuf,
        field: String,
        detail: String,
    },

    #[error("Missing required field in {path}: {field}")]
    MissingField { path: PathBuf, field: &'static str },

    #[error("Invalid reduction mode: {0}")]
    InvalidMode(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
