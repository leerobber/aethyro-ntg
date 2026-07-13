//! Durable snapshot for SovereignBrain + fitness context (foundation A).
//!
//! Saves a directory of portable text files — not a full process image:
//! - `ltm.jsonl` — long-term motifs
//! - `calib.wire` — Phase 4 CalibModel wire format
//! - `meta.json` — generation, working-set capacity, structure stats
//! - `language_docs.jsonl` — optional re-ingestable doc texts if provided
//!
//! Ledger binary format is not fully rehydrated here; decision history
//! should live in JSONL from `selection_loop` for audit replay.

use crate::genomic::language_organ::LanguageOrgan;
use crate::genomic::sovereign_brain::{LtmMotif, SovereignBrain, StructuralMetrics};
use crate::genomic::sovereign_fitness::SovereignFitnessContext;
use crate::ntg::calib::CalibModel;
use std::fs;
use std::io::Write;
use std::path::Path;

/// What was written on save.
#[derive(Clone, Debug, Default)]
pub struct SnapshotReport {
    pub dir: String,
    pub n_motifs: usize,
    pub wrote_calib: bool,
    pub wrote_language_docs: usize,
    pub generation: u64,
}

/// Save durable pieces of the sovereign stack into `dir`.
pub fn save_snapshot(
    dir: &Path,
    brain: &SovereignBrain,
    ctx: &SovereignFitnessContext,
    language_docs: &[(&str, &str)],
) -> Result<SnapshotReport, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    // LTM
    let ltm_path = dir.join("ltm.jsonl");
    {
        let mut f = fs::File::create(&ltm_path).map_err(|e| e.to_string())?;
        for m in &brain.ltm {
            writeln!(
                f,
                "{{\"id\":{},\"chr\":{},\"block_id\":{},\"n_snps\":{},\"mean_r2\":{:.6},\"start_bp\":{},\"end_bp\":{},\"hits\":{},\"sig\":[{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}]}}",
                m.id,
                m.source_chr.0,
                m.block_id,
                m.n_snps,
                m.mean_r_squared,
                m.start_bp,
                m.end_bp,
                m.hit_count,
                m.signature[0],
                m.signature[1],
                m.signature[2],
                m.signature[3],
                m.signature[4],
                m.signature[5],
                m.signature[6],
                m.signature[7],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Calib
    let mut wrote_calib = false;
    if let Some(model) = &ctx.calib_model {
        fs::write(dir.join("calib.wire"), model.to_wire()).map_err(|e| e.to_string())?;
        wrote_calib = true;
    }

    // Language docs for re-ingest
    let mut wrote_docs = 0usize;
    if !language_docs.is_empty() {
        let mut f = fs::File::create(dir.join("language_docs.jsonl")).map_err(|e| e.to_string())?;
        for (label, text) in language_docs {
            let esc = text.replace('\\', "\\\\").replace('\n', "\\n").replace('"', "\\\"");
            writeln!(f, "{{\"label\":\"{}\",\"text\":\"{}\"}}", label.replace('"', "'"), esc)
                .map_err(|e| e.to_string())?;
            wrote_docs += 1;
        }
    }

    let s = brain.measure_structure();
    let meta = format!(
        "{{\n  \"generation\": {},\n  \"working_set_capacity\": {},\n  \"n_chromosomes\": {},\n  \"n_neurons\": {},\n  \"n_synapses\": {},\n  \"n_ltm_motifs\": {},\n  \"mean_synapse_weight\": {:.6},\n  \"mean_block_r2\": {:.6},\n  \"approx_memory_bytes\": {},\n  \"ledger_entries\": {}\n}}\n",
        brain.generation,
        brain.working_set.capacity,
        s.n_chromosomes,
        s.n_neurons,
        s.n_synapses,
        s.n_ltm_motifs,
        s.mean_synapse_weight,
        s.mean_block_r2,
        s.approx_memory_bytes,
        ctx.ledger_entry_count()
    );
    fs::write(dir.join("meta.json"), meta).map_err(|e| e.to_string())?;

    Ok(SnapshotReport {
        dir: dir.display().to_string(),
        n_motifs: brain.ltm.len(),
        wrote_calib,
        wrote_language_docs: wrote_docs,
        generation: brain.generation,
    })
}

/// Load LTM + optional calib into an existing brain/context.
///
/// Chromosome bodies are **not** reconstructed from disk (require VCF re-ingest);
/// this restores memory/scorer state for continued selection after a run.
pub fn load_snapshot_into(
    dir: &Path,
    brain: &mut SovereignBrain,
    ctx: &mut SovereignFitnessContext,
) -> Result<SnapshotReport, String> {
    let ltm_path = dir.join("ltm.jsonl");
    if ltm_path.is_file() {
        let text = fs::read_to_string(&ltm_path).map_err(|e| e.to_string())?;
        let mut motifs = Vec::new();
        let mut max_id = 0u64;
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Some(m) = parse_ltm_line(line) {
                max_id = max_id.max(m.id);
                motifs.push(m);
            }
        }
        brain.ltm = motifs;
        brain.next_motif_id = max_id.saturating_add(1);
    }

    let calib_path = dir.join("calib.wire");
    let mut wrote_calib = false;
    if calib_path.is_file() {
        let wire = fs::read_to_string(&calib_path).map_err(|e| e.to_string())?;
        let model = CalibModel::from_wire(&wire).map_err(|e| e.to_string())?;
        ctx.calib_model = Some(model);
        wrote_calib = true;
    }

    let docs_path = dir.join("language_docs.jsonl");
    let mut wrote_docs = 0usize;
    if docs_path.is_file() {
        let text = fs::read_to_string(&docs_path).map_err(|e| e.to_string())?;
        let mut organ = brain.language.take().unwrap_or_else(LanguageOrgan::new);
        for line in text.lines() {
            if let Some((label, body)) = parse_doc_line(line) {
                organ.ingest_document(&label, &body);
                wrote_docs += 1;
            }
        }
        if let Some(model) = &ctx.calib_model {
            organ.model = Some(model.clone());
        }
        brain.attach_language(organ);
    }

    if let Ok(meta) = fs::read_to_string(dir.join("meta.json")) {
        if let Some(g) = extract_u64_field(&meta, "generation") {
            brain.generation = g;
        }
    }
    brain.refresh_structure();

    Ok(SnapshotReport {
        dir: dir.display().to_string(),
        n_motifs: brain.ltm.len(),
        wrote_calib,
        wrote_language_docs: wrote_docs,
        generation: brain.generation,
    })
}

fn extract_u64_field(json: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\"");
    let i = json.find(&pat)?;
    let rest = &json[i + pat.len()..];
    let colon = rest.find(':')?;
    let num: String = rest[colon + 1..]
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    num.parse().ok()
}

fn parse_ltm_line(line: &str) -> Option<LtmMotif> {
    // Minimal hand parser for our write format.
    let id = extract_json_u64(line, "id")?;
    let chr = extract_json_u64(line, "chr")? as u8;
    let block_id = extract_json_u64(line, "block_id")? as u32;
    let n_snps = extract_json_u64(line, "n_snps")? as u32;
    let mean_r2 = extract_json_f32(line, "mean_r2")?;
    let start_bp = extract_json_u64(line, "start_bp")? as u32;
    let end_bp = extract_json_u64(line, "end_bp")? as u32;
    let hits = extract_json_u64(line, "hits")?;
    let sig = extract_json_sig8(line)?;
    Some(LtmMotif {
        id,
        source_chr: crate::genomic::chromosome_brain::ChromosomeId(chr),
        block_id,
        n_snps,
        mean_r_squared: mean_r2,
        start_bp,
        end_bp,
        signature: sig,
        hit_count: hits,
    })
}

fn parse_doc_line(line: &str) -> Option<(String, String)> {
    let label = extract_json_string(line, "label")?;
    let text = extract_json_string(line, "text")?;
    let text = text.replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\");
    Some((label, text))
}

fn extract_json_u64(line: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)?;
    let rest = &line[i + pat.len()..];
    let num: String = rest
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    num.parse().ok()
}

fn extract_json_f32(line: &str, key: &str) -> Option<f32> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)?;
    let rest = &line[i + pat.len()..];
    let num: String = rest
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == 'e' || *c == 'E' || *c == '+')
        .collect();
    num.parse().ok()
}

fn extract_json_string(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let i = line.find(&pat)?;
    let rest = &line[i + pat.len()..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else if c == '"' {
            break;
        } else {
            out.push(c);
        }
    }
    Some(out)
}

fn extract_json_sig8(line: &str) -> Option<[f32; 8]> {
    let i = line.find("\"sig\":[")?;
    let rest = &line[i + 7..];
    let end = rest.find(']')?;
    let body = &rest[0..end];
    let mut vals = [0.0f32; 8];
    for (idx, part) in body.split(',').enumerate() {
        if idx >= 8 {
            break;
        }
        vals[idx] = part.trim().parse().ok()?;
    }
    Some(vals)
}

/// Convenience: structure metrics from meta file alone.
pub fn read_meta_structure(dir: &Path) -> Result<StructuralMetrics, String> {
    let meta = fs::read_to_string(dir.join("meta.json")).map_err(|e| e.to_string())?;
    Ok(StructuralMetrics {
        n_chromosomes: extract_u64_field(&meta, "n_chromosomes").unwrap_or(0) as u32,
        n_neurons: extract_u64_field(&meta, "n_neurons").unwrap_or(0) as u32,
        n_synapses: extract_u64_field(&meta, "n_synapses").unwrap_or(0) as u32,
        n_blocks: 0,
        n_ltm_motifs: extract_u64_field(&meta, "n_ltm_motifs").unwrap_or(0) as u32,
        working_set_len: 0,
        approx_memory_bytes: extract_u64_field(&meta, "approx_memory_bytes").unwrap_or(0),
        mean_synapse_weight: extract_json_f32(&meta, "mean_synapse_weight").unwrap_or(0.0),
        mean_block_r2: extract_json_f32(&meta, "mean_block_r2").unwrap_or(0.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genomic::language_organ::fixture_docs;
    use crate::genomic::sovereign_brain::synthetic_test_brain;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn roundtrip_ltm_and_calib() {
        let mut brain = synthetic_test_brain();
        brain.consolidate(0.5, 0.0);
        let mut ctx = SovereignFitnessContext::new().unwrap();
        ctx.freeze_all_from_brain(&brain);
        let _ = ctx.install_calib_from_fixtures(12);

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ntg_snap_{stamp}"));
        let docs = fixture_docs();
        let pairs: Vec<(&str, &str)> = docs.iter().map(|(a, b)| (*a, *b)).collect();
        let rep = save_snapshot(&dir, &brain, &ctx, &pairs).unwrap();
        assert!(rep.n_motifs >= 1);
        assert!(rep.wrote_calib);

        let mut brain2 = SovereignBrain::new(32);
        // empty chromosomes; only LTM/calib restored
        let mut ctx2 = SovereignFitnessContext::new().unwrap();
        let loaded = load_snapshot_into(&dir, &mut brain2, &mut ctx2).unwrap();
        assert_eq!(loaded.n_motifs, rep.n_motifs);
        assert!(ctx2.calib_model.is_some());
        assert!(brain2.language.is_some());
        let _ = fs::remove_dir_all(&dir);
    }
}
