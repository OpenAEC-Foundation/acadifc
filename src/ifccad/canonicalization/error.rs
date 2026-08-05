use std::fmt;

/// Stable code identifying why an IFCCAD value cannot be canonicalized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalizationErrorCode {
    IntegerInvalid,
    FloatInvalid,
}

impl CanonicalizationErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IntegerInvalid => "VECTOR_INTEGER_INVALID",
            Self::FloatInvalid => "VECTOR_FLOAT_INVALID",
        }
    }
}

/// Failure to convert an IFCCAD value to its canonical representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalizationError {
    code: CanonicalizationErrorCode,
    message: &'static str,
}

impl CanonicalizationError {
    pub(crate) const fn new(code: CanonicalizationErrorCode, message: &'static str) -> Self {
        Self { code, message }
    }

    pub const fn code(&self) -> CanonicalizationErrorCode {
        self.code
    }
}

impl fmt::Display for CanonicalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for CanonicalizationError {}
