/// Report Generation - Pure Rust
/// Generates CSV, JSON, and HTML reports from domain disease test results

use std::fs;

#[derive(Clone, Debug)]
pub struct DomainResult {
    pub name: String,
    pub severity: String,
    pub score: f32,
    pub patterns: u32,
    pub modules: u32,
    pub latency_ms: u32,
    pub remediation: String,
}

#[derive(Clone, Debug)]
pub struct TestResults {
    pub timestamp: String,
    pub domains: Vec<DomainResult>,
}

impl TestResults {
    pub fn new() -> Self {
        Self {
            timestamp: chrono_format(),
            domains: Vec::new(),
        }
    }

    /// Calculate aggregate statistics
    pub fn aggregate_stats(&self) -> AggregateStats {
        let mut mean_score = 0.0f32;
        let mut critical_count = 0u32;
        let mut high_count = 0u32;
        let mut medium_count = 0u32;
        let mut low_count = 0u32;
        let mut total_latency = 0u32;

        for domain in &self.domains {
            mean_score += domain.score;
            total_latency += domain.latency_ms;

            match domain.severity.as_str() {
                "CRITICAL" => critical_count += 1,
                "HIGH" => high_count += 1,
                "MEDIUM" => medium_count += 1,
                "LOW" => low_count += 1,
                _ => {}
            }
        }

        if !self.domains.is_empty() {
            mean_score /= self.domains.len() as f32;
        }

        AggregateStats {
            mean_score,
            critical_count,
            high_count,
            medium_count,
            low_count,
            total_domains: self.domains.len() as u32,
            total_latency_ms: total_latency,
        }
    }

    /// Export to CSV format
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("Domain,Severity,Score,Patterns,Modules,Latency_ms\n");

        for domain in &self.domains {
            csv.push_str(&format!(
                "{},{},{:.3},{},{},{}\n",
                domain.name, domain.severity, domain.score, domain.patterns, domain.modules, domain.latency_ms
            ));
        }

        csv
    }

    /// Export to JSON format
    pub fn to_json(&self) -> String {
        let stats = self.aggregate_stats();

        let mut json = format!(
            r#"{{
  "test_date": "{}",
  "total_domains": {},
  "mean_score": {:.3},
  "critical_count": {},
  "high_count": {},
  "medium_count": {},
  "low_count": {},
  "total_latency_ms": {},
  "domains": [
"#,
            self.timestamp,
            stats.total_domains,
            stats.mean_score,
            stats.critical_count,
            stats.high_count,
            stats.medium_count,
            stats.low_count,
            stats.total_latency_ms
        );

        for (i, domain) in self.domains.iter().enumerate() {
            json.push_str(&format!(
                r#"    {{
      "name": "{}",
      "severity": "{}",
      "score": {:.3},
      "patterns": {},
      "modules": {}
    }}"#,
                domain.name, domain.severity, domain.score, domain.patterns, domain.modules
            ));

            if i < self.domains.len() - 1 {
                json.push(',');
            }
            json.push('\n');
        }

        json.push_str("\n  ]\n}");
        json
    }

    /// Export to HTML format
    pub fn to_html(&self) -> String {
        let stats = self.aggregate_stats();

        let mut html = String::from(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Domain Disease Detection Results</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }
        h1 { color: #333; border-bottom: 3px solid #0066cc; padding-bottom: 10px; }
        h2 { color: #0066cc; margin-top: 20px; }
        table { border-collapse: collapse; width: 100%; background: white; margin: 20px 0; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        th { background: #0066cc; color: white; padding: 12px; text-align: left; }
        td { padding: 12px; border-bottom: 1px solid #ddd; }
        tr:hover { background: #f9f9f9; }
        .critical { color: #d32f2f; font-weight: bold; }
        .high { color: #f57c00; font-weight: bold; }
        .medium { color: #fbc02d; font-weight: bold; }
        .low { color: #388e3c; font-weight: bold; }
        .summary { background: #e3f2fd; padding: 15px; border-radius: 5px; margin: 20px 0; border-left: 4px solid #0066cc; }
        .metric { display: inline-block; margin: 10px 20px; }
        .severity-indicator { padding: 4px 8px; border-radius: 4px; font-weight: bold; }
        .critical-bg { background: #ffebee; }
        .high-bg { background: #fff3e0; }
        .medium-bg { background: #fffde7; }
        .low-bg { background: #e8f5e9; }
        footer { color: #999; font-size: 12px; margin-top: 40px; border-top: 1px solid #ddd; padding-top: 20px; }
    </style>
</head>
<body>
    <h1>Domain-Agnostic Disease Detection System</h1>
    <p><strong>Test Date:</strong> "#);
        html.push_str(&self.timestamp);
        html.push_str(r#"</p>

    <div class="summary">
        <h2>Summary Statistics</h2>
        <div class="metric"><strong>Mean Risk Score:</strong> "#);

        html.push_str(&format!("{:.3}", stats.mean_score));
        html.push_str(r#"</div>
        <div class="metric"><strong>Critical Issues:</strong> <span class="critical">"#);
        html.push_str(&stats.critical_count.to_string());
        html.push_str(r#"</span></div>
        <div class="metric"><strong>High Issues:</strong> <span class="high">"#);
        html.push_str(&stats.high_count.to_string());
        html.push_str(r#"</span></div>
        <div class="metric"><strong>Medium Issues:</strong> <span class="medium">"#);
        html.push_str(&stats.medium_count.to_string());
        html.push_str(r#"</span></div>
        <div class="metric"><strong>Low Issues:</strong> <span class="low">"#);
        html.push_str(&stats.low_count.to_string());
        html.push_str(r#"</span></div>
    </div>

    <h2>Domain Results</h2>
    <table>
        <tr>
            <th>Domain</th>
            <th>Severity</th>
            <th>Risk Score</th>
            <th>Patterns</th>
            <th>Modules</th>
            <th>Latency (ms)</th>
        </tr>
"#);

        for domain in &self.domains {
            let severity_class = match domain.severity.as_str() {
                "CRITICAL" => "critical-bg critical",
                "HIGH" => "high-bg high",
                "MEDIUM" => "medium-bg medium",
                "LOW" => "low-bg low",
                _ => "low-bg",
            };

            html.push_str(&format!(
                r#"        <tr>
            <td><strong>{}</strong></td>
            <td class="{}"><div class="severity-indicator">{}</div></td>
            <td>{:.3}</td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
        </tr>
"#,
                domain.name, severity_class, domain.severity, domain.score, domain.patterns, domain.modules, domain.latency_ms
            ));
        }

        html.push_str(r#"    </table>

    <h2>Remediation Actions Required</h2>
    <ul>
"#);

        if stats.critical_count > 0 {
            html.push_str(&format!(
                "<li><strong style=\"color: #d32f2f;\">🔴 CRITICAL ({})</strong>: Immediate intervention required</li>\n",
                stats.critical_count
            ));
        }

        if stats.high_count > 0 {
            html.push_str(&format!(
                "<li><strong style=\"color: #f57c00;\">🟠 HIGH ({})</strong>: Urgent remediation within 24-48 hours</li>\n",
                stats.high_count
            ));
        }

        if stats.medium_count > 0 {
            html.push_str(&format!(
                "<li><strong style=\"color: #fbc02d;\">🟡 MEDIUM ({})</strong>: Plan remediation within 1-2 weeks</li>\n",
                stats.medium_count
            ));
        }

        html.push_str(r#"    </ul>

    <h2>Domain-Specific Remediation Guidance</h2>
"#);

        for domain in &self.domains {
            html.push_str(&format!(
                r#"    <h3>{}</h3>
    <p><strong>Severity:</strong> <span class="severity-indicator {}">{}</span></p>
    <p><strong>Risk Score:</strong> {:.3}</p>
    <p><strong>Patterns Detected:</strong> {}</p>
    <p><strong>Action:</strong> {}</p>
    <hr>
"#,
                domain.name,
                match domain.severity.as_str() {
                    "CRITICAL" => "critical-bg critical",
                    "HIGH" => "high-bg high",
                    "MEDIUM" => "medium-bg medium",
                    "LOW" => "low-bg low",
                    _ => "low-bg",
                },
                domain.severity,
                domain.score,
                domain.patterns,
                domain.remediation
            ));
        }

        html.push_str(
            r#"    <footer>
        <p>Domain-Agnostic Disease Detection Framework v1.0</p>
        <p>Pure Rust Implementation | Production Ready</p>
    </footer>
</body>
</html>"#,
        );

        html
    }

    /// Write all formats to files
    pub fn write_reports(&self, directory: &str) -> Result<(), std::io::Error> {
        // Create directory if not exists
        fs::create_dir_all(directory)?;

        // Write CSV
        let csv_path = format!("{}/metrics.csv", directory);
        fs::write(&csv_path, self.to_csv())?;
        println!("✓ Wrote CSV: {}", csv_path);

        // Write JSON
        let json_path = format!("{}/summary.json", directory);
        fs::write(&json_path, self.to_json())?;
        println!("✓ Wrote JSON: {}", json_path);

        // Write HTML
        let html_path = format!("{}/report.html", directory);
        fs::write(&html_path, self.to_html())?;
        println!("✓ Wrote HTML: {}", html_path);

        Ok(())
    }

    /// Print summary to console
    pub fn print_summary(&self) {
        let stats = self.aggregate_stats();

        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║  SUMMARY STATISTICS                                        ║");
        println!("╚════════════════════════════════════════════════════════════════╝");
        println!("\nDomains tested: {}", stats.total_domains);
        println!("Mean risk score: {:.3}", stats.mean_score);
        println!("Total latency: {} ms", stats.total_latency_ms);

        println!("\nSeverity breakdown:");
        println!("  🔴 CRITICAL: {}", stats.critical_count);
        println!("  🟠 HIGH: {}", stats.high_count);
        println!("  🟡 MEDIUM: {}", stats.medium_count);
        println!("  🟢 LOW: {}", stats.low_count);

        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║  RESULTS TABLE                                             ║");
        println!("╚════════════════════════════════════════════════════════════════╝\n");

        println!("| Domain           | Severity | Score | Patterns | Status      |");
        println!("|------------------|----------|-------|----------|-------------|");

        for domain in &self.domains {
            let status_emoji = match domain.severity.as_str() {
                "CRITICAL" => "🔴 CRITICAL",
                "HIGH" => "🟠 HIGH",
                "MEDIUM" => "🟡 MEDIUM",
                "LOW" => "🟢 LOW",
                _ => "✓ CLEAR",
            };

            println!(
                "| {:16} | {:8} | {:.3} | {:8} | {:11} |",
                domain.name, domain.severity, domain.score, domain.patterns, status_emoji
            );
        }

        println!("\n");
    }
}

#[derive(Clone, Debug)]
pub struct AggregateStats {
    pub mean_score: f32,
    pub critical_count: u32,
    pub high_count: u32,
    pub medium_count: u32,
    pub low_count: u32,
    pub total_domains: u32,
    pub total_latency_ms: u32,
}

/// Simple timestamp format (no chrono dependency)
fn chrono_format() -> String {
    format!("2026-07-12T14:30:00Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_results_to_csv() {
        let mut results = TestResults::new();
        results.domains.push(DomainResult {
            name: "Test".to_string(),
            severity: "HIGH".to_string(),
            score: 0.5,
            patterns: 2,
            modules: 3,
            latency_ms: 50,
            remediation: "Test action".to_string(),
        });

        let csv = results.to_csv();
        assert!(csv.contains("Test"));
        assert!(csv.contains("HIGH"));
        assert!(csv.contains("0.500"));
    }

    #[test]
    fn test_results_to_json() {
        let mut results = TestResults::new();
        results.domains.push(DomainResult {
            name: "Test".to_string(),
            severity: "MEDIUM".to_string(),
            score: 0.4,
            patterns: 1,
            modules: 2,
            latency_ms: 40,
            remediation: "Test".to_string(),
        });

        let json = results.to_json();
        assert!(json.contains("\"name\": \"Test\""));
        assert!(json.contains("\"severity\": \"MEDIUM\""));
        assert!(json.contains("\"score\": 0.400"));
    }

    #[test]
    fn test_aggregate_stats() {
        let mut results = TestResults::new();
        results.domains.push(DomainResult {
            name: "Critical".to_string(),
            severity: "CRITICAL".to_string(),
            score: 0.9,
            patterns: 3,
            modules: 5,
            latency_ms: 100,
            remediation: "Urgent".to_string(),
        });

        results.domains.push(DomainResult {
            name: "Low".to_string(),
            severity: "LOW".to_string(),
            score: 0.1,
            patterns: 0,
            modules: 0,
            latency_ms: 20,
            remediation: "Monitor".to_string(),
        });

        let stats = results.aggregate_stats();
        assert_eq!(stats.critical_count, 1);
        assert_eq!(stats.low_count, 1);
        assert!(stats.mean_score > 0.4 && stats.mean_score < 0.6);
    }
}
