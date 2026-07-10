//! Ternary "DNA" genome encoding for graph nodes (prototype).
//!
//! Maps the design metaphor **nodes ≈ genes** onto existing primitives:
//! - [`PackedTernary`] = dense ternary payload (weights / logic bits)
//! - `parent_hash` + `generation` = phylogenetic lineage tags
//! - Mutation is pure / offline until wired through ADR 0002 rails
//!
//! **Safety:** nothing here enables self-mod by default. Apply only via
//! `MutationCycle` + ledger when that loop is deliberately turned on.

use crate::ntg::error::NtgError;
use crate::ntg::graph::NodeId;
use crate::ntg::packed::PackedTernary;
use sha2::{Digest, Sha256};

/// Proposed ternary genome edits (index → new ternary value in {-1,0,1}).
#[derive(Clone, Debug, Default)]
pub struct GenomeDelta {
    pub sets: Vec<(usize, i8)>,
    /// XOR into binary_flags (logic mode / regulatory tags).
    pub logic_mode_flip: u64,
}

/// One graph-side "gene": ternary payload + lineage metadata.
#[derive(Clone, Debug)]
pub struct DNAGraphNode {
    pub id: NodeId,
    pub ternary_genome: PackedTernary,
    pub binary_flags: u64,
    pub parent_hash: Option<[u8; 32]>,
    pub generation: u64,
}

impl DNAGraphNode {
    pub fn from_i8(id: NodeId, values: &[i8]) -> Result<Self, NtgError> {
        Ok(Self {
            id,
            ternary_genome: PackedTernary::from_values(values)?,
            binary_flags: 0,
            parent_hash: None,
            generation: 0,
        })
    }

    pub fn len(&self) -> usize {
        self.ternary_genome.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ternary_genome.len() == 0
    }

    pub fn to_i8(&self) -> Result<Vec<i8>, NtgError> {
        self.ternary_genome.to_values()
    }

    /// Non-zero density of the ternary payload in `[0, 1]`.
    pub fn density(&self) -> Result<f32, NtgError> {
        let v = self.to_i8()?;
        if v.is_empty() {
            return Ok(0.0);
        }
        let nz = v.iter().filter(|&&x| x != 0).count();
        Ok(nz as f32 / v.len() as f32)
    }

    /// SHA-256 of (id, generation, flags, packed bytes) for lineage linking.
    pub fn content_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update((self.id as u64).to_le_bytes());
        h.update(self.generation.to_le_bytes());
        h.update(self.binary_flags.to_le_bytes());
        h.update((self.ternary_genome.len() as u64).to_le_bytes());
        // Pack again from values for stable hashing of content
        if let Ok(vals) = self.ternary_genome.to_values() {
            for t in vals {
                h.update([t as u8]);
            }
        }
        let dig = h.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&dig);
        out
    }

    /// Apply a delta; child records `parent_hash = self.content_hash()`.
    pub fn mutate(&self, delta: &GenomeDelta) -> Result<Self, NtgError> {
        let mut vals = self.ternary_genome.to_values()?;
        for &(idx, val) in &delta.sets {
            if idx >= vals.len() {
                return Err(NtgError::IndexOutOfBounds {
                    index: idx,
                    len: vals.len(),
                });
            }
            if !(-1..=1).contains(&val) {
                return Err(NtgError::InvalidTernaryValue(val));
            }
            vals[idx] = val;
        }
        Ok(Self {
            id: self.id,
            ternary_genome: PackedTernary::from_values(&vals)?,
            binary_flags: self.binary_flags ^ delta.logic_mode_flip,
            parent_hash: Some(self.content_hash()),
            generation: self.generation.saturating_add(1),
        })
    }

    /// Compact ledger / notebook line (not a full SignedEntry yet).
    pub fn lineage_line(&self) -> String {
        let ph = self
            .parent_hash
            .map(|h| {
                h.iter()
                    .take(4)
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            })
            .unwrap_or_else(|| "genesis".into());
        format!(
            "dna id={} gen={} flags={:#x} dens={:.3} parent={}..",
            self.id,
            self.generation,
            self.binary_flags,
            self.density().unwrap_or(0.0),
            ph
        )
    }
}

/// Propose a density-biased delta: flip zeros toward ±1 (intelligence/efficiency bias).
/// Deterministic given `seed` — no RNG crate.
pub fn propose_density_delta(genome: &DNAGraphNode, seed: u64, max_flips: usize) -> Result<GenomeDelta, NtgError> {
    let vals = genome.to_i8()?;
    let mut sets = Vec::new();
    let mut s = seed;
    for (i, &v) in vals.iter().enumerate() {
        if sets.len() >= max_flips {
            break;
        }
        // xorshift
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        if v == 0 && s.is_multiple_of(3) {
            let t = if s & 1 == 0 { 1i8 } else { -1i8 };
            sets.push((i, t));
        }
    }
    Ok(GenomeDelta {
        sets,
        logic_mode_flip: if seed & 0x100 != 0 { 1 } else { 0 },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_density() {
        let g = DNAGraphNode::from_i8(0, &[-1, 0, 1, 0, 1]).unwrap();
        assert_eq!(g.len(), 5);
        assert!((g.density().unwrap() - 0.6).abs() < 1e-5);
        assert_eq!(g.to_i8().unwrap(), vec![-1, 0, 1, 0, 1]);
    }

    #[test]
    fn mutate_sets_parent_and_generation() {
        let g = DNAGraphNode::from_i8(7, &[0, 0, 0, 0]).unwrap();
        let parent = g.content_hash();
        let child = g
            .mutate(&GenomeDelta {
                sets: vec![(0, 1), (2, -1)],
                logic_mode_flip: 0b10,
            })
            .unwrap();
        assert_eq!(child.generation, 1);
        assert_eq!(child.parent_hash, Some(parent));
        assert_eq!(child.to_i8().unwrap(), vec![1, 0, -1, 0]);
        assert_eq!(child.binary_flags, 0b10);
        assert_ne!(child.content_hash(), parent);
    }

    #[test]
    fn mutate_rejects_oob_and_bad_ternary() {
        let g = DNAGraphNode::from_i8(0, &[1, 0]).unwrap();
        assert!(g
            .mutate(&GenomeDelta {
                sets: vec![(9, 1)],
                logic_mode_flip: 0,
            })
            .is_err());
        assert!(g
            .mutate(&GenomeDelta {
                sets: vec![(0, 2)],
                logic_mode_flip: 0,
            })
            .is_err());
    }

    #[test]
    fn density_delta_is_deterministic() {
        let g = DNAGraphNode::from_i8(0, &[0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        let d1 = propose_density_delta(&g, 42, 3).unwrap();
        let d2 = propose_density_delta(&g, 42, 3).unwrap();
        assert_eq!(d1.sets, d2.sets);
        let c = g.mutate(&d1).unwrap();
        assert!(c.density().unwrap() >= g.density().unwrap());
    }

    #[test]
    fn lineage_line_readable() {
        let g = DNAGraphNode::from_i8(1, &[1, -1]).unwrap();
        let line = g.lineage_line();
        assert!(line.contains("dna"));
        assert!(line.contains("gen=0"));
    }
}
