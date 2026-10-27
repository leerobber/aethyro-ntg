# Deploying hostframe-backend

Status: **one live deployment, verified from outside on 2026-10-27.**

| | |
|---|---|
| Target | Render, free web service `aethyro-hostframe` (Docker runtime, Oregon) |
| URL | https://aethyro-hostframe.onrender.com |
| Commit deployed | `6b3b7e0` (main) |
| Verified | 2026-10-27T00:25:03Z by `scripts/verify-deploy.sh` run from a machine outside Render: `GET /health`, `POST /ingest`, `GET /metrics` all PASS |
| Deploy settings | Docker context `hostframe-backend`, Dockerfile `hostframe-backend/Dockerfile`, env `GCP_PROJECT=aethyro-ntg`, auto-deploy off (redeploy by hand) |

This verifies `hostframe-backend` only (the telemetry-ingest stub, see
[ADR 0013](architecture/0013-telemetry-loop.md)); not the NTG engine, the
dashboard or `kernel_host`. Free instances sleep when idle, so the first request
after a pause is slow; hit `/health` and wait before running the verify script.
State is in memory and is lost on restart or sleep.

To verify a deployment, run `scripts/verify-deploy.sh <url>` and paste its last
line into [STATUS.md](STATUS.md).

`hostframe-backend`'s BigQuery client is currently a stub: `/ingest` updates
in-memory counters and does not write to BigQuery, and startup needs only
`GCP_PROJECT` set to any value. So it runs on any host, with no GCP account.
State is lost on restart.

## Build context (read this before deploying anywhere)

`hostframe-backend/Dockerfile` does `COPY . .` and then `cargo build --release`, so the
Docker **build context must be the `hostframe-backend/` directory** (that is how the Render
deploy above is configured, and how `fly.toml` is set up). With the repo root as context the
build would compile the root workspace (the `kernel` crate) and the final `COPY --from=builder`
of `hostframe-backend` would have nothing to copy. This is reasoned from the Dockerfile, not
reproduced. `railway.json` uses the repo root and has never been deployed (the Railway trial
ended), so treat it as unverified; on Railway, set the service's Root Directory to
`hostframe-backend` and check the first build.

## Railway (config is in `railway.json`; never deployed, unverified)

1. New project → Deploy from GitHub repo → `leerobber/aethyro-ntg`.
2. Variables: `GCP_PROJECT=aethyro-ntg` (any non-empty value). `PORT` is set by
   Railway; the service reads it.
3. The config builds `hostframe-backend/Dockerfile` and health-checks `/health`.
4. Settings → Networking → Generate domain.
5. `scripts/verify-deploy.sh https://<your-domain>`

## Fly (config is in `fly.toml`)

```bash
fly launch --no-deploy --copy-config   # keeps the repo's fly.toml
fly secrets set GCP_PROJECT=aethyro-ntg
fly deploy
scripts/verify-deploy.sh https://aethyro-ntg.fly.dev
```

## What this does not cover

- The dashboard (`/api/*`) and `kernel_host` are not part of this image. The
  dashboard runs next to `kernel_host` (see `scripts/smoke-whole.sh`).
- Real BigQuery ingestion and GCP/Cloud Run (`deploy.yml`, manual dispatch;
  needs billing and four repository secrets).
