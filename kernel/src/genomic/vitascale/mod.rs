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
pub mod phageguard;
pub mod adult_course;
pub mod human_growth_course;
pub mod world_knowledge_course;
pub mod kairos;
pub mod cradle;
pub mod talk;
pub mod gh05t3_bridge;
pub mod lab;

pub use pulsewire::{
    PulseEvent, Pulsewire, VitalMeters, PulseHandles, SRC_HEARTBEAT, SRC_ACTIVATE, SRC_SCORE,
    SRC_SELECT, SRC_LD, KIND_BEGIN, KIND_END, KIND_TICK, KIND_DROP,
};
pub use life_course::{
    LifeStage, StagePermissions, LifeCourse, StageGateResult, DevelopmentalJournalEntry,
};
pub use guardian::{
    Guardian, DisciplineEthos, BirthImprint, GuardianAward, FIRST_WORDS, GUARDIAN_NAME,
    GUARDIAN_ROLE,
};
pub use trajectory::{TrajectoryCharter, TrajectoryPillar};
pub use phageguard::{
    Phageguard, PhageEvent, ImmuneConfig, DetectorClass, QuarantineClass, ThreatKind,
};
pub use adult_course::{AdultModule, adult_course_catalog, module_count as adult_module_count};
pub use human_growth_course::{
    GrowthModule, human_growth_catalog, growth_module_count, growth_module_at,
};
pub use world_knowledge_course::{
    WorldModule, world_knowledge_catalog, world_module_count, world_module_at,
};
pub use kairos::{
    Kairos, KairosReport, NurseryGenomeSpec, NeonateCareReport, InfantCareReport,
    ToddlerCareReport, ChildSchoolReport, AdolescentCampaignReport, YoungAdultDayReport,
    AdultCourseReport, SexEdCourseReport, WorldCourseReport, ADOLESCENT_MAX_VARIANTS,
    ADOLESCENT_DEFAULT_CHR, ADOLESCENT_SELECTION_STEPS,
};
pub use cradle::{
    CradleOpenReport, default_cradle_dir, open_or_birth, save_cradle, load_cradle,
};
pub use talk::{talk_once, write_avatar_ui, load_talk_history, TalkReply, TalkTurn};
pub use gh05t3_bridge::{bridge_status, try_remote_kairos_reply, ReplySource};
pub use lab::{
    default_lab_dir, default_primary_dir, fork_lab_from_primary, open_or_fork_lab,
    run_lab_science_day, run_lab_science_day_cfg, promote_lab_weights_to_primary,
    reaffirm_love_note, LabDayReport, LabDayConfig,
};
pub use lab::hypothesis::{Hypothesis, HypothesisRegistry, HypothesisVerdict};
pub use lab::science::{
    evaluate_self_mod_readiness, run_combined_heal_crisis, run_goal_improve_campaign,
    run_multi_heal_pack, run_self_heal_drill, run_self_improve_campaign, run_single_self_mod_lab,
    run_ternary_ladder, run_ternary_popcount_experiment, run_ternary_popcount_real_or_synthetic,
    CombinedHealReport, GoalImproveReport, MultiHealPackReport, SelfHealReport, SelfImproveReport,
    SelfModReadiness, SelfModRunReport, TernaryLadderReport, TernaryPopcountReport,
};
