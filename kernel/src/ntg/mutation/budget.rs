//! Cycle-accurate budget enforcement for mutation cycles.
//!
//! ADR 0002 rule 2: "Bounded compute/time budget per modification cycle.
//! Each cycle runs within a hard, configurable ceiling."
//!
//! This module tracks wall-clock time and enforces hard limits to prevent
//! runaway mutation evaluation.

use super::super::error::NtgError;
use std::time::Instant;

pub struct BudgetTracker {
    /// Budget in microseconds
    budget_us: u64,
    /// Start of the cycle (when BudgetTracker was created)
    cycle_start: Instant,
    /// Total consumed so far (in microseconds)
    consumed_us: u64,
}

impl BudgetTracker {
    pub fn new(budget_us: u64) -> Self {
        Self {
            budget_us,
            cycle_start: Instant::now(),
            consumed_us: 0,
        }
    }

    /// Return current wall-clock time in nanoseconds (since cycle_start).
    pub fn wall_time_ns(&self) -> u64 {
        self.cycle_start.elapsed().as_nanos() as u64
    }

    /// Consume some budget. Fails if consumption exceeds the limit.
    pub fn consume_us(&mut self, us: u64) -> Result<(), NtgError> {
        self.consumed_us += us;
        if self.consumed_us > self.budget_us {
            return Err(NtgError::InvalidInput(format!(
                "Budget exceeded: {} us consumed > {} us limit",
                self.consumed_us, self.budget_us
            )));
        }
        Ok(())
    }

    /// Check if still within budget (non-failing check).
    pub fn within_budget(&self) -> bool {
        self.consumed_us <= self.budget_us
    }

    /// Remaining budget in microseconds.
    pub fn remaining_us(&self) -> u64 {
        self.budget_us.saturating_sub(self.consumed_us)
    }

    /// Status: (consumed_us, remaining_us, budget_us).
    pub fn status(&self) -> (u64, u64, u64) {
        (self.consumed_us, self.remaining_us(), self.budget_us)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn new_budget_starts_at_zero_consumed() {
        let tracker = BudgetTracker::new(1_000_000);
        assert_eq!(tracker.consumed_us, 0);
        assert_eq!(tracker.budget_us, 1_000_000);
    }

    #[test]
    fn consume_budget() -> Result<(), NtgError> {
        let mut tracker = BudgetTracker::new(1_000_000);
        tracker.consume_us(100_000)?;
        assert_eq!(tracker.consumed_us, 100_000);
        assert_eq!(tracker.remaining_us(), 900_000);
        Ok(())
    }

    #[test]
    fn exceeding_budget_fails() {
        let mut tracker = BudgetTracker::new(100_000);
        assert!(tracker.consume_us(50_000).is_ok());
        assert!(tracker.consume_us(50_000).is_ok());
        assert!(tracker.consume_us(1).is_err()); // Exceeds budget
    }

    #[test]
    fn within_budget_check() -> Result<(), NtgError> {
        let mut tracker = BudgetTracker::new(1_000_000);
        assert!(tracker.within_budget());
        tracker.consume_us(500_000)?;
        assert!(tracker.within_budget());
        Ok(())
    }

    #[test]
    fn wall_time_ns_increases() {
        let tracker = BudgetTracker::new(1_000_000);
        let t1 = tracker.wall_time_ns();
        sleep(Duration::from_micros(100));
        let t2 = tracker.wall_time_ns();
        assert!(t2 > t1);
    }

    #[test]
    fn status_returns_tuple() -> Result<(), NtgError> {
        let mut tracker = BudgetTracker::new(1_000_000);
        tracker.consume_us(200_000)?;
        let (consumed, remaining, budget) = tracker.status();
        assert_eq!(consumed, 200_000);
        assert_eq!(remaining, 800_000);
        assert_eq!(budget, 1_000_000);
        Ok(())
    }

    #[test]
    fn remaining_is_zero_when_exhausted() -> Result<(), NtgError> {
        let mut tracker = BudgetTracker::new(100_000);
        tracker.consume_us(100_000)?;
        assert_eq!(tracker.remaining_us(), 0);
        Ok(())
    }
}
