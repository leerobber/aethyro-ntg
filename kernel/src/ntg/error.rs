use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NtgError {
    ShapeMismatch { expected: usize, got: usize },
    InvalidTernaryValue(i8),
}

impl fmt::Display for NtgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NtgError::ShapeMismatch { expected, got } => {
                write!(f, "shape mismatch: expected {expected} elements, got {got}")
            }
            NtgError::InvalidTernaryValue(v) => {
                write!(f, "invalid ternary value: {v} (must be -1, 0, or 1)")
            }
        }
    }
}

impl std::error::Error for NtgError {}
