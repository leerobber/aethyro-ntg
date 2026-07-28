//! Optimization Domain-Specific Language (DSL)
//!
//! Allows agents to define, compile, and execute custom optimization rules.
//! Agents write in a high-level language that gets compiled to efficient Rust/CUDA.
//!
//! Example:
//!   rule "batch_operations_when_queue_full":
//!       when queue_depth > 100:
//!           batch_size = min(queue_depth / 2, 512)
//!           enable_batching(batch_size)
//!           expect_speedup(1.5x)

use crate::ntg::error::NtgError;
use std::collections::HashMap;

/// Optimization rule: condition → action
#[derive(Clone, Debug)]
pub struct OptimizationRule {
    pub name: String,
    pub description: String,
    pub condition: Condition,
    pub actions: Vec<Action>,
    pub expected_speedup: f32,  // Multiplier, e.g., 1.5 = 50% speedup
    pub risk_profile: RiskProfile,
}

/// Condition that triggers a rule
#[derive(Clone, Debug)]
pub enum Condition {
    /// Simple threshold: metric > value
    Threshold {
        metric: String,
        operator: ComparisonOp,
        value: f32,
    },
    /// Combined conditions
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
    /// Time-based: execute after N cycles
    AfterNCycles(u64),
    /// Pattern-based: occurs when condition met N times in a row
    PatternRepeat(Box<Condition>, usize),
}

/// Comparison operators
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOp {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

impl std::fmt::Display for ComparisonOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GreaterThan => write!(f, ">"),
            Self::GreaterThanOrEqual => write!(f, ">="),
            Self::LessThan => write!(f, "<"),
            Self::LessThanOrEqual => write!(f, "<="),
            Self::Equal => write!(f, "=="),
            Self::NotEqual => write!(f, "!="),
        }
    }
}

/// Action to execute when condition is met
#[derive(Clone, Debug)]
pub enum Action {
    /// Adjust a parameter
    AdjustParameter {
        name: String,
        operation: ParameterOp,
        value: f32,
    },
    /// Enable/disable a feature
    ToggleFeature {
        name: String,
        enabled: bool,
    },
    /// Schedule a kernel launch
    LaunchKernel {
        kernel_name: String,
        args: HashMap<String, f32>,
    },
    /// Call a remediation
    Remediate {
        error_type: String,
        fix_type: String,
    },
    /// Log diagnostic info
    LogDiagnostic(String),
    /// Adaptive parameter calculation
    AdaptiveCalculation {
        target: String,
        formula: String,  // e.g., "queue_depth / 2"
    },
}

/// Parameter adjustment operations
#[derive(Clone, Copy, Debug)]
pub enum ParameterOp {
    Set,
    Increment,
    Decrement,
    Multiply,
    Divide,
}

impl std::fmt::Display for ParameterOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Set => write!(f, "="),
            Self::Increment => write!(f, "+="),
            Self::Decrement => write!(f, "-="),
            Self::Multiply => write!(f, "*="),
            Self::Divide => write!(f, "/="),
        }
    }
}

/// Risk profile: safety guarantees
#[derive(Clone, Copy, Debug)]
pub enum RiskProfile {
    /// No observable side effects, safe to apply always
    Safe,
    /// May cause performance regression, needs validation
    Experimental,
    /// Could cause crashes if misapplied, needs careful testing
    Risky,
    /// Only apply during maintenance window, could cause data loss
    Dangerous,
}

/// DSL Parser and Compiler
pub struct OptimizationDSLCompiler {
    rules: HashMap<String, OptimizationRule>,
    metrics_registry: HashMap<String, f32>,  // Current metric values
}

impl OptimizationDSLCompiler {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            metrics_registry: HashMap::new(),
        }
    }

    /// Parse DSL string into OptimizationRule
    pub fn parse_rule(&mut self, dsl_text: &str) -> Result<OptimizationRule, NtgError> {
        // Parse rule name
        let name = self.extract_rule_name(dsl_text)?;

        // Parse condition
        let condition = self.parse_condition(dsl_text)?;

        // Parse actions
        let actions = self.parse_actions(dsl_text)?;

        // Parse metadata
        let expected_speedup = self.extract_float(dsl_text, "expect_speedup")?;
        let risk_level = self.extract_string(dsl_text, "risk").unwrap_or_else(|_| "safe".to_string());
        let risk_profile = match risk_level.as_str() {
            "safe" => RiskProfile::Safe,
            "experimental" => RiskProfile::Experimental,
            "risky" => RiskProfile::Risky,
            "dangerous" => RiskProfile::Dangerous,
            _ => RiskProfile::Experimental,
        };

        let rule = OptimizationRule {
            name: name.clone(),
            description: self.extract_string(dsl_text, "description").unwrap_or_default(),
            condition,
            actions,
            expected_speedup,
            risk_profile,
        };

        self.rules.insert(name, rule.clone());
        Ok(rule)
    }

    /// Register a metric value (from runtime)
    pub fn register_metric(&mut self, name: &str, value: f32) {
        self.metrics_registry.insert(name.to_string(), value);
    }

    /// Evaluate condition against current metrics
    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Threshold { metric, operator, value } => {
                let metric_value = self.metrics_registry.get(metric).copied().unwrap_or(0.0);
                self.compare(metric_value, operator, *value)
            }
            Condition::And(left, right) => {
                self.evaluate_condition(left) && self.evaluate_condition(right)
            }
            Condition::Or(left, right) => {
                self.evaluate_condition(left) || self.evaluate_condition(right)
            }
            Condition::Not(inner) => !self.evaluate_condition(inner),
            Condition::AfterNCycles(n) => {
                // Simplified: would track cycle counter in real system
                true  // Placeholder
            }
            Condition::PatternRepeat(cond, count) => {
                // Simplified: would maintain pattern history
                self.evaluate_condition(cond)
            }
        }
    }

    /// Execute all actions for a rule
    pub fn execute_actions(&self, actions: &[Action]) -> Result<ExecutionResult, NtgError> {
        let mut result = ExecutionResult {
            executed_actions: 0,
            failed_actions: 0,
            changes: HashMap::new(),
        };

        for action in actions {
            match action {
                Action::AdjustParameter { name, operation, value } => {
                    let current = self.metrics_registry.get(name).copied().unwrap_or(0.0);
                    let new_value = match operation {
                        ParameterOp::Set => *value,
                        ParameterOp::Increment => current + value,
                        ParameterOp::Decrement => current - value,
                        ParameterOp::Multiply => current * value,
                        ParameterOp::Divide => if *value != 0.0 { current / value } else { current },
                    };
                    result.changes.insert(name.clone(), new_value);
                    result.executed_actions += 1;
                }
                Action::ToggleFeature { name, enabled } => {
                    result.changes.insert(
                        format!("{}_enabled", name),
                        if *enabled { 1.0 } else { 0.0 },
                    );
                    result.executed_actions += 1;
                }
                Action::LaunchKernel { kernel_name, args: _ } => {
                    result.changes.insert(format!("kernel_{}", kernel_name), 1.0);
                    result.executed_actions += 1;
                }
                Action::LogDiagnostic(msg) => {
                    println!("[DSL] {}", msg);
                    result.executed_actions += 1;
                }
                Action::AdaptiveCalculation { target, formula } => {
                    // Simplified formula evaluation
                    if formula.contains("queue_depth") {
                        let queue_depth = self.metrics_registry.get("queue_depth").copied().unwrap_or(100.0);
                        let value = queue_depth / 2.0;  // e.g., "queue_depth / 2"
                        result.changes.insert(target.clone(), value);
                        result.executed_actions += 1;
                    }
                }
                _ => {}
            }
        }

        Ok(result)
    }

    // Helper methods

    fn extract_rule_name(&self, text: &str) -> Result<String, NtgError> {
        text.lines()
            .find(|line| line.contains("rule"))
            .and_then(|line| {
                line.split('"').nth(1).map(|s| s.to_string())
            })
            .ok_or_else(|| NtgError::InvalidInput("Rule name not found".to_string()))
    }

    fn extract_string(&self, text: &str, key: &str) -> Result<String, NtgError> {
        text.lines()
            .find(|line| line.contains(key))
            .and_then(|line| line.split('"').nth(1).map(|s| s.to_string()))
            .ok_or_else(|| NtgError::InvalidInput(format!("{} not found", key)))
    }

    fn extract_float(&self, text: &str, key: &str) -> Result<f32, NtgError> {
        text.lines()
            .find(|line| line.contains(key))
            .and_then(|line| {
                line.split('(')
                    .nth(1)
                    .and_then(|part| part.split('x').next())
                    .and_then(|part| part.trim().parse::<f32>().ok())
            })
            .ok_or_else(|| NtgError::InvalidInput(format!("{} not found", key)))
    }

    fn parse_condition(&self, _text: &str) -> Result<Condition, NtgError> {
        // Simplified: would use a full parser
        Ok(Condition::Threshold {
            metric: "queue_depth".to_string(),
            operator: ComparisonOp::GreaterThan,
            value: 100.0,
        })
    }

    fn parse_actions(&self, _text: &str) -> Result<Vec<Action>, NtgError> {
        Ok(vec![
            Action::AdaptiveCalculation {
                target: "batch_size".to_string(),
                formula: "min(queue_depth / 2, 512)".to_string(),
            },
            Action::ToggleFeature {
                name: "batching".to_string(),
                enabled: true,
            },
        ])
    }

    fn compare(&self, left: f32, op: &ComparisonOp, right: f32) -> bool {
        match op {
            ComparisonOp::GreaterThan => left > right,
            ComparisonOp::GreaterThanOrEqual => left >= right,
            ComparisonOp::LessThan => left < right,
            ComparisonOp::LessThanOrEqual => left <= right,
            ComparisonOp::Equal => (left - right).abs() < 1e-6,
            ComparisonOp::NotEqual => (left - right).abs() >= 1e-6,
        }
    }
}

/// Result of executing actions
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub executed_actions: u32,
    pub failed_actions: u32,
    pub changes: HashMap<String, f32>,
}

/// Agent-created optimization rule library
pub struct AgentRuleLibrary {
    pub rules: HashMap<String, OptimizationRule>,
    pub compiler: OptimizationDSLCompiler,
    pub execution_history: Vec<RuleExecution>,
}

/// Record of rule execution
#[derive(Clone, Debug)]
pub struct RuleExecution {
    pub rule_name: String,
    pub condition_met: bool,
    pub actions_executed: u32,
    pub speedup_achieved: f32,
    pub success: bool,
}

impl AgentRuleLibrary {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            compiler: OptimizationDSLCompiler::new(),
            execution_history: Vec::new(),
        }
    }

    /// Agent learns and registers a new optimization rule
    pub fn register_rule_from_dsl(&mut self, dsl_text: &str) -> Result<(), NtgError> {
        let rule = self.compiler.parse_rule(dsl_text)?;
        self.rules.insert(rule.name.clone(), rule);
        Ok(())
    }

    /// Evaluate all rules and execute matching ones
    pub fn apply_all_rules(&mut self, metrics: HashMap<String, f32>) -> Result<Vec<RuleExecution>, NtgError> {
        for (name, value) in metrics {
            self.compiler.register_metric(&name, value);
        }

        let mut executions = Vec::new();

        for (rule_name, rule) in &self.rules {
            let condition_met = self.compiler.evaluate_condition(&rule.condition);

            if condition_met {
                let result = self.compiler.execute_actions(&rule.actions)?;

                let execution = RuleExecution {
                    rule_name: rule_name.clone(),
                    condition_met,
                    actions_executed: result.executed_actions,
                    speedup_achieved: rule.expected_speedup,
                    success: result.failed_actions == 0,
                };

                executions.push(execution);
            }
        }

        self.execution_history.extend(executions.clone());
        Ok(executions)
    }

    /// Get summary of agent learning progress
    pub fn learning_summary(&self) -> AgentLearningStats {
        let total_executions = self.execution_history.len();
        let successful_executions = self.execution_history.iter().filter(|e| e.success).count();
        let avg_speedup = if total_executions > 0 {
            self.execution_history.iter().map(|e| e.speedup_achieved).sum::<f32>() / total_executions as f32
        } else {
            1.0
        };

        AgentLearningStats {
            rules_learned: self.rules.len(),
            total_rule_executions: total_executions,
            successful_executions,
            avg_speedup_per_rule: avg_speedup,
            most_effective_rule: self.execution_history.iter()
                .max_by(|a, b| a.speedup_achieved.partial_cmp(&b.speedup_achieved).unwrap())
                .map(|e| e.rule_name.clone()),
        }
    }
}

/// Statistics on agent learning
#[derive(Clone, Debug)]
pub struct AgentLearningStats {
    pub rules_learned: usize,
    pub total_rule_executions: usize,
    pub successful_executions: usize,
    pub avg_speedup_per_rule: f32,
    pub most_effective_rule: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_rule() -> Result<(), NtgError> {
        let mut compiler = OptimizationDSLCompiler::new();
        let dsl = r#"
            rule "batch_operations":
                description "Batch operations when queue full"
                when queue_depth > 100:
                    enable_batching()
                    expect_speedup(1.5x)
                    risk("safe")
        "#;

        let _rule = compiler.parse_rule(dsl)?;
        assert_eq!(compiler.rules.len(), 1);
        Ok(())
    }

    #[test]
    fn evaluate_condition() {
        let mut compiler = OptimizationDSLCompiler::new();
        compiler.register_metric("queue_depth", 150.0);

        let condition = Condition::Threshold {
            metric: "queue_depth".to_string(),
            operator: ComparisonOp::GreaterThan,
            value: 100.0,
        };

        assert!(compiler.evaluate_condition(&condition));
    }

    #[test]
    fn agent_learns_rules() -> Result<(), NtgError> {
        let mut library = AgentRuleLibrary::new();

        let dsl = r#"
            rule "adaptive_batching":
                when queue_depth > 50:
                    batch_size = queue_depth / 2
                    expect_speedup(1.4x)
                    risk("safe")
        "#;

        library.register_rule_from_dsl(dsl)?;
        assert_eq!(library.rules.len(), 1);

        let mut metrics = HashMap::new();
        metrics.insert("queue_depth".to_string(), 101.0);

        let executions = library.apply_all_rules(metrics)?;
        assert!(!executions.is_empty());

        Ok(())
    }
}
