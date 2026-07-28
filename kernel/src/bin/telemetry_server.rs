use ntg_kernel::ntg::websocket::{SenseReport, TelemetryStream, TelemetryMessage};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() {
    println!("🚀 Aethyro-NTG Telemetry Server — 60 Hz WebSocket Streaming");
    println!("Starting real-time telemetry endpoint...");

    let mut telemetry = TelemetryStream::new();
    let cycle_counter = Arc::new(RwLock::new(0u64));

    // Simulate 60 Hz ticker (16.667ms between ticks)
    let mut ticker = interval(Duration::from_secs_f64(1.0 / 60.0));

    println!("✓ Telemetry stream initialized");
    println!("✓ 60 Hz ticker ready (16.67ms intervals)");
    println!("✓ Ready to emit SenseReports");
    println!();

    // Simulation loop: emit 10 cycles of telemetry
    for _ in 0..10 {
        ticker.tick().await;

        let mut cycle = cycle_counter.write().await;
        *cycle += 1;

        // Create realistic SenseReport with varying values
        let report = SenseReport {
            timestamp_ms: telemetry.elapsed_ms(),
            cycle: *cycle,
            hormone_adrenaline: 0.5 + ((*cycle as f32).sin() * 0.2),
            hormone_cortisol: 0.3 + ((*cycle as f32 * 0.5).cos() * 0.15),
            hormone_serotonin: 0.7 + ((*cycle as f32 * 0.3).sin() * 0.1),
            active_nodes: ((*cycle as usize) % 256) + 64,
            mutations_queued: (*cycle as usize / 2) % 8,
            safety_score: 0.95 - ((*cycle as f32 * 0.01).sin() * 0.05),
            behavioral_drift: ((*cycle as f32 * 0.02).sin().abs() * 0.1),
            energy_consumed_uj: (*cycle as f32) * 2.5,
            coherence: 0.98 - ((*cycle as f32 * 0.015).sin() * 0.03),
        };

        telemetry.emit_report(report.clone()).await;

        // Display telemetry
        println!(
            "[CYCLE {:4}] Adr:{:.2} Cort:{:.2} Sero:{:.2} | Nodes:{:3} | Safety:{:.3} | Energy:{:.1}µJ",
            cycle,
            report.hormone_adrenaline,
            report.hormone_cortisol,
            report.hormone_serotonin,
            report.active_nodes,
            report.safety_score,
            report.energy_consumed_uj
        );

        // Create and serialize telemetry message
        let msg = TelemetryMessage::sense_report(&report);
        let json = msg.to_json_string();
        println!("   → {}", json);
        println!();
    }

    println!("✓ Telemetry server simulation complete");
    println!("Total cycles emitted: {}", *cycle_counter.read().await);
    println!("Streaming latency: < 16.67ms per cycle");
    println!("Buffer capacity: 3600 reports (60 seconds @ 60 Hz)");
}
