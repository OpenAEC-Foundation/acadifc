mod codes;
mod diagnostic;
mod error;
mod path;

pub use diagnostic::{
    PackageDiagnostic, PackageDiagnosticContextValue, PackageDiagnosticSeverity,
    PackageValidationReport,
};
pub use error::PackageOpenError;

/// Fixed IFCX entrypoint inside a directory-based IFCCAD package.
pub const PACKAGE_ENTRYPOINT: &str = "package.ifcx.json";
