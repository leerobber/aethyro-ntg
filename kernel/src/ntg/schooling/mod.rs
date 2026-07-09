//! Doctorate-style schooling: real datasets, study, advanced exams, 75% gate.
//!
//! See `docs/schooling/` for curriculum notebooks and run logs.

pub mod data;
pub mod notebook;
pub mod phases;
pub mod protocol;
pub mod runner;

pub use protocol::{PASS_THRESHOLD, DEFAULT_CAMPAIGN_RUNS, DEFAULT_MAX_ATTEMPTS};
pub use runner::{run_campaign, run_schooling_program, SchoolConfig};
