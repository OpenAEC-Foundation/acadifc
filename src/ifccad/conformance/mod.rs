//! Language-neutral IFCCAD conformance support.

mod error;
mod manifest;

use std::path::PathBuf;

pub use error::ConformanceError;
pub use manifest::{
    parse_conformance_manifest, ConformanceCase, ConformanceCategory, ConformanceManifest,
    ConformanceOperation, ConformanceOperationName, ExpectedOutcome,
};

/// Version of the conformance test collection bundled with this crate.
pub const BUNDLED_CONFORMANCE_VERSION: &str = "1.0.0";

/// Repository or crate-package path containing the bundled test collection.
pub fn bundled_conformance_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance")
        .join("ifccad")
        .join(BUNDLED_CONFORMANCE_VERSION)
}
