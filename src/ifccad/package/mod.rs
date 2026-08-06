//! Directory-based IFCCAD package foundations.
//!
//! Complete package validation is added separately; this module currently
//! exposes its stable diagnostic vocabulary and entrypoint name.

mod codes;
mod diagnostic;
mod error;
// Composed by the public validator in the follow-up package-validation change.
#[allow(dead_code)]
mod loader;
mod path;

pub use diagnostic::{
    PackageDiagnostic, PackageDiagnosticContextValue, PackageDiagnosticSeverity,
    PackageValidationReport,
};
pub use error::PackageOpenError;

/// Fixed IFCX entrypoint inside a directory-based IFCCAD package.
pub const PACKAGE_ENTRYPOINT: &str = "package.ifcx.json";
