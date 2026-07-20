//! Hypothesis registry for Adult Lab science (theory → test → measure).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Outcome of an empirical test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HypothesisVerdict {
    Accept,
    Reject,
    Inconclusive,
}

impl HypothesisVerdict {
    pub fn name(self) -> &'static str {
        match self {
            HypothesisVerdict::Accept => "ACCEPT",
            HypothesisVerdict::Reject => "REJECT",
            HypothesisVerdict::Inconclusive => "INCONCLUSIVE",
        }
    }
}

/// One science hypothesis with predicted and measured metrics.
#[derive(Clone, Debug)]
pub struct Hypothesis {
    pub id: String,
    pub track: String,
    pub claim: String,
    pub method: String,
    pub predicted: String,
    pub measured: String,
    pub verdict: HypothesisVerdict,
    pub notes: String,
    pub sealed_ns: u64,
}

impl Hypothesis {
    pub fn new(
        id: impl Into<String>,
        track: impl Into<String>,
        claim: impl Into<String>,
        method: impl Into<String>,
        predicted: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            track: track.into(),
            claim: claim.into(),
            method: method.into(),
            predicted: predicted.into(),
            measured: String::new(),
            verdict: HypothesisVerdict::Inconclusive,
            notes: String::new(),
            sealed_ns: now_ns(),
        }
    }

    pub fn conclude(
        mut self,
        measured: impl Into<String>,
        verdict: HypothesisVerdict,
        notes: impl Into<String>,
    ) -> Self {
        self.measured = measured.into();
        self.verdict = verdict;
        self.notes = notes.into();
        self.sealed_ns = now_ns();
        self
    }

    pub fn to_jsonl_line(&self) -> String {
        let esc = |s: &str| {
            s.replace('\\', "\\\\")
                .replace('\n', "\\n")
                .replace('"', "'")
        };
        format!(
            "{{\"id\":\"{}\",\"track\":\"{}\",\"claim\":\"{}\",\"method\":\"{}\",\"predicted\":\"{}\",\"measured\":\"{}\",\"verdict\":\"{}\",\"notes\":\"{}\",\"ns\":{}}}",
            esc(&self.id),
            esc(&self.track),
            esc(&self.claim),
            esc(&self.method),
            esc(&self.predicted),
            esc(&self.measured),
            self.verdict.name(),
            esc(&self.notes),
            self.sealed_ns
        )
    }
}

/// Append-only hypothesis log under lab dir.
#[derive(Clone, Debug, Default)]
pub struct HypothesisRegistry {
    pub entries: Vec<Hypothesis>,
}

impl HypothesisRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn path(lab_dir: &Path) -> std::path::PathBuf {
        lab_dir.join("science").join("hypotheses.jsonl")
    }

    pub fn load(lab_dir: &Path) -> Result<Self, String> {
        let path = Self::path(lab_dir);
        if !path.is_file() {
            return Ok(Self::new());
        }
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut entries = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Some(h) = parse_hyp_line(line) {
                entries.push(h);
            }
        }
        Ok(Self { entries })
    }

    pub fn record(&mut self, lab_dir: &Path, h: Hypothesis) -> Result<(), String> {
        let sci = lab_dir.join("science");
        fs::create_dir_all(&sci).map_err(|e| e.to_string())?;
        let path = Self::path(lab_dir);
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        writeln!(f, "{}", h.to_jsonl_line()).map_err(|e| e.to_string())?;
        self.entries.push(h);
        Ok(())
    }

    pub fn n_accept(&self) -> usize {
        self.entries
            .iter()
            .filter(|h| h.verdict == HypothesisVerdict::Accept)
            .count()
    }

    pub fn n_reject(&self) -> usize {
        self.entries
            .iter()
            .filter(|h| h.verdict == HypothesisVerdict::Reject)
            .count()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "hypotheses total={} accept={} reject={} inconclusive={}",
            self.entries.len(),
            self.n_accept(),
            self.n_reject(),
            self.entries.len() - self.n_accept() - self.n_reject()
        )
    }
}

fn parse_hyp_line(line: &str) -> Option<Hypothesis> {
    let id = extract(line, "id")?;
    let track = extract(line, "track").unwrap_or_default();
    let claim = extract(line, "claim").unwrap_or_default();
    let method = extract(line, "method").unwrap_or_default();
    let predicted = extract(line, "predicted").unwrap_or_default();
    let measured = extract(line, "measured").unwrap_or_default();
    let verdict = match extract(line, "verdict").as_deref() {
        Some("ACCEPT") => HypothesisVerdict::Accept,
        Some("REJECT") => HypothesisVerdict::Reject,
        _ => HypothesisVerdict::Inconclusive,
    };
    let notes = extract(line, "notes").unwrap_or_default();
    let sealed_ns = extract(line, "ns")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    Some(Hypothesis {
        id,
        track,
        claim,
        method,
        predicted,
        measured,
        verdict,
        notes,
        sealed_ns,
    })
}

fn extract(line: &str, key: &str) -> Option<String> {
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

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_roundtrip() {
        let dir = std::env::temp_dir().join(format!("kairos_hyp_{}", now_ns()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("science")).unwrap();
        let mut reg = HypothesisRegistry::new();
        let h = Hypothesis::new("H1", "ternary", "popcount matches scalar", "bench", "agree")
            .conclude("max_delta=0", HypothesisVerdict::Accept, "ok");
        reg.record(&dir, h).unwrap();
        let reg2 = HypothesisRegistry::load(&dir).unwrap();
        assert_eq!(reg2.entries.len(), 1);
        assert_eq!(reg2.n_accept(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
