# Deploying hostframe-backend

Status: **no live deployment is verified yet.** This page is the shortest path
to one. When you have a URL, run `scripts/verify-deploy.sh <url>` and paste its
last line into [STATUS.md](STATUS.md).

`hostframe-backend`'s BigQuery client is currently a stub: `/ingest` updates
in-memory counters and does not write to BigQuery, and startup needs only
`GCP_PROJECT` set to any value. So it runs on any host, with no GCP account.
State is lost on restart.

## Railway (config is in `railway.json`)

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
