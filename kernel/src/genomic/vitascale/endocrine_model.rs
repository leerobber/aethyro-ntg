//! Endocrine model — biologically plausible hormone dynamics.
//! Hormones drive behavioral modulation: learning rate, exploration bias, recovery speed.
//! ADR 0011 §6: Hormone profile with 8 dimensions.

/// Hormone profile — 8 biologically plausible dimensions [0, 1] each.
#[derive(Clone, Debug, Copy)]
pub struct HormoneProfile {
    /// Growth hormone — peaks during learning phase, decays with age.
    pub growth: f32,
    /// Cortisol (stress) — rises on safety failures, decays exponentially.
    pub cortisol: f32,
    /// Adrenaline (alarm) — immediate response to danger, very fast decay.
    pub adrenaline: f32,
    /// Dopamine (reward) — reinforces successful actions.
    pub dopamine: f32,
    /// Serotonin (mood) — regulates emotional valence, decays slowly.
    pub serotonin: f32,
    /// Acetylcholine (attention) — peaks during high-workload periods.
    pub acetylcholine: f32,
    /// GABA (inhibition) — promotes safety and risk-aversion.
    pub gaba: f32,
    /// Oxytocin (social/trust) — bonds agent to trajectory/guardian.
    pub oxytocin: f32,
}

impl Default for HormoneProfile {
    fn default() -> Self {
        Self {
            growth: 0.6,      // High at birth
            cortisol: 0.1,    // Low baseline
            adrenaline: 0.0,  // No alarm at rest
            dopamine: 0.5,    // Neutral reward
            serotonin: 0.7,   // Good baseline mood
            acetylcholine: 0.3, // Low attention at rest
            gaba: 0.5,        // Balanced safety
            oxytocin: 0.8,    // High trust in guardian
        }
    }
}

impl HormoneProfile {
    /// Clip all hormones to [0, 1].
    pub fn saturate(&mut self) {
        self.growth = self.growth.clamp(0.0, 1.0);
        self.cortisol = self.cortisol.clamp(0.0, 1.0);
        self.adrenaline = self.adrenaline.clamp(0.0, 1.0);
        self.dopamine = self.dopamine.clamp(0.0, 1.0);
        self.serotonin = self.serotonin.clamp(0.0, 1.0);
        self.acetylcholine = self.acetylcholine.clamp(0.0, 1.0);
        self.gaba = self.gaba.clamp(0.0, 1.0);
        self.oxytocin = self.oxytocin.clamp(0.0, 1.0);
    }

    /// Stress level [0, 1]: composite of cortisol, adrenaline, low serotonin.
    pub fn stress_level(&self) -> f32 {
        (0.4 * self.cortisol + 0.4 * self.adrenaline + 0.2 * (1.0 - self.serotonin)).clamp(0.0, 1.0)
    }

    /// Recovery urgency [0, 1]: driven by high stress and low dopamine.
    pub fn recovery_urgency(&self) -> f32 {
        (0.6 * self.stress_level() + 0.4 * (1.0 - self.dopamine)).clamp(0.0, 1.0)
    }
}

/// Endocrine system — updates hormones based on fitness and system state.
#[derive(Clone, Debug)]
pub struct EndocrineModel {
    profile: HormoneProfile,
    /// Running mean fitness (EMA with alpha=0.1).
    fitness_ema: f32,
    /// Tick counter (used for age-dependent decay of growth).
    tick: u64,
}

impl Default for EndocrineModel {
    fn default() -> Self {
        Self::new()
    }
}

impl EndocrineModel {
    pub fn new() -> Self {
        Self {
            profile: HormoneProfile::default(),
            fitness_ema: 0.5,
            tick: 0,
        }
    }

    /// Update hormone profile based on fitness, quarantine fraction, and bus state.
    pub fn tick(
        &mut self,
        mean_fitness: f32,
        quarantine_fraction: f32,
        bus_drop_rate: f32,
    ) -> HormoneProfile {
        self.tick += 1;

        // Update EMA of fitness
        let alpha = 0.15;
        self.fitness_ema = alpha * mean_fitness + (1.0 - alpha) * self.fitness_ema;

        // Growth: decay over time with slower rate, boost on high fitness
        let age_decay = (-(self.tick as f32) / 500_000.0).exp(); // Decay much slower
        self.profile.growth = 0.2 * age_decay + 0.8 * self.fitness_ema.max(0.3);

        // Cortisol: rises on quarantine, decays exponentially
        let quarantine_pressure = quarantine_fraction * 1.0;
        self.profile.cortisol = 0.4 * quarantine_pressure + 0.6 * self.profile.cortisol * 0.92;

        // Adrenaline: immediate spike on drops, very fast decay
        if bus_drop_rate > 0.1 {
            self.profile.adrenaline = bus_drop_rate.min(1.0);
        } else {
            self.profile.adrenaline *= 0.4; // Fast decay
        }

        // Dopamine: reward on high fitness, penalty on drops
        let fitness_reward = self.fitness_ema * 0.8;
        let drop_penalty = bus_drop_rate * 0.3;
        self.profile.dopamine = 0.7 * (fitness_reward - drop_penalty).clamp(0.0, 1.0) + 0.3 * self.profile.dopamine;

        // Serotonin: tracks overall health, slower update
        let health = (self.fitness_ema * 0.6 + (1.0 - quarantine_fraction) * 0.4).clamp(0.0, 1.0);
        self.profile.serotonin = 0.4 * health + 0.6 * self.profile.serotonin;

        // Acetylcholine: peaks on high load (drops or high cortisol), decays
        let workload = bus_drop_rate + quarantine_fraction;
        self.profile.acetylcholine = 0.6 * workload + 0.4 * self.profile.acetylcholine * 0.9;

        // GABA: rises when we need safety (high stress, low fitness)
        let safety_need = self.profile.stress_level() * (1.0 - self.fitness_ema);
        self.profile.gaba = 0.4 * (0.5 + safety_need) + 0.6 * self.profile.gaba;

        // Oxytocin: stable but sensitive to sustained health; bonding signal
        if self.fitness_ema > 0.7 {
            self.profile.oxytocin = 0.9; // Strong bond when thriving
        } else if self.fitness_ema < 0.3 {
            self.profile.oxytocin = 0.4; // Weakened bond under stress
        }
        // Else drift towards equilibrium
        self.profile.oxytocin = 0.7 * self.profile.oxytocin + 0.3 * 0.6;

        self.profile.saturate();
        self.profile
    }

    pub fn profile(&self) -> HormoneProfile {
        self.profile
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_is_balanced() {
        let p = HormoneProfile::default();
        assert!(p.growth >= 0.0 && p.growth <= 1.0);
        assert!(p.cortisol >= 0.0 && p.cortisol <= 1.0);
        assert_eq!(p.adrenaline, 0.0);
        assert!(p.stress_level() < 0.5); // Should be low at default
    }

    #[test]
    fn saturate_clips_bounds() {
        let mut p = HormoneProfile {
            growth: 1.5,
            cortisol: -0.2,
            ..Default::default()
        };
        p.saturate();
        assert_eq!(p.growth, 1.0);
        assert_eq!(p.cortisol, 0.0);
    }

    #[test]
    fn stress_level_composite() {
        let p = HormoneProfile {
            cortisol: 1.0,
            adrenaline: 1.0,
            serotonin: 0.0,
            ..Default::default()
        };
        let stress = p.stress_level();
        assert!(stress > 0.9);
        assert!(stress <= 1.0);
    }

    #[test]
    fn stress_level_low_when_balanced() {
        let p = HormoneProfile::default();
        assert!(p.stress_level() < 0.4);
    }

    #[test]
    fn recovery_urgency_high_under_stress() {
        let p = HormoneProfile {
            cortisol: 1.0,
            dopamine: 0.0,
            serotonin: 0.2,
            ..Default::default()
        };
        assert!(p.recovery_urgency() > 0.7);
    }

    #[test]
    fn endocrine_tick_healthy_state() {
        let mut endo = EndocrineModel::new();
        let p = endo.tick(0.9, 0.0, 0.0); // High fitness, no quarantine, no drops
        assert!(p.dopamine > 0.4);
        assert!(p.serotonin > 0.5);
        assert!(p.stress_level() < 0.4);
    }

    #[test]
    fn endocrine_tick_stress_response() {
        let mut endo = EndocrineModel::new();
        let p = endo.tick(0.2, 0.5, 0.4); // Low fitness, high quarantine, high drops
        assert!(p.cortisol > 0.1);
        assert!(p.adrenaline > 0.2);
        assert!(p.stress_level() > 0.3);
    }

    #[test]
    fn endocrine_tick_adrenaline_decay() {
        let mut endo = EndocrineModel::new();
        let p1 = endo.tick(0.5, 0.3, 0.5); // High drop rate
        assert!(p1.adrenaline > 0.3);

        let p2 = endo.tick(0.5, 0.0, 0.0); // Drop rate cleared
        assert!(p2.adrenaline < p1.adrenaline); // Should decay fast
    }

    #[test]
    fn endocrine_tick_growth_responsive() {
        let mut endo = EndocrineModel::new();
        let p1 = endo.tick(0.9, 0.0, 0.0);
        let p_high_fitness = p1.growth;

        // Test low fitness
        let p2 = endo.tick(0.1, 0.8, 0.5);
        let p_low_fitness = p2.growth;

        // Growth should be lower when fitness is low
        assert!(p_low_fitness < p_high_fitness);
    }

    #[test]
    fn endocrine_tick_gaba_rises_under_threat() {
        let mut endo = EndocrineModel::new();
        let p = endo.tick(0.1, 0.8, 0.5); // Low fitness, high stress
        assert!(p.gaba > 0.5); // Safety-seeking behavior
    }

    #[test]
    fn endocrine_tick_oxytocin_responds_to_fitness() {
        let mut endo = EndocrineModel::new();
        endo.tick(0.8, 0.0, 0.0); // High fitness
        let p_high = endo.profile();
        assert!(p_high.oxytocin > 0.7); // Strong bond when thriving

        endo.tick(0.2, 0.7, 0.5); // Low fitness, high stress
        let p_low = endo.profile();
        assert!(p_low.oxytocin < 0.7); // Weakened bond under stress
    }

    #[test]
    fn all_hormones_bounded() {
        let mut endo = EndocrineModel::new();
        // Extreme states
        for _ in 0..100 {
            endo.tick(1.0, 1.0, 1.0); // Maximum stress
            endo.tick(0.0, 0.0, 0.0); // Minimum stress
        }
        let p = endo.profile();
        assert!(p.growth >= 0.0 && p.growth <= 1.0);
        assert!(p.cortisol >= 0.0 && p.cortisol <= 1.0);
        assert!(p.adrenaline >= 0.0 && p.adrenaline <= 1.0);
        assert!(p.dopamine >= 0.0 && p.dopamine <= 1.0);
        assert!(p.serotonin >= 0.0 && p.serotonin <= 1.0);
        assert!(p.acetylcholine >= 0.0 && p.acetylcholine <= 1.0);
        assert!(p.gaba >= 0.0 && p.gaba <= 1.0);
        assert!(p.oxytocin >= 0.0 && p.oxytocin <= 1.0);
    }

    #[test]
    fn recovery_urgency_responds_correctly() {
        let p_stressed = HormoneProfile {
            cortisol: 0.9,
            dopamine: 0.2,
            serotonin: 0.1,
            ..Default::default()
        };
        let p_healthy = HormoneProfile::default();
        assert!(p_stressed.recovery_urgency() > p_healthy.recovery_urgency());
    }
}
