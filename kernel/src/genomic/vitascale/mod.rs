//! VITASCALE Hostframe — biology-shaped host on iron chassis (ADR 0010).
//!
//! **Child name: KAIROS** — human-pronounceable, badass, native to this build
//! (`KairosState` already lives on chromosome brains: the right moment,
//! embodied as a host we raise from zygote → adulthood).
//!
//! Stage 0 (this module's first life): heartbeat (Pulsewire + VitalMeters),
//! genome/chromosome tissues may be *present* under Guardian lock — no free
//! agency (no train/prune/self-mod/real-VCF).
//!
//! Phase F L0 additions: Sensefield bus, NeurocyteHandle, Crown, PressureMesh,
//! Oculus, Phageguard, IronChassis.

pub mod pulsewire;
pub mod life_course;
pub mod guardian;
pub mod trajectory;
pub mod kairos;

// Phase F L0 — VITASCALE awareness and control layer.
pub mod awareness_bus;
pub mod organ_live;
pub mod nano_agent;
pub mod assembly;
pub mod fitness_federation;
pub mod oculus_organ;
pub mod immune_organ;
pub mod body;
pub mod self_awareness;

// Phase F A — Hostframe integration.
pub mod hostframe_bridge;

// Phase F B — Self-awareness telemetry expansion.
pub mod endocrine_model;
pub mod regime_detector;
pub mod emotion_model;

// Phase F C — Lifecycle transitions and self-modification.
pub mod adulthood_gate;
pub mod mutation_activation;

pub use pulsewire::{
    PulseEvent, Pulsewire, VitalMeters, PulseHandles, SRC_HEARTBEAT, SRC_ACTIVATE, SRC_SCORE,
    SRC_SELECT, SRC_LD, SRC_PHAGE, SRC_OCULUS, KIND_BEGIN, KIND_END, KIND_TICK, KIND_DROP,
};
pub use life_course::{
    LifeStage, StagePermissions, LifeCourse, StageGateResult, DevelopmentalJournalEntry,
};
pub use guardian::{
    Guardian, DisciplineEthos, BirthImprint, FIRST_WORDS, GUARDIAN_NAME, GUARDIAN_ROLE,
};
pub use trajectory::{TrajectoryCharter, TrajectoryPillar};
pub use kairos::{Kairos, KairosReport, NurseryGenomeSpec, NeonateCareReport};

// Phase F L0 re-exports.
pub use awareness_bus::{NeuroSignal, Sensefield, StreamId};
pub use organ_live::TissueLive;
pub use nano_agent::{
    CrownView, LocalGoal, MultiAxisFitness, NanoBrain, NeuroProposal, NanoStatus,
    NanoTickResult, NeurocyteHandle, OrganKey, QuarantineClass,
};
pub use assembly::{Crown, GestaltReport};
pub use fitness_federation::{FederationWeights, PressureHint, PressureMesh};
pub use oculus_organ::{AwarenessFrame, EyeStream, Oculus};
pub use immune_organ::{ImmuneConfig, PhageEvent, Phageguard};
pub use body::{BodyAdapter, BodyCommand, BodyError, BodyFrame, BodyHealth};
pub use body::host_cpu::IronChassis;
pub use self_awareness::{SelfAwarenessProbe, SenseReport};
pub use hostframe_bridge::{HostframeConfig, TelemetryClient, TelemetryPayload, TelemetryStats};
pub use adulthood_gate::{AdulthoodEvidence, AdulthoodGraduation};
pub use mutation_activation::{MutationAuthorization, MutationTracker, activate_on_graduation, is_mutation_allowed, log_mutation_to_ledger};
