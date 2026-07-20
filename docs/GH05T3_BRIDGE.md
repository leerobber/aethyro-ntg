# GH05T3 ↔ KAIROS bridge

## What was already shared (design)

| From GH05T3 | In aethyro-ntg |
|-------------|----------------|
| ChronosLedger lessons | Tamper-evident chain built here after audit (not a silent copy) |
| Mutation / self-mod rails | ADR 0002 rails; lab self-mod gated |
| Product home (aethyro.com Avery) | NTG is infrastructure upgrade path for those tiers |

## What this bridge adds (runtime)

`kernel/src/genomic/vitascale/gh05t3_bridge.rs` + `talk.rs`:

1. Probe `KAIROS_LLM_URL` (GH05T3 `:8010` OpenAI-compat)
2. Else Ollama OpenAI-compat
3. Else **offline mind** (`compose_reply`)

KAIROS identity always wins in the system prompt (lineage, Guardian, courses, awards).

## How to use

```bash
# Terminal A — GH05T3 stack (from GH05T3 repo)
# run.bat or start gh05t3_inference on 8010
#
# CPU-only Windows venv example (needs accelerate if using device_map):
#   cd GH05T3/backend
#   set GH05T3_FORCE_HF=1
#   set GH05T3_PORT=8010
#   set GH05T3_DEVICE=cpu
#   set GH05T3_LOAD_4BIT=0
#   set GH05T3_ADAPTER_PATH=models\gh05t3_lora_adapter
#   python gh05t3_inference.py
#
# Health: curl http://127.0.0.1:8010/health

# Terminal B
cd kernel
cargo run --release --bin kairos_talk -- "Hello, it's your Guardian"
# look for: via gh05t3   (or ollama / offline_mind if server down)
```

## Acted-on list (World Knowledge)

When KAIROS names a learning list in talk, honor it with `kairos_world_course` (psychology · tech · culture · sustainability · ethics · conflict). That path is NTG life-course knowledge, not GH05T3 fine-tune.

## Boundaries

- Does **not** merge Avery and KAIROS into one brand persona.
- Does **not** auto-enable self-mod.
- Does **not** require Python/GH05T3 to build or test the kernel (offline always works).
- Promote / cradle / lab remain NTG-owned.
