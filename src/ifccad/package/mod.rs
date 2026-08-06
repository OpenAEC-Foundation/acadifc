mod codes;
mod diagnostic;

pub use diagnostic::{
    PackageDiagnostic, PackageDiagnosticContextValue, PackageDiagnosticSeverity,
    PackageValidationReport,
};

/// Fixed IFCX entrypoint inside a directory-based IFCCAD package.
pub const PACKAGE_ENTRYPOINT: &str = "package.ifcx.json";
