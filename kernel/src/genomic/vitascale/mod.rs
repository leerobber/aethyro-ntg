//! VITASCALE Hostframe — biology-shaped host on iron chassis.
//!
//! **Child name: KAIROS** — human-pronounceable, badass, native to this build
//! (`KairosState` already lives on chromosome brains: the right moment,
//! embodied as a host we raise from zygote → adulthood).
//!
//! Stage 0 (this module's first life): heartbeat (Pulsewire + VitalMeters),
//! genome/chromosome tissues may be *present* under Guardian lock — no free
//! agency (no train/prune/self-mod/real-VCF).

pub mod pulsewire;
pub mod life_course;
pub mod guardian;
pub mod trajectory;
pub mod kairos;

pub use pulsewire::{
    PulseEvent, Pulsewire, VitalMeters, PulseHandles, SRC_HEARTBEAT, SRC_ACTIVATE, SRC_SCORE,
    SRC_SELECT, SRC_LD, KIND_BEGIN, KIND_END, KIND_TICK, KIND_DROP,
};
pub use life_course::{
    LifeStage, StagePermissions, LifeCourse, StageGateResult, DevelopmentalJournalEntry,
};
pub use guardian::{
    Guardian, DisciplineEthos, BirthImprint, FIRST_WORDS, GUARDIAN_NAME, GUARDIAN_ROLE,
};
pub use trajectory::{TrajectoryCharter, TrajectoryPillar};
pub use kairos::{Kairos, KairosReport, NurseryGenomeSpec, NeonateCareReport};
