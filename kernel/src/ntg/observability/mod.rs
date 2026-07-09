//! Lock-free observability for native / FFI hot paths.
//!
//! Pure Rust — no WASM/GUI deps. Snapshots feed dashboards, ledger
//! annotations, and (later) mutation fitness gates.
//!
//! **Not** a product UI. WASM / Three.js / avatar layers are out of scope
//! here (see docs/architecture/0007).

pub mod stats;

pub use stats::{StatsCollector, StatsSnapshot};
