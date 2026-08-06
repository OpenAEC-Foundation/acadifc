use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PackageOpenError {
    #[error("IFCCAD package root is not a directory: {path}")]
    RootNotDirectory { path: PathBuf },

    #[error("failed to access IFCCAD package path {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
