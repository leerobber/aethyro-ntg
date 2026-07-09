# 0003: SIS front-end — parsing documents, paths, and glyphs into the NTG graph

**Status:** Phase 2 SIS **implemented** (2026-07-09) with honest scope.
Docs/path/fs-event pure layer, LeafSignal, LazyLeaf body resolution,
GlyphFingerprint **v0** (deterministic shape-class proxy — **not** trained
PIXEL), bytemerge cost mitigation, and `Graph::log_execution_nodes` are
real and tested. Trained PIXEL-lite and OS `notify` watching remain
**explicitly deferred** (see PHASE_2_COMPLETE).

## Context

The founder asked for a front-end where documents parse directly into
execution ops, filesystem/graph paths are treated as intelligence graphs,
and punctuation, case, and glyph geometry are mapped into first-class
primitives — with the explicit design constraint that no signal is
discarded as "just formatting." This ADR treats that as a real
architecture problem to solve, not a rhetorical goal, and grounds it in
existing research rather than inventing from nothing (see
[LITERATURE.md](../LITERATURE.md) for full citations).

Four real, separately-proven threads are relevant:

- **PIXEL** (ICLR 2023): text rendered as images, processed as visual
  patches — proves glyph geometry can be a legitimate model substrate,
  not just a display detail. Documented weakness: generation/reconstruction
  from patches is hard, which matters here because this front-end must
  support *execution*, not just recognition.
- **ByT5 / CANINE**: tokenizer-free, byte/codepoint-level models. Case,
  punctuation, and whitespace are never normalized away, because there is
  no subword vocabulary to normalize them out of. Documented cost: 4-6x
  longer sequences, slower inference/training than subword tokenization.
- **MrT5**: dynamic token merging built specifically to reduce that
  byte-level cost.
- **GraphMD / "Literate Execution"** (2026): Markdown documents parsed
  directly into executable knowledge graphs — headings/sections become
  graph nodes, fenced code blocks become executable ops, references
  become edges, with a bridge to RDF/OWL for formal semantics. This is
  close to exactly "docs parsed into execution ops, paths as graphs" —
  it is prior art, not something this project invented, and the design
  below builds on it rather than re-deriving it from scratch.

## Decision

**Structural-first, content-lazy design**, chosen over two rejected
alternatives:

- *Rejected: pure visual substrate* (render everything as images,
  PIXEL-only). Fails the execution requirement — cannot tolerate
  patch-reconstruction uncertainty in something meant to actually run.
- *Rejected: byte-level-graph-as-bolt-on* (raw bytes tagged with
  structural metadata, treated as a separate system feeding NTG). Works,
  but treats document structure as metadata on a sequence rather than as
  the primary computation, and duplicates work Phase 2 already does.

**Adopted:**

1. **Documents and paths parse into the same typed graph Phase 2 already
   builds** — not a separate front-end representation. Headings,
   sections, fenced code/command blocks, and path segments become graph
   nodes; containment, reference, and execution-dependency become edges.
   This reuses GraphMD's actual mechanism (a fenced code block is an
   executable-typed node) rather than inventing a new one.
2. **Byte/glyph-level detail is lazy, not always-materialized.** A leaf
   node's raw content is only resolved to the byte level when it's
   actually read or executed. When it is, encoding is byte-exact
   (ByT5/CANINE-style — case, punctuation, whitespace preserved, nothing
   normalized away).
3. **Glyph geometry is a precomputed per-symbol fingerprint, not
   full-page rendering.** Each distinct glyph/codepoint gets a small,
   frozen visual descriptor computed once (a PIXEL-lite fingerprint),
   attached alongside its byte identity — so genuine geometric signal is
   available without paying PIXEL's full page-rendering cost or its
   generation-side weakness.
4. **"SIS primitive" is defined here as: the fused unit at a leaf
   node — (byte/codepoint identity, precomputed glyph-geometry
   fingerprint, position in the containing graph).** This term wasn't
   defined when requested; this is the working definition until
   corrected. If "SIS" referred to something specific from elsewhere,
   this ADR needs a follow-up correction.
5. **Execution nodes are ledger-logged exactly like Phase 3's
   self-modification events.** Running an embedded command/op gets the
   same audit rigor (ADR 0002 rails) as the graph mutating its own
   topology — both are "the system doing something consequential," and
   both get the same tamper-evident record.
6. **The known byte-level cost (4-6x sequence length) is mitigated, not
   ignored.** MrT5-style dynamic merging applies at the byte layer before
   this is claimed production-viable — this is a real, documented tradeoff
   that a design document doesn't get to wave away.

## Consequences

- This is now in-scope for Phase 2 (graph structure), not a separate
  phase — ROADMAP.md should reflect that the graph structure work
  includes typed nodes (execution vs. plain content) and the lazy
  byte/glyph resolution described above, not just generic node/edge
  primitives.
- Phase 2's exit criteria gain a new item: prove the byte-level cost
  mitigation (MrT5-style merging or equivalent) actually reduces the
  measured overhead, not just cite that such techniques exist elsewhere.
- Every claim in this ADR is either sourced (see LITERATURE.md) or
  explicitly marked as this project's own design choice — per
  CONTRIBUTING.md rule 2, no unsourced novelty claims.
- If "SIS" turns out to mean something different than the working
  definition in decision 4, this ADR needs a follow-up amendment before
  Phase 2 implementation starts — do not let code diverge silently from
  a corrected definition.

## Sources

See [LITERATURE.md](../LITERATURE.md) for the dated, verified source list.

## Progress note (2026-07-08)

What "docs + paths + glyphs" means concretely, right now, stated
precisely rather than as a tagline:

**Real and tested:**
- Docs → graph: `docparse.rs` (headings, lists, fenced code → typed nodes).
- Paths → graph: `pathparse.rs` (directory segments → `Content` nodes,
  leaf typed by extension, shared prefixes deduplicated).
- Path mutation → graph mutation: `fsevents.rs` (`Created`/`Removed`/
  `Renamed` → `add_node`/`remove_node`), pure and deterministic.
- Case/punctuation preserved as signal: `leafsignal.rs` (real per-
  character counts; every character accounted for, none dropped).

**Explicitly not real yet — do not describe these as done:**
- No real OS filesystem watching. `fsevents.rs` only translates an
  already-known event into a graph mutation; nothing observes an actual
  directory yet. Adding that needs a new external dependency (e.g.
  `notify`), a decision not yet made.
- No PIXEL-lite visual glyph fingerprint. `leafsignal.rs` counts
  characters by category; it does not render or encode glyph shape.
  That needs an actual trained visual feature extractor.
- No lazy byte-exact leaf resolution as originally scoped in decision 2
  above — `docparse.rs` stores leaf content as a plain, eagerly-built
  `String` today.
- No ledger. ADR 0001 already decided to reuse GH05T3's ChronosLedger;
  it hasn't been ported into this repo. A stand-in was deliberately not
  built here to avoid contradicting that decision.
