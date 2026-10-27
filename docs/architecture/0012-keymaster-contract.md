# 0012: Keymaster contract (`KEYMASTER_BACKEND_URL`)

**Status:** Proposed (2026-10-27). Needs an owner decision; see "Decision needed".
**Phase:** Foundation Lock A5.1 ([plan](../FOUNDATION_LOCK_PLAN.md)).

## Context

Two components are both called "keymaster" and are easy to assume are one system.

| | `kernel_host` (NanoKeymaster) | `backend/keymaster` (`keymaster-backend`) |
|---|---|---|
| Transport | newline-delimited JSON on stdin/stdout | HTTP (axum): `POST /route`, `GET /health`, `GET /ready` |
| Role | Answers `classify` / `score` / `predict` locally with the ternary policy brain; sends other work to an external backend if one is configured | Stateless routing-policy service: given a policy score it returns `local`, `local_fallback` or `external` |
| Config | `KEYMASTER_BACKEND_URL` (optional) | `PORT` |

## What `kernel_host` actually does with `KEYMASTER_BACKEND_URL`

* Routing: for `classify`/`score`/`predict`, local if the score reaches the model threshold,
  otherwise external if a URL is set, otherwise `local_fallback`. Any other intent: external if a
  URL is set, otherwise `local_fallback`.
* Request: `POST` to **exactly** the configured URL (no path is added), JSON body
  `{"intent": <string>, "payload": <json>}`, 10 second timeout.
* Response: any JSON; it is returned to the caller unchanged as `result`, with `"backend": "external"`.
* Failure (connection refused, timeout, non-2xx, non-JSON): `result` becomes
  `{"error": "external backend error: ...", "external": true}` and `confidence` is `0.0`. There
  is **no retry and no local answer**; the caller gets an error payload inside a normal response
  and the process keeps running. This is fail-closed for the request, not fail-open.

## Measured: the two components do not talk to each other

Run on 2026-10-27 with `keymaster-backend` on a local port and an intent no local handler
answers (`summarize`):

| `KEYMASTER_BACKEND_URL` | Result in `kernel_host` |
|---|---|
| `http://127.0.0.1:<port>` | `status code 404` (the service has no route at `/`) |
| `http://127.0.0.1:<port>/route` | `status code 422` (body is `{intent,payload}`, the service wants a `RouteRequest`) |
| `http://127.0.0.1:1` (nothing listening) | `Connection refused` |

`POST /route` with a valid `RouteRequest` (`schema_version`, `request_id`, `agent_id`,
`context_hash`, `local_policy_score`, `estimated_tokens`, `ms_since_last_local`) works and returns a
routing decision, not an answer. The comment in `backend/keymaster/src/types.rs` saying the types
"must stay wire-compatible with the ureq POST in kernel_host" was false; this change corrects it.

Two related facts:
* `scripts/smoke-whole.sh` sets `KEYMASTER_BACKEND_URL` to the keymaster service but only sends
  intents answered locally, so the smoke never exercises (or detects) the broken path.
* The dashboard's keymaster health dot probes a hard-coded `127.0.0.1:8080/health`, not
  `KEYMASTER_BACKEND_URL`. It can show "up" while every external call fails.

## Decision needed

**A. (Recommended for now; no code.)** `KEYMASTER_BACKEND_URL` means "any HTTP backend that
accepts `{intent, payload}` and returns JSON", for example an LLM gateway. `keymaster-backend`
is a separate policy service that `kernel_host` does not call. Fix the docs and the smoke
script's misleading variable, and keep the current fail-closed behaviour for unknown intents.

**B. (Capability work, after the Lock.)** Make `kernel_host` call `POST /route` with a
`RouteRequest` built from its own score and timing, and act on the returned target. This makes
the sovereign-routing story real, but it is new behaviour and needs tests and a decision on what
happens when the policy service is down.

Either way, one smaller decision belongs to the owner: should a `classify`/`score` request that
falls below the confidence threshold and fails externally return the error payload (today) or fall
back to the local low-confidence answer? The recommendation is to fall back to local for those
three intents and keep the error for unknown intents.
