use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NtgError {
    ShapeMismatch { expected: usize, got: usize },
    InvalidTernaryValue(i8),
    IndexOutOfBounds { index: usize, len: usize },
    EdgeNotFound { from: usize, to: usize },
    CycleDetected,
    // Phase 3 ledger errors
    LedgerTampering(String),
    InvalidInput(String),
    ChainBroken(usize), // index where chain broke
    // Native runtime / shape guards
    LayerNotFound { layer_idx: usize, layer_count: usize },
    /// Product of dimensions overflowed `usize` (allocation would OOM).
    DimensionOverflow { m: usize, k: usize, n: usize },
    /// Sequential layer node-id contract violated.
    NonSequentialNodeId { expected: usize, got: usize },
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
            NtgError::IndexOutOfBounds { index, len } => {
                write!(f, "index {index} out of bounds (len {len})")
            }
            NtgError::EdgeNotFound { from, to } => {
                write!(f, "no edge from {from} to {to}")
            }
            NtgError::CycleDetected => {
                write!(f, "cycle detected: graph has no valid topological order")
            }
            NtgError::LedgerTampering(msg) => {
                write!(f, "ledger tampering detected: {msg}")
            }
            NtgError::InvalidInput(msg) => {
                write!(f, "invalid input: {msg}")
            }
            NtgError::ChainBroken(idx) => {
                write!(f, "hash chain broken at entry {idx}")
            }
            NtgError::LayerNotFound {
                layer_idx,
                layer_count,
            } => {
                write!(
                    f,
                    "layer {layer_idx} not found (runtime has {layer_count} layer(s))"
                )
            }
            NtgError::DimensionOverflow { m, k, n } => {
                write!(f, "dimension overflow for matmul {m}x{k}x{n}")
            }
            NtgError::NonSequentialNodeId { expected, got } => {
                write!(
                    f,
                    "layer node IDs must be sequential starting at 0: expected id={expected}, got {got}"
                )
            }
        }
    }
}

impl std::error::Error for NtgError {}
