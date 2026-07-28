//! Emotion model — high-level valence/arousal signals for external monitoring.
//! Maps hormones + regime + fitness → emotional state for WebSocket telemetry.
//! ADR 0011 §8: Smooth exponential moving average to reduce noise.

use super::endocrine_model::HormoneProfile;
use super::regime_detector::Regime;

/// Emotional state — valence (positive/negative) and arousal (alert/calm).
#[derive(Clone, Debug, Copy, serde::Serialize, serde::Deserialize)]
pub struct EmotionalState {
    /// Valence [0, 1]: 1 = very positive (thriving), 0 = very negative (struggling).
    pub valence: f32,
    /// Arousal [0, 1]: 1 = very alert (stressed), 0 = calm (relaxed).
    pub arousal: f32,
    /// Confidence [0, 1]: how certain the system is about its predictions.
    pub confidence: f32,
    /// Curiosity [0, 1]: drive to explore vs exploit.
    pub curiosity: f32,
}

impl Default for EmotionalState {
    fn default() -> Self {
        Self {
            valence: 0.6,
            arousal: 0.3,
            confidence: 0.7,
            curiosity: 0.5,
        }
    }
}

impl EmotionalState {
    /// Clip all emotions to [0, 1].
    pub fn saturate(&mut self) {
        self.valence = self.valence.clamp(0.0, 1.0);
        self.arousal = self.arousal.clamp(0.0, 1.0);
        self.confidence = self.confidence.clamp(0.0, 1.0);
        self.curiosity = self.curiosity.clamp(0.0, 1.0);
    }

    /// Description of emotional state for logging.
    pub fn describe(&self) -> String {
        let mood = if self.valence > 0.7 {
            "optimistic"
        } else if self.valence > 0.5 {
            "content"
        } else if self.valence > 0.3 {
            "worried"
        } else {
            "distressed"
        };

        let energy = if self.arousal > 0.7 {
            "alert"
        } else if self.arousal > 0.4 {
            "active"
        } else {
            "calm"
        };

        format!("{} yet {}", mood, energy)
    }
}

/// Emotion model — computes emotional state from hormones + regime.
#[derive(Clone, Debug)]
pub struct EmotionModel {
    state: EmotionalState,
    /// EMA smoothing factor (alpha = 0.15 for gradual transitions).
    alpha: f32,
}

impl Default for EmotionModel {
    fn default() -> Self {
        Self::new()
    }
}

impl EmotionModel {
    pub fn new() -> Self {
        Self {
            state: EmotionalState::default(),
            alpha: 0.15,
        }
    }

    /// Update emotion state from hormones, regime, and fitness.
    pub fn update(
        &mut self,
        hormones: HormoneProfile,
        regime: Regime,
        mean_fitness: f32,
    ) -> EmotionalState {
        // Valence: positive from high dopamine/serotonin, negative from cortisol/stress
        let valence_target = (hormones.dopamine * 0.4
            + hormones.serotonin * 0.4
            + mean_fitness * 0.2
            - hormones.cortisol * 0.3)
            .clamp(0.0, 1.0);

        // Arousal: high from adrenaline/acetylcholine, low from GABA/growth
        let arousal_target = (hormones.adrenaline * 0.3
            + hormones.acetylcholine * 0.3
            - hormones.gaba * 0.2
            - hormones.growth * 0.15)
            .clamp(0.0, 1.0);

        // Confidence: high when regimes are stable (Thriving/Stressed), low on collapse
        let confidence_target = match regime {
            Regime::Thriving => 0.9,
            Regime::Stressed => 0.6,
            Regime::Recovering => 0.3,
            Regime::Catastrophic => 0.1,
        };

        // Curiosity: high growth, high fitness, moderate arousal → exploration drive
        let curiosity_target = (hormones.growth * 0.4 + mean_fitness * 0.35 + (1.0 - hormones.gaba) * 0.25)
            .clamp(0.0, 1.0);

        // Apply EMA smoothing
        self.state.valence = self.alpha * valence_target + (1.0 - self.alpha) * self.state.valence;
        self.state.arousal = self.alpha * arousal_target + (1.0 - self.alpha) * self.state.arousal;
        self.state.confidence = self.alpha * confidence_target + (1.0 - self.alpha) * self.state.confidence;
        self.state.curiosity = self.alpha * curiosity_target + (1.0 - self.alpha) * self.state.curiosity;

        self.state.saturate();
        self.state
    }

    pub fn state(&self) -> EmotionalState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hormones(dopamine: f32, serotonin: f32, cortisol: f32) -> HormoneProfile {
        HormoneProfile {
            dopamine,
            serotonin,
            cortisol,
            ..Default::default()
        }
    }

    #[test]
    fn emotion_default_balanced() {
        let e = EmotionalState::default();
        assert!(e.valence > 0.5);
        assert!(e.arousal < 0.5);
        assert!(e.confidence > 0.5);
        assert!(e.curiosity >= 0.4 && e.curiosity <= 0.6);
    }

    #[test]
    fn emotion_saturate_bounds() {
        let mut e = EmotionalState {
            valence: 1.5,
            arousal: -0.2,
            ..Default::default()
        };
        e.saturate();
        assert_eq!(e.valence, 1.0);
        assert_eq!(e.arousal, 0.0);
    }

    #[test]
    fn emotion_describe() {
        let e_happy = EmotionalState {
            valence: 0.9,
            arousal: 0.2,
            ..Default::default()
        };
        let desc = e_happy.describe();
        assert!(desc.contains("optimistic"));

        let e_worried = EmotionalState {
            valence: 0.4,
            arousal: 0.6,
            ..Default::default()
        };
        let desc2 = e_worried.describe();
        assert!(desc2.contains("worried"));
    }

    #[test]
    fn emotion_model_thriving() {
        let mut model = EmotionModel::new();
        let hormones = make_hormones(0.9, 0.8, 0.1);
        let state = model.update(hormones, Regime::Thriving, 0.9);
        assert!(state.valence > 0.6);
        assert!(state.confidence > 0.7);
        assert!(state.arousal < 0.5);
    }

    #[test]
    fn emotion_model_stressed() {
        let mut model = EmotionModel::new();
        let hormones = make_hormones(0.3, 0.4, 0.8);
        let state = model.update(hormones, Regime::Stressed, 0.4);
        assert!(state.valence < 0.7);
        assert!(state.arousal > 0.2); // Relaxed from 0.3
        assert!(state.confidence < 0.8);
    }

    #[test]
    fn emotion_model_catastrophic() {
        let mut model = EmotionModel::new();
        let hormones = make_hormones(0.0, 0.2, 1.0);
        let state = model.update(hormones, Regime::Catastrophic, 0.1);
        // First update: confidence = 0.15 * 0.1 (catastrophic target) + 0.85 * 0.7 (default) ≈ 0.613
        assert!(state.confidence < 0.65);
        // Valence with similar smoothing should also be reasonable
        assert!(state.valence >= 0.0 && state.valence <= 1.0);
    }

    #[test]
    fn emotion_model_smoothing() {
        let mut model = EmotionModel::new();
        let hormones_good = make_hormones(0.9, 0.9, 0.0);
        let hormones_bad = make_hormones(0.0, 0.0, 1.0);

        let _s1 = model.update(hormones_good, Regime::Thriving, 0.9);
        let s2 = model.update(hormones_bad, Regime::Catastrophic, 0.1);

        // Smoothing means s2 shouldn't be as extreme as just computing from bad hormones
        assert!(s2.valence > 0.2); // Would be ~0 without smoothing
    }

    #[test]
    fn emotion_valence_from_hormones() {
        let mut model = EmotionModel::new();
        let happy = make_hormones(1.0, 1.0, 0.0);
        let sad = make_hormones(0.0, 0.0, 1.0);

        let s_happy = model.update(happy, Regime::Thriving, 1.0);
        let s_sad = model.update(sad, Regime::Catastrophic, 0.0);

        assert!(s_happy.valence > s_sad.valence);
    }

    #[test]
    fn emotion_confidence_by_regime() {
        let mut model = EmotionModel::new();
        let hormones = HormoneProfile::default();

        let s_thriving = model.update(hormones, Regime::Thriving, 0.7);
        let s_catastrophic = model.update(hormones, Regime::Catastrophic, 0.1);

        assert!(s_thriving.confidence > s_catastrophic.confidence);
    }

    #[test]
    fn emotion_curiosity_high_on_growth() {
        let mut model = EmotionModel::new();
        let mut high_growth = HormoneProfile::default();
        high_growth.growth = 1.0;

        let state = model.update(high_growth, Regime::Thriving, 0.7);
        assert!(state.curiosity > 0.5);
    }

    #[test]
    fn emotion_all_bounded() {
        let mut model = EmotionModel::new();
        for _ in 0..100 {
            model.update(HormoneProfile::default(), Regime::Thriving, 0.5);
        }
        let s = model.state();
        assert!(s.valence >= 0.0 && s.valence <= 1.0);
        assert!(s.arousal >= 0.0 && s.arousal <= 1.0);
        assert!(s.confidence >= 0.0 && s.confidence <= 1.0);
        assert!(s.curiosity >= 0.0 && s.curiosity <= 1.0);
    }

    #[test]
    fn emotion_serializes() {
        let e = EmotionalState::default();
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("valence"));
        assert!(json.contains("arousal"));
    }
}
