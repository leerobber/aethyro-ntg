//! Aethyro Dashboard — HTTP bridge + installable web control panel for kernel_host.
//!
//! kernel_host speaks newline-delimited JSON over stdin/stdout. This process
//! spawns it once, keeps it alive, and exposes it as a small REST API so a
//! browser (desktop or mobile, installed as a PWA) can drive it without a
//! terminal. It also runs a small RAG pipeline over the repo's markdown docs
//! (via Ollama + nomic-embed-text) so /api/chat can ground answers from a
//! local sovereign model (e.g. gh05t3-sovereign) in the actual project docs.

mod rag;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use rag::RagIndex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, RwLock};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

struct KernelHost {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl KernelHost {
    async fn spawn() -> anyhow::Result<Self> {
        let bin = std::env::var("KERNEL_HOST_BIN").unwrap_or_else(|_| "kernel_host".into());
        let mut cmd = Command::new(&bin);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Ok(url) = std::env::var("KEYMASTER_BACKEND_URL") {
            cmd.env("KEYMASTER_BACKEND_URL", url);
        }
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    /// Send one request line, skip any non-JSON banner/log noise, return the first valid JSON reply.
    async fn request(&mut self, req: &Value) -> anyhow::Result<Value> {
        let mut line = serde_json::to_string(req)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        loop {
            let mut buf = String::new();
            let n = self.stdout.read_line(&mut buf).await?;
            if n == 0 {
                anyhow::bail!("kernel_host closed stdout (process exited)");
            }
            let trimmed = buf.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                return Ok(v);
            }
            // not JSON — startup banner / log noise, skip it
        }
    }

    fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

/// Optional so the HTTP server can bind even when kernel_host is not yet available.
type SharedKernel = Arc<Mutex<Option<KernelHost>>>;

#[derive(Clone)]
struct AppState {
    kernel: SharedKernel,
    rag: Arc<RwLock<RagIndex>>,
    http: reqwest::Client,
    ollama_url: String,
    chat_model: String,
    embed_model: String,
}

async fn ensure_alive(state: &SharedKernel) -> anyhow::Result<()> {
    let mut guard = state.lock().await;
    let needs_spawn = match guard.as_mut() {
        None => true,
        Some(k) => !k.alive(),
    };
    if needs_spawn {
        *guard = Some(KernelHost::spawn().await?);
    }
    Ok(())
}

async fn call_intent(
    state: &SharedKernel,
    req: Value,
) -> Result<Json<Value>, (StatusCode, String)> {
    ensure_alive(state)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut guard = state.lock().await;
    let kernel = guard
        .as_mut()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "kernel_host not running".into()))?;
    match kernel.request(&req).await {
        Ok(v) => Ok(Json(v)),
        Err(e) => {
            // Drop dead child so next call re-spawns
            *guard = None;
            Err((StatusCode::BAD_GATEWAY, e.to_string()))
        }
    }
}

#[derive(Deserialize)]
struct TextBody {
    text: String,
}

#[derive(Deserialize)]
struct ChatBody {
    message: String,
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    call_intent(&state.kernel, json!({ "intent": "_status" })).await
}

async fn verify_ledger(State(state): State<AppState>) -> impl IntoResponse {
    call_intent(&state.kernel, json!({ "intent": "_verify_ledger" })).await
}

async fn classify(State(state): State<AppState>, Json(body): Json<TextBody>) -> impl IntoResponse {
    call_intent(
        &state.kernel,
        json!({ "intent": "classify", "payload": { "text": body.text } }),
    )
    .await
}

async fn score(State(state): State<AppState>, Json(body): Json<TextBody>) -> impl IntoResponse {
    call_intent(
        &state.kernel,
        json!({ "intent": "score", "payload": { "text": body.text } }),
    )
    .await
}

#[derive(Deserialize)]
struct OllamaChatMessage {
    content: String,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaChatMessage,
}

async fn chat(State(state): State<AppState>, Json(body): Json<ChatBody>) -> impl IntoResponse {
    let query_embedding =
        match rag::embed(&state.http, &state.ollama_url, &state.embed_model, &body.message).await {
            Ok(e) => e,
            Err(e) => return Err((StatusCode::BAD_GATEWAY, format!("embedding failed: {e}"))),
        };

    let (context, sources, index_ready) = {
        let idx = state.rag.read().await;
        let top = idx.top_k(&query_embedding, 8);
        let context = top
            .iter()
            .map(|c| format!("### {}\n{}", c.source, c.text))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        let sources: Vec<String> = top.iter().map(|c| c.source.clone()).collect();
        (context, sources, idx.is_ready())
    };

    let system_prompt = format!(
        "You are the assistant embedded in the aethyro-ntg project's local control panel. \
         Answer the user's question about the project accurately and concisely, grounded in \
         the context below when it's relevant. If the context doesn't cover something, say so \
         plainly instead of guessing.\n\n{}",
        if context.is_empty() {
            "(no matching project context found)".to_string()
        } else {
            context
        }
    );

    let resp = state
        .http
        .post(format!("{}/api/chat", state.ollama_url))
        .json(&json!({
            "model": state.chat_model,
            "stream": false,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": body.message },
            ]
        }))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return Err((StatusCode::BAD_GATEWAY, format!("ollama unreachable: {e}"))),
    };

    let parsed: Result<OllamaChatResponse, _> = resp.json().await;
    match parsed {
        Ok(v) => Ok(Json(json!({
            "reply": v.message.content,
            "sources": sources,
            "index_ready": index_ready,
        }))),
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            format!("ollama response parse error: {e}"),
        )),
    }
}

/// Best-effort raw HTTP GET against a local backend, used to report its liveness.
async fn probe_http(addr: &str, path: &str) -> bool {
    let Ok(mut stream) = TcpStream::connect(addr).await else {
        return false;
    };
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n",
        path = path,
        addr = addr
    );
    if stream.write_all(req.as_bytes()).await.is_err() {
        return false;
    }
    let mut buf = Vec::new();
    use tokio::io::AsyncReadExt;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), stream.read_to_end(&mut buf)).await;
    String::from_utf8_lossy(&buf).starts_with("HTTP/1.1 200")
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let kernel_alive = {
        let mut guard = state.kernel.lock().await;
        match guard.as_mut() {
            Some(k) => k.alive(),
            None => false,
        }
    };
    let keymaster_backend_up = probe_http("127.0.0.1:8080", "/health").await;
    let (rag_ready, rag_chunks) = {
        let idx = state.rag.read().await;
        (idx.is_ready(), idx.chunks.len())
    };
    let ollama_addr = state
        .ollama_url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_string();
    let ollama_up = probe_http(&ollama_addr, "/api/tags").await;
    Json(json!({
        "kernel_host": if kernel_alive { "up" } else { "down" },
        "keymaster_backend": if keymaster_backend_up { "up" } else { "down" },
        "ollama": if ollama_up { "up" } else { "down" },
        "rag_index_ready": rag_ready,
        "rag_chunks": rag_chunks,
        "dashboard": "up",
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Best-effort spawn: dashboard must still bind so PWA / health work without kernel.
    let kernel: SharedKernel = Arc::new(Mutex::new(match KernelHost::spawn().await {
        Ok(k) => {
            tracing::info!("kernel_host spawned");
            Some(k)
        }
        Err(e) => {
            tracing::warn!(error = %e, "kernel_host spawn failed; will retry on API calls");
            None
        }
    }));

    let ollama_url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".into());
    let chat_model =
        std::env::var("OLLAMA_CHAT_MODEL").unwrap_or_else(|_| "gh05t3-sovereign".into());
    let embed_model =
        std::env::var("OLLAMA_EMBED_MODEL").unwrap_or_else(|_| "nomic-embed-text".into());
    let repo_root = std::env::var("REPO_ROOT").unwrap_or_else(|_| ".".into());
    let http = reqwest::Client::new();
    let rag_index = Arc::new(RwLock::new(RagIndex::empty()));

    tokio::spawn(rag::build_index(
        repo_root,
        ollama_url.clone(),
        embed_model.clone(),
        http.clone(),
        rag_index.clone(),
    ));

    let state = AppState {
        kernel,
        rag: rag_index,
        http,
        ollama_url,
        chat_model,
        embed_model,
    };

    let static_dir = std::env::var("DASHBOARD_STATIC_DIR").unwrap_or_else(|_| "static".into());

    let app = Router::new()
        .route("/api/status", get(status))
        .route("/api/verify_ledger", get(verify_ledger))
        .route("/api/classify", post(classify))
        .route("/api/score", post(score))
        .route("/api/chat", post(chat))
        .route("/api/health", get(health))
        .fallback_service(ServeDir::new(&static_dir))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, static_dir, "aethyro-dashboard listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
