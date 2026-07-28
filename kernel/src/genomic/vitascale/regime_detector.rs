//! Regime detector — online classification of system state (thriving/stressing/catastrophic).
//! ADR 0011 §7: Threshold-based classification with hysteresis to prevent chatter.

/// System regime classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Regime {
    /// High fitness, low stress, stable
    Thriving,
    /// Moderate fitness, elevated stress, active recovery
    Stressed,
    /// Low fitness, high stress, critical condition
    Recovering,
    /// Catastrophic: complete breakdown
    Catastrophic,
}

impl Regime {
    pub fn as_str(&self) -> &'static str {
        match self {
            Regime::Thriving => "Thriving",
            Regime::Stressed => "Stressed",
            Regime::Recovering => "Recovering",
            Regime::Catastrophic => "Catastrophic",
        }
    }
}

/// Regime detector — simple threshold-based state classification with hysteresis.
pub struct RegimeDetector {
    current_regime: Regime,
    ticks_in_regime: u32,
    /// Minimum ticks in a regime before transition allowed (hysteresis).
    hysteresis_ticks: u32,
}

impl RegimeDetector {
    pub fn new() -> Self {
        Self {
            current_regime: Regime::Thriving,
            ticks_in_regime: 0,
            hysteresis_ticks: 5,
        }
    }

    /// Classify based on fitness, stress, drop rate; apply hysteresis.
    pub fn update(
        &mut self,
        mean_fitness: f32,
        quarantine_fraction: f32,
        bus_drop_rate: f32,
        stress: f32,
    ) -> Regime {
        let predicted = self.classify(mean_fitness, quarantine_fraction, bus_drop_rate, stress);

        if predicted == self.current_regime {
            // In the predicted regime; increment counter
            self.ticks_in_regime += 1;
        } else {
            // Predicted regime differs from current
            if self.ticks_in_regime > 0 {
                self.ticks_in_regime -= 1; // Decay hysteresis
            } else {
                // Hysteresis expired; transition
                self.current_regime = predicted;
                self.ticks_in_regime = self.hysteresis_ticks;
            }
        }

        self.current_regime
    }

    /// Direct classification without hysteresis.
    fn classify(
        &self,
        fitness: f32,
        quarantine: f32,
        drop_rate: f32,
        stress: f32,
    ) -> Regime {
        if fitness < 0.2 && stress > 0.7 {
            Regime::Catastrophic
        } else if fitness < 0.4 || drop_rate > 0.5 || quarantine > 0.6 {
            Regime::Recovering
        } else if fitness > 0.75 && stress < 0.3 {
            Regime::Thriving
        } else {
            Regime::Stressed
        }
    }

    pub fn current_regime(&self) -> Regime {
        self.current_regime
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regime_thriving_on_high_fitness() {
        let mut detector = RegimeDetector::new();
        for _ in 0..10 {
            detector.update(0.9, 0.0, 0.01, 0.1);
        }
        assert_eq!(detector.current_regime(), Regime::Thriving);
    }

    #[test]
    fn regime_stressed_on_moderate_fitness() {
        let mut detector = RegimeDetector::new();
        for _ in 0..10 {
            detector.update(0.55, 0.2, 0.1, 0.4);
        }
        assert_eq!(detector.current_regime(), Regime::Stressed);
    }

    #[test]
    fn regime_recovering_on_low_fitness() {
        let mut detector = RegimeDetector::new();
        for _ in 0..10 {
            detector.update(0.3, 0.6, 0.3, 0.8);
        }
        assert_eq!(detector.current_regime(), Regime::Recovering);
    }

    #[test]
    fn regime_catastrophic_on_collapse() {
        let mut detector = RegimeDetector::new();
        for _ in 0..10 {
            detector.update(0.15, 0.9, 0.8, 1.0);
        }
        assert_eq!(detector.current_regime(), Regime::Catastrophic);
    }

    #[test]
    fn hysteresis_prevents_downward_bounce() {
        let mut detector = RegimeDetector::new();
        // Bootstrap as thriving
        for _ in 0..10 {
            detector.update(0.9, 0.0, 0.0, 0.1);
        }
        assert_eq!(detector.current_regime(), Regime::Thriving);

        // Single stressed sample shouldn't change regime (hysteresis)
        detector.update(0.5, 0.3, 0.2, 0.5);
        assert_eq!(detector.current_regime(), Regime::Thriving);
    }

    #[test]
    fn recovery_from_stressed() {
        let mut detector = RegimeDetector::new();
        // Bootstrap as stressed
        for _ in 0..10 {
            detector.update(0.6, 0.2, 0.1, 0.4);
        }
        assert_eq!(detector.current_regime(), Regime::Stressed);

        // Improve over several ticks
        for _ in 0..15 {
            detector.update(0.85, 0.05, 0.02, 0.2);
        }
        // Should eventually reach Thriving (after hysteresis delay)
        assert_eq!(detector.current_regime(), Regime::Thriving);
    }

    #[test]
    fn regime_as_str() {
        assert_eq!(Regime::Thriving.as_str(), "Thriving");
        assert_eq!(Regime::Stressed.as_str(), "Stressed");
        assert_eq!(Regime::Recovering.as_str(), "Recovering");
        assert_eq!(Regime::Catastrophic.as_str(), "Catastrophic");
    }

    #[test]
    fn classification_on_drop_rate() {
        let detector = RegimeDetector::new();
        let predicted = detector.classify(0.8, 0.1, 0.6, 0.2); // High drop rate
        assert_eq!(predicted, Regime::Recovering);
    }

    #[test]
    fn classification_on_quarantine() {
        let detector = RegimeDetector::new();
        let predicted = detector.classify(0.7, 0.7, 0.1, 0.2); // High quarantine
        assert_eq!(predicted, Regime::Recovering);
    }
}
