//! Minimal RAG (retrieval-augmented generation) over the repo's markdown docs.
//!
//! Indexes every *.md file under REPO_ROOT into ~1200-char chunks, embeds
//! each with Ollama's nomic-embed-text, and does in-memory cosine-similarity
//! retrieval at chat time. No vector DB — the doc set is small enough
//! (≈1MB) that a flat scan is plenty fast.

use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use walkdir::WalkDir;

const CHUNK_TARGET_CHARS: usize = 1200;
const EMBED_CONCURRENCY: usize = 4;

#[derive(Clone)]
pub struct Chunk {
    pub source: String,
    pub text: String,
    pub embedding: Vec<f32>,
}

pub struct RagIndex {
    pub chunks: Vec<Chunk>,
    pub ready: AtomicBool,
}

impl RagIndex {
    pub fn empty() -> Self {
        Self { chunks: Vec::new(), ready: AtomicBool::new(false) }
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub fn top_k(&self, query_embedding: &[f32], k: usize) -> Vec<&Chunk> {
        let mut scored: Vec<(f32, &Chunk)> = self
            .chunks
            .iter()
            .map(|c| (cosine(query_embedding, &c.embedding), c))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(k).map(|(_, c)| c).collect()
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

fn chunk_markdown(path: &Path, root: &Path) -> Vec<(String, String)> {
    let Ok(content) = std::fs::read_to_string(path) else { return Vec::new() };
    let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");

    let mut chunks = Vec::new();
    let mut buf = String::new();
    for para in content.split("\n\n") {
        if buf.len() + para.len() > CHUNK_TARGET_CHARS && !buf.is_empty() {
            chunks.push((rel.clone(), std::mem::take(&mut buf)));
        }
        if !buf.is_empty() {
            buf.push_str("\n\n");
        }
        buf.push_str(para);
    }
    if !buf.trim().is_empty() {
        chunks.push((rel.clone(), buf));
    }
    chunks
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

pub async fn embed(client: &reqwest::Client, ollama_url: &str, model: &str, text: &str) -> Result<Vec<f32>> {
    let resp: EmbedResponse = client
        .post(format!("{ollama_url}/api/embeddings"))
        .json(&json!({ "model": model, "prompt": text }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp.embedding)
}

pub async fn build_index(
    repo_root: String,
    ollama_url: String,
    embed_model: String,
    client: reqwest::Client,
    index: Arc<tokio::sync::RwLock<RagIndex>>,
) {
    let root = Path::new(&repo_root);
    let mut paths = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let s = p.to_string_lossy();
        if s.contains("/target/") || s.contains("\\target\\") || s.contains("/.git/") {
            continue;
        }
        paths.push(p.to_path_buf());
    }

    let mut raw_chunks = Vec::new();
    for p in &paths {
        raw_chunks.extend(chunk_markdown(p, root));
    }
    tracing::info!(files = paths.len(), chunks = raw_chunks.len(), "RAG: indexing markdown docs");

    let sem = Arc::new(Semaphore::new(EMBED_CONCURRENCY));
    let mut handles = Vec::new();
    for (source, text) in raw_chunks {
        let sem = sem.clone();
        let client = client.clone();
        let ollama_url = ollama_url.clone();
        let embed_model = embed_model.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire_owned().await.ok()?;
            match embed(&client, &ollama_url, &embed_model, &text).await {
                Ok(embedding) => Some(Chunk { source, text, embedding }),
                Err(e) => {
                    tracing::warn!(error = %e, "RAG: embed failed for chunk, skipping");
                    None
                }
            }
        }));
    }

    let mut chunks = Vec::new();
    for h in handles {
        if let Ok(Some(c)) = h.await {
            chunks.push(c);
        }
    }

    tracing::info!(indexed = chunks.len(), "RAG: index build complete");
    let mut guard = index.write().await;
    guard.chunks = chunks;
    guard.ready.store(true, Ordering::Relaxed);
}
