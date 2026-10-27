# 0012: Keymaster contract (`KEYMASTER_BACKEND_URL`)

**Status:** Accepted, option A (2026-10-27). See "Decision".
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
* Before this decision, `scripts/smoke-whole.sh` set `KEYMASTER_BACKEND_URL` to the keymaster service
  but only sent intents answered locally, so the smoke never exercised (or detected) the broken path.
* The dashboard's keymaster health dot probes a hard-coded `127.0.0.1:8080/health`, not
  `KEYMASTER_BACKEND_URL`. It can show "up" while every external call fails.

## Decision (2026-10-27): option A

The owner chose option A. `KEYMASTER_BACKEND_URL` is the URL of **any HTTP service that accepts
`POST {"intent": ..., "payload": ...}` and returns JSON**, for example an LLM gateway.
`backend/keymaster` is a **separate policy service that `kernel_host` does not call**; it stays in
the repo, built and tested, but nothing routes through it today. Making `kernel_host` call
`POST /route` (former option B) is capability work, not scheduled.

### Contract

| | |
|---|---|
| Request | `POST <KEYMASTER_BACKEND_URL>` (the URL is used as given, no path added), `Content-Type: application/json`, body `{"intent": <string>, "payload": <json>}` |
| Sent when | `classify`/`score`/`predict` below the confidence threshold, or any other intent, and a URL is set |
| Timeout | 10 seconds |
| Response | any JSON; returned to the caller as `result`, with `"backend": "external"` |
| Failure (refused, timeout, non-2xx, non-JSON) | `result` is `{"error": "external backend error: ...", "external": true}`, `confidence` 0.0; **no retry, no local answer**; the process keeps serving |
| Unset | no external calls; those requests are answered by `local_fallback` |

The failure behaviour is **fail-closed for the request and left as it was**: a request the local
brain cannot answer confidently returns an error payload when the backend is unreachable, for
every intent including `classify`/`score`/`predict`. The earlier suggestion to fall back to the
local answer for those three intents is a behaviour change and was not part of this decision; it
remains open.

### What changed with this decision

* `scripts/smoke-whole.sh` no longer points `KEYMASTER_BACKEND_URL` at the keymaster service. It now
  starts a small mock backend and checks the contract end to end: the request body arrives as
  `{intent, payload}`, the response comes back as `result` with `backend: "external"`, and an
  unreachable backend yields the error payload while `kernel_host` keeps answering.
* Comments in `kernel/src/main.rs`, `backend/keymaster/src/main.rs` and `types.rs`, and the
  NanoKeymaster section of `CLAUDE.md`, say the above.
* The dashboard's keymaster health dot is unchanged and still probes `127.0.0.1:8080/health`, which
  is the policy service, not the external backend. A "down" dot therefore says nothing about
  `KEYMASTER_BACKEND_URL`.
