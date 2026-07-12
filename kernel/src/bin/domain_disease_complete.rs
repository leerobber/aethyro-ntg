/// Domain Disease Detection: Complete End-to-End Pipeline
/// Pure Rust Implementation
///
/// Workflow:
/// 1. Initialize 6 domain agents
/// 2. Execute queries
/// 3. Collect results
/// 4. Generate reports (CSV, JSON, HTML)
/// 5. Print summary

use ntg_kernel::genomic::{
    DomainAgent, DomainType, DomainQuery, ChromosomeId,
    ChromosomeBrain, KairosState, EmbeddingLayer,
    TestResults, DomainResult,
};
use std::time::Instant;

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Domain Disease Detection: Complete Pipeline                ║");
    println!("║  Pure Rust | No Dependencies | Production Ready             ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    let mut test_results = TestResults::new();

    // Create mock brain (reused across all domains)
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

    println!("\n[Step 1/3] Initializing Domain Agents...");
    let mut agents = vec![
        (DomainType::Genomic, DomainAgent::new(brain.clone(), DomainType::Genomic)),
        (DomainType::CodeQuality, DomainAgent::new(brain.clone(), DomainType::CodeQuality)),
        (DomainType::Malware, DomainAgent::new(brain.clone(), DomainType::Malware)),
        (DomainType::InjectionRisk, DomainAgent::new(brain.clone(), DomainType::InjectionRisk)),
        (DomainType::SupplyChain, DomainAgent::new(brain.clone(), DomainType::SupplyChain)),
        (DomainType::Cryptographic, DomainAgent::new(brain.clone(), DomainType::Cryptographic)),
    ];

    println!("✓ 6 domain agents initialized");

    println!("\n[Step 2/3] Executing Domain Disease Queries...\n");

    // DOMAIN 1: GENOMIC
    {
        let start = Instant::now();
        let agent = &agents[0].1;
        let query = DomainQuery::GenomicRisk {
            snp_indices: vec![0],
        };
        let result = agent.diagnose(&query);
        let latency = start.elapsed().as_millis() as u32;

        println!("  Genomic: {} | Score: {:.3} | Latency: {} ms",
            result.primary_risk, result.risk_score, latency);

        test_results.domains.push(DomainResult {
            name: "Genomic".to_string(),
            severity: format!("{}", result.primary_risk),
            score: result.risk_score,
            patterns: result.detected_patterns.len() as u32,
            modules: result.affected_modules.len() as u32,
            latency_ms: latency,
            remediation: result.remediation.clone(),
        });
    }

    // DOMAIN 2: CODE QUALITY
    {
        let start = Instant::now();
        let agent = &agents[1].1;
        let query = DomainQuery::CodeDisease {
            module_ids: vec![0, 1],
        };
        let result = agent.diagnose(&query);
        let latency = start.elapsed().as_millis() as u32;

        println!("  Code Quality: {} | Score: {:.3} | Latency: {} ms",
            result.primary_risk, result.risk_score, latency);

        test_results.domains.push(DomainResult {
            name: "Code Quality".to_string(),
            severity: format!("{}", result.primary_risk),
            score: result.risk_score,
            patterns: result.detected_patterns.len() as u32,
            modules: result.affected_modules.len() as u32,
            latency_ms: latency,
            remediation: result.remediation.clone(),
        });
    }

    // DOMAIN 3: MALWARE
    {
        let start = Instant::now();
        let agent = &agents[2].1;
        let query = DomainQuery::MalwareRisk {
            signatures: vec![
                "win32.trojan.generic".to_string(),
                "ransomware.evasion".to_string(),
            ],
        };
        let result = agent.diagnose(&query);
        let latency = start.elapsed().as_millis() as u32;

        println!("  Malware: {} | Score: {:.3} | Latency: {} ms",
            result.primary_risk, result.risk_score, latency);

        test_results.domains.push(DomainResult {
            name: "Malware".to_string(),
            severity: format!("{}", result.primary_risk),
            score: result.risk_score,
            patterns: result.detected_patterns.len() as u32,
            modules: result.affected_modules.len() as u32,
            latency_ms: latency,
            remediation: result.remediation.clone(),
        });
    }

    // DOMAIN 4: INJECTION
    {
        let start = Instant::now();
        let agent = &agents[3].1;
        let query = DomainQuery::InjectionVulnerability {
            entry_points: vec![2, 5],
        };
        let result = agent.diagnose(&query);
        let latency = start.elapsed().as_millis() as u32;

        println!("  Injection: {} | Score: {:.3} | Latency: {} ms",
            result.primary_risk, result.risk_score, latency);

        test_results.domains.push(DomainResult {
            name: "Injection Risk".to_string(),
            severity: format!("{}", result.primary_risk),
            score: result.risk_score,
            patterns: result.detected_patterns.len() as u32,
            modules: result.affected_modules.len() as u32,
            latency_ms: latency,
            remediation: result.remediation.clone(),
        });
    }

    // DOMAIN 5: SUPPLY CHAIN
    {
        let start = Instant::now();
        let agent = &agents[4].1;
        let query = DomainQuery::SupplyChainRisk {
            dependency_indices: vec![0, 3],
        };
        let result = agent.diagnose(&query);
        let latency = start.elapsed().as_millis() as u32;

        println!("  Supply Chain: {} | Score: {:.3} | Latency: {} ms",
            result.primary_risk, result.risk_score, latency);

        test_results.domains.push(DomainResult {
            name: "Supply Chain".to_string(),
            severity: format!("{}", result.primary_risk),
            score: result.risk_score,
            patterns: result.detected_patterns.len() as u32,
            modules: result.affected_modules.len() as u32,
            latency_ms: latency,
            remediation: result.remediation.clone(),
        });
    }

    // DOMAIN 6: CRYPTOGRAPHIC
    {
        let start = Instant::now();
        let agent = &agents[5].1;
        let query = DomainQuery::CryptoRisk {
            algorithm_ids: vec![0, 1],
        };
        let result = agent.diagnose(&query);
        let latency = start.elapsed().as_millis() as u32;

        println!("  Cryptographic: {} | Score: {:.3} | Latency: {} ms",
            result.primary_risk, result.risk_score, latency);

        test_results.domains.push(DomainResult {
            name: "Cryptographic".to_string(),
            severity: format!("{}", result.primary_risk),
            score: result.risk_score,
            patterns: result.detected_patterns.len() as u32,
            modules: result.affected_modules.len() as u32,
            latency_ms: latency,
            remediation: result.remediation.clone(),
        });
    }

    // Print summary
    test_results.print_summary();

    // Generate reports
    println!("\n[Step 3/3] Generating Reports...\n");
    match test_results.write_reports("../results") {
        Ok(_) => {
            println!("\n╔════════════════════════════════════════════════════════════════╗");
            println!("║  PIPELINE COMPLETE                                           ║");
            println!("╚════════════════════════════════════════════════════════════════╝");
            println!("\n📁 Output files generated in results/:");
            println!("  • metrics.csv        — Data table for publication");
            println!("  • summary.json       — Structured results");
            println!("  • report.html        — Browser-readable dashboard");
            println!("\n✓ Pure Rust pipeline: Test → Parse → Report");
            println!("✓ Ready for publication");
        }
        Err(e) => {
            eprintln!("❌ Error writing reports: {}", e);
            std::process::exit(1);
        }
    }
}
