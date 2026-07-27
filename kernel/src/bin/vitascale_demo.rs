//! vitascale_demo — end-to-end VITASCALE L0 demonstration (ADR 0010 §4.3, Phase F).
//!
//! Wires up Crown → Oculus + Phageguard + IronChassis + two stub NanoBrains.
//! Runs the vitascale loop for 10 ticks and prints a Gestalt report.
//! No biological life or consciousness claimed — engineering host on iron only.

use ntg_kernel::genomic::organ::Organ;
use ntg_kernel::genomic::sovereign_fitness::SovereignFitnessContext;
use ntg_kernel::genomic::vitascale::{
    awareness_bus::{NeuroSignal, StreamId},
    assembly::Crown,
    body::{host_cpu::IronChassis, BodyAdapter},
    fitness_federation::{FederationWeights, PressureMesh},
    immune_organ::{ImmuneConfig, Phageguard},
    nano_agent::{
        CrownView, MultiAxisFitness, NanoBrain, NanoTickResult, NeuroProposal, OrganKey,
    },
    oculus_organ::{EyeStream, Oculus},
    organ_live::TissueLive,
};

// ---------------------------------------------------------------------------
// Demo NanoBrain: emits a heartbeat signal every tick.
// ---------------------------------------------------------------------------

struct HeartbeatBrain {
    id: u32,
    organ: OrganKey,
    ticks: u32,
}

impl NanoBrain for HeartbeatBrain {
    fn agent_id(&self) -> u32 {
        self.id
    }
    fn organ_key(&self) -> OrganKey {
        self.organ
    }
    fn tick(&mut self, view: &mut CrownView<'_>) -> NanoTickResult {
        self.ticks += 1;
        view.push_signal(
            NeuroSignal::new(StreamId::Nano(self.id), 0, self.ticks as u16)
                .with_payload([self.id, self.ticks, 0, 0]),
        );
        NanoTickResult::idle(self.local_fitness())
    }
    fn local_fitness(&self) -> MultiAxisFitness {
        MultiAxisFitness {
            task: 0.85,
            bio: 0.90,
            structural_cost: 0.10,
            safety: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Demo NanoBrain: structural proposal brain (proposes buffer resize once).
// ---------------------------------------------------------------------------

struct StructuralBrain {
    id: u32,
    proposed: bool,
}

impl NanoBrain for StructuralBrain {
    fn agent_id(&self) -> u32 {
        self.id
    }
    fn organ_key(&self) -> OrganKey {
        OrganKey::Genomic
    }
    fn tick(&mut self, view: &mut CrownView<'_>) -> NanoTickResult {
        view.push_signal(NeuroSignal::new(StreamId::Structural, 0, 1));
        let proposal = if !self.proposed {
            self.proposed = true;
            Some(NeuroProposal {
                agent_id: self.id,
                description: "increase LD window from 128 to 256".into(),
                urgency: 0.6,
            })
        } else {
            None
        };
        NanoTickResult {
            signals: vec![],
            fitness: self.local_fitness(),
            proposal,
        }
    }
    fn local_fitness(&self) -> MultiAxisFitness {
        MultiAxisFitness {
            task: 0.75,
            bio: 0.80,
            structural_cost: 0.15,
            safety: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Demo stub organ (genomic tissue placeholder)
// ---------------------------------------------------------------------------

struct StubGenomicOrgan;

impl Organ for StubGenomicOrgan {
    fn kind(&self) -> &'static str {
        "stub_genomic"
    }
    fn approx_memory_bytes(&self) -> u64 {
        64
    }
    fn structure_fingerprint(&self) -> u64 {
        0xdeadbeef
    }
    fn unit_count(&self) -> usize {
        1
    }
}
impl TissueLive for StubGenomicOrgan {}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("=== VITASCALE L0 Demo (Phase F) ===");
    println!("Engineering host on iron — no claim of biological life or consciousness.\n");

    // 1. Crown + SovereignFitnessContext (single selection authority, KD15).
    let ctx = SovereignFitnessContext::new().expect("SovereignFitnessContext init");
    let mut crown = Crown::new(ctx, None);

    // 2. Oculus: registers Structural and Nano streams on SLA.
    let mut oculus = Oculus::new();
    oculus.register_stream(EyeStream {
        id: StreamId::Structural,
        sla_min_signals_per_tick: 0,
    });
    oculus.register_stream(EyeStream {
        id: StreamId::Nano(1),
        sla_min_signals_per_tick: 0,
    });
    oculus.register_stream(EyeStream {
        id: StreamId::Nano(2),
        sla_min_signals_per_tick: 0,
    });

    // 3. Phageguard: default config (safety_floor=0, utility_floor=0.05).
    let phage = Phageguard::new(ImmuneConfig::default());

    // 4. IronChassis: detect the host CPU.
    let chassis = IronChassis::detect();
    let chassis_frame = chassis.sample();
    println!(
        "IronChassis detected: {} threads, RSS={} bytes\n",
        chassis_frame.health.thread_count, chassis_frame.health.mem_used_bytes
    );

    // 5. Register tissues + brains into Crown.
    crown.add_tissue(Box::new(StubGenomicOrgan), Box::new(HeartbeatBrain { id: 1, organ: OrganKey::Genomic, ticks: 0 }));
    crown.add_tissue(Box::new(StubGenomicOrgan), Box::new(StructuralBrain { id: 2, proposed: false }));
    crown.add_tissue(Box::new(oculus), Box::new(HeartbeatBrain { id: 3, organ: OrganKey::Eye, ticks: 0 }));
    crown.add_tissue(Box::new(phage), Box::new(HeartbeatBrain { id: 4, organ: OrganKey::Immune, ticks: 0 }));
    crown.add_tissue(Box::new(chassis), Box::new(HeartbeatBrain { id: 5, organ: OrganKey::Body, ticks: 0 }));

    println!("Crown: {} tissues registered.\n", crown.tissue_count());

    // 6. PressureMesh: aggregate fitness across nano reports.
    let mesh = PressureMesh::new(FederationWeights::default(), 0.0);

    // 7. Run the VITASCALE loop for 10 ticks.
    let mut all_results: Vec<ntg_kernel::genomic::vitascale::nano_agent::NanoTickResult> = Vec::new();
    for tick_i in 0..10u32 {
        let results = crown.tick_all();
        for r in &results {
            all_results.push(r.clone());
        }
        if tick_i == 9 {
            // Final-tick pressure report.
            let hint = mesh.federate(&results);
            println!(
                "Tick {}: mean_utility={:.3}, min_safety={:.3}, stress={:.3}, veto={}",
                tick_i + 1,
                hint.mean_utility,
                hint.min_safety,
                hint.stress_level,
                hint.selection_veto
            );
        }
    }

    // 8. Gestalt summary.
    let proposals = crown.take_proposals();
    println!("\n--- Gestalt Summary ---");
    println!("Ticks run: 10");
    println!("Bus drops: {}", crown.bus.total_drops());
    println!("Pending proposals: {}", proposals.len());
    for p in &proposals {
        println!("  proposal[{}]: \"{}\" (urgency={:.2})", p.agent_id, p.description, p.urgency);
    }
    println!("Bus pending after loop: {}", crown.bus.pending());
    println!("\nVITASCALE L0 demo complete.");
}
