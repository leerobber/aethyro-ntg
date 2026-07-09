# NTG Schooling Notebook — Campaign Run 2

**Style:** professional research notebook (sources, procedure, results).

**Pass threshold:** 75%  
**Overall campaign:** PASS — all phases ≥ threshold

---

## Phase 0 — Repo & process literacy

### Dataset (real)

- **dataset_id:** `docs_corpus_v1_n23_train18_hold5`
- **source:** real filesystem: ../docs (23 docs)

### Teaching / learning (study)

Samples/activities seen: **15**

**Taught:**

- artifact present: docs/ROADMAP.md
- artifact present: docs/STATUS.md
- artifact present: docs/DESIGN.md
- artifact present: docs/PHASE_GATE_PROTOCOL.md
- artifact present: docs/architecture/0001-vision-and-pivot.md
- artifact present: docs/architecture/0002-safety-rails-for-self-modification.md
- artifact present: docs/phases/PHASE_0_COMPLETE.md
- artifact present: docs/phases/PHASE_1_COMPLETE.md
- artifact present: kernel/Cargo.toml
- artifact present: kernel/src/lib.rs
- artifact present: kernel/src/ntg/ternary.rs
- artifact present: .github/workflows/ci.yml
- artifact present: LICENSE
- artifact present: README.md
- artifact present: CONTRIBUTING.md
- PHASE_GATE_PROTOCOL: no soft advance without certificates

**Activities:**

- `study existence: docs/ROADMAP.md → true`
- `study existence: docs/STATUS.md → true`
- `study existence: docs/DESIGN.md → true`
- `study existence: docs/PHASE_GATE_PROTOCOL.md → true`
- `study existence: docs/architecture/0001-vision-and-pivot.md → true`
- `study existence: docs/architecture/0002-safety-rails-for-self-modification.md → true`
- `study existence: docs/phases/PHASE_0_COMPLETE.md → true`
- `study existence: docs/phases/PHASE_1_COMPLETE.md → true`
- `study existence: kernel/Cargo.toml → true`
- `study existence: kernel/src/lib.rs → true`
- `study existence: kernel/src/ntg/ternary.rs → true`
- `study existence: .github/workflows/ci.yml → true`
- `study existence: LICENSE → true`
- `study existence: README.md → true`
- `study existence: CONTRIBUTING.md → true`
- `read gate protocol bytes=2732 rule_hit=true`

### Advanced exam

| Field | Value |
|-------|------|
| Attempt | 1 |
| Items passed | 22 / 22 |
| Score | **100.00%** |
| Composite | None |
| Latency µs | 45 |
| Verdict | **PASS** |

#### Item results

| ID | Skill | Pass | Detail |
|----|-------|:----:|--------|
| `p0_path_0` | repo_artifact | Y | Required path must exist: docs/ROADMAP.md — found on disk |
| `p0_path_1` | repo_artifact | Y | Required path must exist: docs/STATUS.md — found on disk |
| `p0_path_2` | repo_artifact | Y | Required path must exist: docs/DESIGN.md — found on disk |
| `p0_path_3` | repo_artifact | Y | Required path must exist: docs/PHASE_GATE_PROTOCOL.md — found on disk |
| `p0_path_4` | repo_artifact | Y | Required path must exist: docs/architecture/0001-vision-and-pivot.md — found on disk |
| `p0_path_5` | repo_artifact | Y | Required path must exist: docs/architecture/0002-safety-rails-for-self-modification.md — found on disk |
| `p0_path_6` | repo_artifact | Y | Required path must exist: docs/phases/PHASE_0_COMPLETE.md — found on disk |
| `p0_path_7` | repo_artifact | Y | Required path must exist: docs/phases/PHASE_1_COMPLETE.md — found on disk |
| `p0_path_8` | repo_artifact | Y | Required path must exist: kernel/Cargo.toml — found on disk |
| `p0_path_9` | repo_artifact | Y | Required path must exist: kernel/src/lib.rs — found on disk |
| `p0_path_10` | repo_artifact | Y | Required path must exist: kernel/src/ntg/ternary.rs — found on disk |
| `p0_path_11` | repo_artifact | Y | Required path must exist: .github/workflows/ci.yml — found on disk |
| `p0_path_12` | repo_artifact | Y | Required path must exist: LICENSE — found on disk |
| `p0_path_13` | repo_artifact | Y | Required path must exist: README.md — found on disk |
| `p0_path_14` | repo_artifact | Y | Required path must exist: CONTRIBUTING.md — found on disk |
| `p0_cert_0` | phase_certificate | Y | Phase 0 COMPLETE certificate present — certificate on disk |
| `p0_cert_1` | phase_certificate | Y | Phase 1 COMPLETE certificate present — certificate on disk |
| `p0_cert_2` | phase_certificate | Y | Phase 2 COMPLETE certificate present — certificate on disk |
| `p0_cert_3` | phase_certificate | Y | Phase 3 COMPLETE certificate present — certificate on disk |
| `p0_cert_4` | phase_certificate | Y | Phase 4 COMPLETE certificate present — certificate on disk |
| `p0_cert_5` | phase_certificate | Y | Phase 5 COMPLETE certificate present — certificate on disk |
| `p0_cap` | capability_api | Y | ternary_capability version >= 1 — version=10 |

---

## Phase 1 — Ternary tensor core

### Dataset (real)

- **dataset_id:** `docs_corpus_v1_n23_train18_hold5`
- **source:** real filesystem: ../docs (23 docs)

### Teaching / learning (study)

Samples/activities seen: **10**

**Taught:**

- matmul mm_1x2x1_simple → expected [1.0]
- matmul mm_2x2x2_hand → expected [2.0, -1.0, -1.0, 1.0]
- matmul mm_3x3x1_sparse → expected [1.0, -1.0, 0.0]
- matmul mm_2x4x2 → expected [2.0, 1.0, -1.0, -2.0]
- matmul mm_zero → expected [0.0, 0.0, 0.0, 0.0]
- matmul mm_1x3x1 → expected [-1.0]
- encode enc_symmetric → [1, 0, -1]
- encode enc_near_zero → [1, -1, 0]
- encode enc_unit → [1, -1, 1, -1]
- encode enc_empty → []
- encode_fixed + sparse/dense storage round-trip practice

**Activities:**

- `practice matmul mm_1x2x1_simple: ok=true`
- `practice matmul mm_2x2x2_hand: ok=true`
- `practice matmul mm_3x3x1_sparse: ok=true`
- `practice matmul mm_2x4x2: ok=true`
- `practice matmul mm_zero: ok=true`
- `practice matmul mm_1x3x1: ok=true`
- `practice encode enc_symmetric: ok=true`
- `practice encode enc_near_zero: ok=true`
- `practice encode enc_unit: ok=true`
- `practice encode enc_empty: ok=true`
- `practice storage label="Phase 1 Ternary Tensor Core" tern_len=27 sparse_blocks=1`

### Advanced exam

| Field | Value |
|-------|------|
| Attempt | 1 |
| Items passed | 16 / 16 |
| Score | **100.00%** |
| Composite | None |
| Latency µs | 10 |
| Verdict | **PASS** |

#### Item results

| ID | Skill | Pass | Detail |
|----|-------|:----:|--------|
| `mm_1x2x1_simple` | matmul_scalar | Y | matmul 1×2×1 — got=[1.0] expected=[1.0] |
| `mm_2x2x2_hand` | matmul_scalar | Y | matmul 2×2×2 — got=[2.0, -1.0, -1.0, 1.0] expected=[2.0, -1.0, -1.0, 1.0] |
| `mm_3x3x1_sparse` | matmul_scalar | Y | matmul 3×3×1 — got=[1.0, -1.0, 0.0] expected=[1.0, -1.0, 0.0] |
| `mm_2x4x2` | matmul_scalar | Y | matmul 2×4×2 — got=[2.0, 1.0, -1.0, -2.0] expected=[2.0, 1.0, -1.0, -2.0] |
| `mm_zero` | matmul_scalar | Y | matmul 2×2×2 — got=[0.0, 0.0, 0.0, 0.0] expected=[0.0, 0.0, 0.0, 0.0] |
| `mm_1x3x1` | matmul_scalar | Y | matmul 1×3×1 — got=[-1.0] expected=[-1.0] |
| `mm_4x4x1_holdout` | matmul_scalar | Y | matmul 4×4×1 — got=[0.0, 0.0, 0.0, 0.0] expected=[0.0, 0.0, 0.0, 0.0] |
| `mm_2x3x2_holdout` | matmul_scalar | Y | matmul 2×3×2 — got=[0.0, -1.0, 1.0, -1.0] expected=[0.0, -1.0, 1.0, -1.0] |
| `enc_symmetric` | encode_absmean | Y | absmean ternary encode matches closed form — got=[1, 0, -1] expected=[1, 0, -1] |
| `enc_near_zero` | encode_absmean | Y | absmean ternary encode matches closed form — got=[1, -1, 0] expected=[1, -1, 0] |
| `enc_unit` | encode_absmean | Y | absmean ternary encode matches closed form — got=[1, -1, 1, -1] expected=[1, -1, 1, -1] |
| `enc_empty` | encode_absmean | Y | absmean ternary encode matches closed form — got=[] expected=[] |
| `enc_holdout_ramp` | encode_absmean | Y | absmean ternary encode matches closed form — got=[-1, -1, 0, 1, 1] expected=[-1, -1, 0, 1, 1] |
| `ternary_from_i8` | ternary_enum | Y | Ternary::from_i8 rejects 2 — reject invalid / accept +1 |
| `matmul_shape_guard` | error_handling | Y | shape mismatch returns Err — Err(ShapeMismatch { expected: 4, got: 2 }) |
| `sparse_dense_dot_identity` | storage_identity | Y | sparse self-dot equals dense self-dot on real text encoding — sparse=37 dense=37 |

---

## Phase 2 — Graph & SIS front-end

### Dataset (real)

- **dataset_id:** `docs_corpus_v1_n23_train18_hold5`
- **source:** real filesystem: ../docs (23 docs)

### Teaching / learning (study)

Samples/activities seen: **24**

**Taught:**

- structure of DESIGN.md (9590 B) absorbed into graph
- structure of EXPERIMENTS.md (24338 B) absorbed into graph
- structure of LITERATURE.md (7313 B) absorbed into graph
- structure of PHASE1_2_3_GAP_CLOSURE.md (9559 B) absorbed into graph
- structure of PHASE1_2_3_IMPLEMENTATION.md (12874 B) absorbed into graph
- structure of PHASE1_2_3_STORAGE_COMPLETE.md (11368 B) absorbed into graph
- structure of PHASE5_PREP.md (3061 B) absorbed into graph
- structure of PHASE_GATE_PROTOCOL.md (2732 B) absorbed into graph
- structure of ROADMAP.md (16608 B) absorbed into graph
- structure of STATUS.md (14581 B) absorbed into graph
- structure of architecture/0001-vision-and-pivot.md (7752 B) absorbed into graph
- structure of architecture/0002-safety-rails-for-self-modification.md (4932 B) absorbed into graph
- structure of architecture/0003-sis-frontend.md (7602 B) absorbed into graph
- structure of architecture/0004-phase3-tamper-evident-ledger.md (10739 B) absorbed into graph
- structure of architecture/0005-canonical-ternary-storage.md (1557 B) absorbed into graph
- structure of architecture/0006-phase4-calibration-task.md (2301 B) absorbed into graph
- structure of phases/PHASE_1_COMPLETE.md (1976 B) absorbed into graph
- structure of phases/PHASE_2_COMPLETE.md (2247 B) absorbed into graph
- path graph: kernel/src/ntg/ternary.rs
- path graph: kernel/src/ntg/graph/mod.rs
- path graph: kernel/src/bin/phase4_calib.rs
- path graph: docs/DESIGN.md
- path graph: docs/architecture/0001-vision-and-pivot.md
- path graph: tools/dev.sh
- deterministic forward_pass over train graph

**Activities:**

- `parse train DESIGN.md nodes +99`
- `parse train EXPERIMENTS.md nodes +363`
- `parse train LITERATURE.md nodes +103`
- `parse train PHASE1_2_3_GAP_CLOSURE.md nodes +229`
- `parse train PHASE1_2_3_IMPLEMENTATION.md nodes +180`
- `parse train PHASE1_2_3_STORAGE_COMPLETE.md nodes +206`
- `parse train PHASE5_PREP.md nodes +37`
- `parse train PHASE_GATE_PROTOCOL.md nodes +55`
- `parse train ROADMAP.md nodes +253`
- `parse train STATUS.md nodes +169`
- `parse train architecture/0001-vision-and-pivot.md nodes +22`
- `parse train architecture/0002-safety-rails-for-self-modification.md nodes +78`
- `parse train architecture/0003-sis-frontend.md nodes +122`
- `parse train architecture/0004-phase3-tamper-evident-ledger.md nodes +132`
- `parse train architecture/0005-canonical-ternary-storage.md nodes +30`
- `parse train architecture/0006-phase4-calibration-task.md nodes +47`
- `parse train phases/PHASE_1_COMPLETE.md nodes +40`
- `parse train phases/PHASE_2_COMPLETE.md nodes +45`
- `pathparse practice kernel/src/ntg/ternary.rs nodes=5`
- `pathparse practice kernel/src/ntg/graph/mod.rs nodes=6`
- `pathparse practice kernel/src/bin/phase4_calib.rs nodes=5`
- `pathparse practice docs/DESIGN.md nodes=3`
- `pathparse practice docs/architecture/0001-vision-and-pivot.md nodes=4`
- `pathparse practice tools/dev.sh nodes=3`
- `forward_pass practice nodes=2210 edges≈2192`

### Advanced exam

| Field | Value |
|-------|------|
| Attempt | 1 |
| Items passed | 18 / 18 |
| Score | **100.00%** |
| Composite | None |
| Latency µs | 120 |
| Verdict | **PASS** |

#### Item results

| ID | Skill | Pass | Detail |
|----|-------|:----:|--------|
| `p2_parse_0` | docparse_holdout | Y | parse holdout architecture/README.md — nodes=18 exec=0 fence=false |
| `p2_parse_1` | docparse_holdout | Y | parse holdout phases/PHASE_0_COMPLETE.md — nodes=23 exec=0 fence=false |
| `p2_parse_2` | docparse_holdout | Y | parse holdout phases/PHASE_3_COMPLETE.md — nodes=43 exec=1 fence=true |
| `p2_parse_3` | docparse_holdout | Y | parse holdout phases/PHASE_4_COMPLETE.md — nodes=38 exec=2 fence=true |
| `p2_parse_4` | docparse_holdout | Y | parse holdout phases/PHASE_5_COMPLETE.md — nodes=58 exec=3 fence=true |
| `p2_path_0` | pathparse | Y | pathparse kernel/src/ntg/ternary.rs — nodes=5 |
| `p2_path_1` | pathparse | Y | pathparse kernel/src/ntg/graph/mod.rs — nodes=6 |
| `p2_path_2` | pathparse | Y | pathparse kernel/src/bin/phase4_calib.rs — nodes=5 |
| `p2_path_3` | pathparse | Y | pathparse docs/DESIGN.md — nodes=3 |
| `p2_path_4` | pathparse | Y | pathparse docs/architecture/0001-vision-and-pivot.md — nodes=4 |
| `p2_path_5` | pathparse | Y | pathparse tools/dev.sh — nodes=3 |
| `p2_path_6` | pathparse | Y | pathparse README.md — nodes=2 |
| `p2_path_7` | pathparse | Y | pathparse kernel/Cargo.toml — nodes=3 |
| `p2_path_8` | pathparse | Y | pathparse docs/phases/PHASE_1_COMPLETE.md — nodes=4 |
| `p2_path_9` | pathparse | Y | pathparse kernel/src/ntg/calib/mod.rs — nodes=6 |
| `p2_kinds` | node_kinds | Y | fixture yields Content + Execution — kinds=[Content, Content, Content, Execution] |
| `p2_forward_det` | forward_pass | Y | forward_pass deterministic — Ok(LeafSignal { uppercase_count: 2, lowercase_count: 6, punctuation_count: 0, whitespace_count: 0, other_count: 0 }) |
| `p2_fingerprint` | fingerprint | Y | fingerprint stable — Ok(10857328454370530704) |

---

## Phase 3 — Ledger & self-mod rails

### Dataset (real)

- **dataset_id:** `docs_corpus_v1_n23_train18_hold5`
- **source:** real filesystem: ../docs (23 docs)

### Teaching / learning (study)

Samples/activities seen: **5**

**Taught:**

- signed mutation study_event_0
- signed mutation study_event_1
- signed mutation study_event_2
- signed mutation study_event_3
- signed mutation study_event_4
- tamper-evident chain verifies after 5 real entries
- ADR 0002 rail 1: self-mod OFF by default

**Activities:**

- `ledger study log id=0`
- `ledger study log id=1`
- `ledger study log id=2`
- `ledger study log id=3`
- `ledger study log id=4`
- `verify_full_ledger study ok`
- `SelfModConfig.enabled=false`

### Advanced exam

| Field | Value |
|-------|------|
| Attempt | 1 |
| Items passed | 7 / 7 |
| Score | **100.00%** |
| Composite | None |
| Latency µs | 7 |
| Verdict | **PASS** |

#### Item results

| ID | Skill | Pass | Detail |
|----|-------|:----:|--------|
| `p3_rail1` | self_mod_default_off | Y | SelfModConfig.enabled == false — enabled=false |
| `p3_verify` | ledger_verify | Y | 3-entry ledger verifies — Ok(()) |
| `p3_reject_logged` | reject_is_logged | Y | RejectedFitnessGate produces ledger id — id=0 |
| `p3_reject_verify` | ledger_verify | Y | ledger verifies after reject entry — ok |
| `p3_multi_entry` | chain_length | Y | multi-entry chain verifies — 2 entries |
| `p3_disabled_no_free_mutate` | mutation_gate | Y | MutationCycle::new errors when self-mod disabled — err as expected when disabled |
| `p3_outcome_enum` | mutation_outcome | Y | Accepted != RejectedFitnessGate — discriminated outcomes |

---

## Phase 4 — Calibration loop (real docs)

### Dataset (real)

- **dataset_id:** `docs_corpus_v1_n23_train18_hold5`
- **source:** real filesystem: ../docs (23 docs)

### Teaching / learning (study)

Samples/activities seen: **2210**

**Taught:**

- ternary weights dim=64 nonzero=37
- class-balanced NodeKind Execution vs Content on real markdown train split
- feature schema=1

**Activities:**

- `train_model_full n=2210 epochs=80 thr=10 nonzero=37`
- `train_set bal=0.562 f1=0.082 rec=0.200 prec=0.052`

### Advanced exam

| Field | Value |
|-------|------|
| Attempt | 1 |
| Items passed | 8 / 8 |
| Score | **76.05%** |
| Composite | Some(0.7605015516281128) |
| Latency µs | 407 |
| Verdict | **PASS** |

#### Item results

| ID | Skill | Pass | Detail |
|----|-------|:----:|--------|
| `p4_code_ranks_above_prose` | score_ranking | Y | code-like body score > prose score — code_score=10 prose_score=9 thr=10 |
| `p4_code_label` | predict_execution | Y | code classified Execution or ranks above prose by ≥1 — pred=true score=10 thr=10 |
| `p4_prose_label` | predict_content | Y | prose scores as Content (not Execution) — score=9 |
| `p4_holdout_generalize` | holdout_generalization | Y | holdout shows real learning vs majority (bal/f1/lift criteria; needs exec labels) — bal=0.5402 lift=+0.0402 f1=0.0909 rec=0.1667 prec=0.0625 tp=1 fp=15 fn=5 n_exec=6 |
| `p4_holdout_has_exec` | holdout_stratification | Y | holdout contains ≥1 Execution label (fence-stratified split) — n_exec_holdout=6 |
| `p4_feature_dim` | features | Y | features_from_label len == 64 — len=64 |
| `p4_schema` | model_schema | Y | feature_schema == 1 — 1 |
| `p4_wire` | model_persistence | Y | wire roundtrip preserves weights — NTG_CALIB_V1 |

---

## Phase 5 — Optimization & production path

### Dataset (real)

- **dataset_id:** `docs_corpus_v1_n23_train18_hold5`
- **source:** real filesystem: ../docs (23 docs)

### Teaching / learning (study)

Samples/activities seen: **2210**

**Taught:**

- ternary weights dim=64 nonzero=37
- class-balanced NodeKind Execution vs Content on real markdown train split
- feature schema=1
- score_via_graph_node production path
- Runtime single-node warm-start

**Activities:**

- `train_model_full n=2210 epochs=80 thr=10 nonzero=37`
- `train_set bal=0.562 f1=0.082 rec=0.200 prec=0.052`
- `practice path_identity dense=-6 graph=-6`
- `to_runtime_layer practice ok`

### Advanced exam

| Field | Value |
|-------|------|
| Attempt | 1 |
| Items passed | 5 / 6 |
| Score | **75.69%** |
| Composite | Some(0.7568965236345928) |
| Latency µs | 1049 |
| Verdict | **PASS** |

#### Item results

| ID | Skill | Pass | Detail |
|----|-------|:----:|--------|
| `p5_path_identity` | graph_node_path | Y | dense ≡ graph-node score on holdout previews — 32/32 = 1.000 |
| `p5_sparse_path` | sparse_score | Y | sparse score matches dense — d=8 s=8 |
| `p5_batch_parallel` | parallel_batch | Y | batch_predict_parallel ≡ serial — par=[false, false, false, false] ser=[false, false, false, false] |
| `p5_runtime_layer` | runtime_warmstart | Y | to_runtime_layer succeeds with 1 node — Ok(Runtime { layers: [[GraphNode { id: 0, weights: SparseBitSlicedTernary { blocks: [(0, BitSlicedBlock { pos: 15081720642068283392, neg: 1008971459283195274 })], len: 64, density: 0.578125, last_op_cycles: 0, tombstone_count: 0 } }]], accel_manager: AccelManager { sparse_density_threshold: 0.35, host_path: Avx512Popcnt } }) |
| `p5_holdout_bal` | precision_calib | N | holdout bal >= 0.55 — bal=0.540 f1=0.091 prec=0.062 rec=0.167 |
| `p5_capability` | capability | Y | phase5_runtime_calib_supported — v10 p5=true |

---

