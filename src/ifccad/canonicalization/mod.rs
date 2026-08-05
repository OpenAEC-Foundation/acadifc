//! Canonical byte representation for language-neutral IFCCAD values.

mod error;
mod value;

pub use error::{CanonicalizationError, CanonicalizationErrorCode};
pub use value::{canonicalize, CanonicalValue};
