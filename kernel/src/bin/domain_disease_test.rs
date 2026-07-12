/// Domain-Agnostic Disease Detection Test
/// Demonstrates risk assessment across 6 disease domains:
/// 1. Genomic (SNPs → disease)
/// 2. Code Quality (anti-patterns, complexity)
/// 3. Malware (virus, trojan signatures)
/// 4. Injection Risk (SQL injection, XSS, prompt injection)
/// 5. Supply Chain (dependency poisoning)
/// 6. Cryptographic (weak algorithms)

use ntg_kernel::genomic::{
    DomainAgent, DomainType, DomainQuery, RiskSeverity,
    ChromosomeBrain, ChromosomeId, KairosState, EmbeddingLayer,
};

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Domain-Agnostic Disease Detection System                    ║");
    println!("║  Genomic | Code | Malware | Injection | Supply Chain | Crypto ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    // Create a mock brain (can be from real data or simulation)
    let brain = ChromosomeBrain {
        chr: ChromosomeId(1),
        neurons: vec![],
        synapses: vec![],
        blocks: vec![],
        embeddings: EmbeddingLayer {
            snp_embeddings: vec![],
            block_embeddings: vec![],
            consolidated: vec![],
        },
        training_cycles: 0,
        kairos_state: KairosState::default(),
    };

    // Create agents for each disease domain
    println!("\n[Initializing Domain Agents]");
    let mut genomic_agent = DomainAgent::new(brain.clone(), DomainType::Genomic);
    let mut code_agent = DomainAgent::new(brain.clone(), DomainType::CodeQuality);
    let mut malware_agent = DomainAgent::new(brain.clone(), DomainType::Malware);
    let mut injection_agent = DomainAgent::new(brain.clone(), DomainType::InjectionRisk);
    let mut supply_agent = DomainAgent::new(brain.clone(), DomainType::SupplyChain);
    let mut crypto_agent = DomainAgent::new(brain.clone(), DomainType::Cryptographic);

    println!("✓ Genomic Agent");
    println!("✓ Code Quality Agent");
    println!("✓ Malware Detection Agent");
    println!("✓ Injection Risk Agent");
    println!("✓ Supply Chain Agent");
    println!("✓ Cryptographic Agent");

    // ========== DOMAIN 1: GENOMIC DISEASE ==========
    println!("\n╔═════════════════════════════════════════════════════════════╗");
    println!("║  DOMAIN 1: GENOMIC DISEASE RISK                           ║");
    println!("╚═════════════════════════════════════════════════════════════╝");
    let genomic_query = DomainQuery::GenomicRisk {
        snp_indices: vec![0, 1, 5],
    };
    let genomic_result = genomic_agent.diagnose(&genomic_query);
    println!(
        "[Risk] Severity: {} | Score: {:.3}",
        genomic_result.primary_risk, genomic_result.risk_score
    );
    println!("  Detected Patterns: {}", genomic_result.detected_patterns.len());
    println!("  Affected Modules: {}", genomic_result.affected_modules.len());
    println!("  Remediation: {}", genomic_result.remediation);

    // ========== DOMAIN 2: CODE QUALITY DISEASE ==========
    println!("\n╔═════════════════════════════════════════════════════════════╗");
    println!("║  DOMAIN 2: CODE QUALITY DISEASE (Anti-patterns)           ║");
    println!("╚═════════════════════════════════════════════════════════════╝");
    let code_query = DomainQuery::CodeDisease {
        module_ids: vec![0, 1, 2],
    };
    let code_result = code_agent.diagnose(&code_query);
    println!(
        "[Risk] Severity: {} | Score: {:.3}",
        code_result.primary_risk, code_result.risk_score
    );
    println!("  Detected Patterns: {}", code_result.detected_patterns.len());
    for pattern in &code_result.detected_patterns {
        println!("    • {} (confidence: {:.2})", pattern.name, pattern.confidence);
    }
    println!("  Remediation: {}", code_result.remediation);

    // ========== DOMAIN 3: MALWARE RISK ==========
    println!("\n╔═════════════════════════════════════════════════════════════╗");
    println!("║  DOMAIN 3: MALWARE DETECTION                              ║");
    println!("║  (Virus, Trojan, Ransomware signatures)                   ║");
    println!("╚═════════════════════════════════════════════════════════════╝");
    let malware_signatures = vec![
        "win32.trojan.generic".to_string(),
        "ransomware.evasion.pattern".to_string(),
        "rootkit.signature".to_string(),
    ];
    let malware_query = DomainQuery::MalwareRisk {
        signatures: malware_signatures,
    };
    let malware_result = malware_agent.diagnose(&malware_query);
    println!(
        "[Risk] Severity: {} | Score: {:.3}",
        malware_result.primary_risk, malware_result.risk_score
    );
    println!("  Detected Patterns: {}", malware_result.detected_patterns.len());
    if malware_result.primary_risk >= RiskSeverity::High {
        println!("  ⚠️  HIGH-RISK MALWARE DETECTED");
    }
    println!("  Remediation: {}", malware_result.remediation);

    // ========== DOMAIN 4: INJECTION RISK ==========
    println!("\n╔═════════════════════════════════════════════════════════════╗");
    println!("║  DOMAIN 4: INJECTION VULNERABILITY                        ║");
    println!("║  (SQL Injection, XSS, Prompt Injection, Command Injection)║");
    println!("╚═════════════════════════════════════════════════════════════╝");
    let injection_query = DomainQuery::InjectionVulnerability {
        entry_points: vec![2, 5, 10],
    };
    let injection_result = injection_agent.diagnose(&injection_query);
    println!(
        "[Risk] Severity: {} | Score: {:.3}",
        injection_result.primary_risk, injection_result.risk_score
    );
    println!("  Detected Patterns: {}", injection_result.detected_patterns.len());
    println!(
        "  Vulnerable Chains: {}",
        injection_result.affected_modules.len()
    );
    for pattern in &injection_result.detected_patterns {
        println!("    • {} → Confidence: {:.2}", pattern.name, pattern.confidence);
    }
    println!("  Remediation: {}", injection_result.remediation);

    // ========== DOMAIN 5: SUPPLY CHAIN RISK ==========
    println!("\n╔═════════════════════════════════════════════════════════════╗");
    println!("║  DOMAIN 5: SUPPLY CHAIN RISK                              ║");
    println!("║  (Dependency Poisoning, Compromised Packages)             ║");
    println!("╚═════════════════════════════════════════════════════════════╝");
    let supply_query = DomainQuery::SupplyChainRisk {
        dependency_indices: vec![0, 3, 7],
    };
    let supply_result = supply_agent.diagnose(&supply_query);
    println!(
        "[Risk] Severity: {} | Score: {:.3}",
        supply_result.primary_risk, supply_result.risk_score
    );
    println!("  Detected Patterns: {}", supply_result.detected_patterns.len());
    println!("  Tight Coupling Risks: {}", supply_result.affected_modules.len());
    println!("  Remediation: {}", supply_result.remediation);

    // ========== DOMAIN 6: CRYPTOGRAPHIC RISK ==========
    println!("\n╔═════════════════════════════════════════════════════════════╗");
    println!("║  DOMAIN 6: CRYPTOGRAPHIC WEAKNESS                         ║");
    println!("║  (Weak Algorithms, Short Keys, Deprecated Ciphers)        ║");
    println!("╚═════════════════════════════════════════════════════════════╝");
    let crypto_query = DomainQuery::CryptoRisk {
        algorithm_ids: vec![0, 1, 2],
    };
    let crypto_result = crypto_agent.diagnose(&crypto_query);
    println!(
        "[Risk] Severity: {} | Score: {:.3}",
        crypto_result.primary_risk, crypto_result.risk_score
    );
    println!("  Detected Patterns: {}", crypto_result.detected_patterns.len());
    println!("  Weak Algorithms Found: {}", crypto_result.affected_modules.len());
    println!("  Remediation: {}", crypto_result.remediation);

    // ========== UNIFIED RISK DASHBOARD ==========
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║  UNIFIED RISK DASHBOARD                                     ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    let results = vec![
        ("Genomic", genomic_result),
        ("Code Quality", code_result),
        ("Malware", malware_result),
        ("Injection", injection_result),
        ("Supply Chain", supply_result),
        ("Cryptographic", crypto_result),
    ];

    println!("\n| Domain             | Severity | Score | Patterns | Status          |");
    println!("|-------------------|----------|-------|----------|-----------------|");
    for (name, result) in &results {
        let status = match result.primary_risk {
            RiskSeverity::Critical => "🔴 CRITICAL",
            RiskSeverity::High => "🟠 HIGH",
            RiskSeverity::Medium => "🟡 MEDIUM",
            RiskSeverity::Low => "🟢 LOW",
            RiskSeverity::None => "✓ CLEAR",
        };
        println!(
            "| {:17} | {:8} | {:.3} | {:8} | {:15} |",
            name, result.primary_risk, result.risk_score, result.detected_patterns.len(), status
        );
    }

    // Overall risk assessment
    let avg_score: f32 = results.iter().map(|(_, r)| r.risk_score).sum::<f32>() / results.len() as f32;
    let critical_count = results.iter().filter(|(_, r)| r.primary_risk == RiskSeverity::Critical).count();
    let high_count = results.iter().filter(|(_, r)| r.primary_risk == RiskSeverity::High).count();

    println!("\n[OVERALL RISK ASSESSMENT]");
    println!("  Average Risk Score: {:.3}", avg_score);
    println!("  Critical Issues: {}", critical_count);
    println!("  High-Severity Issues: {}", high_count);

    if critical_count > 0 {
        println!("\n  ⚠️  ACTION REQUIRED: {} critical issues detected!", critical_count);
        println!("      Immediate remediation required in:");
        for (name, result) in &results {
            if result.primary_risk == RiskSeverity::Critical {
                println!("        • {}", name);
            }
        }
    } else if high_count > 0 {
        println!(
            "\n  ⚠️  ATTENTION: {} high-severity issues detected.",
            high_count
        );
        println!("      Plan remediation within 24-48 hours.");
    } else {
        println!("\n  ✓ No critical or high-severity issues detected.");
    }

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Domain Disease Detection Complete                          ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
}
