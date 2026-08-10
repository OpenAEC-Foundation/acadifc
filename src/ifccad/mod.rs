//! IFCCAD format foundations.
//!
//! IFCCAD uses IFCX for its semantic graph and resource references, one or
//! more IFCDR resources for drawing data, and optional IFCPR resources for
//! source-format preservation.

pub mod canonicalization;
pub mod conformance;
pub mod package;
