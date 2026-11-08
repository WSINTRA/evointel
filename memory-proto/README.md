# memory-proto

Wunicorm's Rust memory layer and resident macOS daemon: a semantic space-time knowledge graph built on [LadybugDB](https://ladybugdb.com/), fed by Mac Notes and presented through a menu-bar tray.

## What this is

The crate contains both the proven memory pipeline and the working daemon shell. The memory layer is the most novel part of Wunicorm v2. It models concepts as a multipartite graph where nodes carry both **semantic position** (embedding vectors) and **temporal position** (timestamps), and edges are typed by four pillars:

| Pillar | Relationship |
|---|---|
| **Similarity** | semantic proximity |
| **Containment** | hierarchical / categorical |
| **Property Attribution** | a concept carrying a value |
| **Causality** | one tends to produce/motivate another |

Because time is a dimension of position, a thought can be seen *drifting* across its neighborhood -- which is how the system infers habits and thinking patterns.

### The function space (the Jungian axes, as geometry)

The four functions -- thinking, feeling, sensing, intuiting -- live in the graph itself, not just the prompts:

- **Two bipolar axes** (judgment: thinking ↔ feeling; perception: sensing ↔ intuiting), seeded from the frozen exemplar corpus in `prompts/anchors-v1.md` and stored as `FunctionAnchor` rows. Every `Note` projects to a signed 2-dim coordinate; the corpus-mean centering (anisotropy removal) is part of the method -- the raw embedding space failed the gate at 0.94 opposite-pole cosine; centered, it passes at −0.35 with leave-one-out 1.0/1.0/1.0/0.9.
- **Two points, one frame.** **U** (the user's psyche) is the windowed centroid of note coordinates. **W** (Wunicorm's voice) is the position implied by its *choices* -- what it kept, dropped, and emphasized, tagged by function. W is a damped core (hysteresis: personality, not mood) plus a fast compensation component gated by U's **rigidity** (straightness of trajectory, not direction).
- **The self pass** -- a deterministic stage in the dream cycle between writing judgments and reflection. The sermon speaks from it in movement language, from day one; numbers belong to the ledger.

The gate (`tests/anchor_eval.rs`, live) decides whether pure geometry is trustworthy before anything downstream depends on it.

## Stack

- **LadybugDB** (`lbug`) -- embedded graph database, Cypher query language, property graph model
- **Rust** -- type safety, memory safety, zero-cost performance
- **Vector search** -- native HNSW index over `FLOAT[768]` embedding properties (EmbeddingGemma-2 on a local `llama.cpp` server)
- **Versioned migrations** -- `SchemaMeta` marker + idempotent DDL batches; `schema::init` migrates in place on every open

## Ontology

Content and infrastructure tables across layers:

| Layer | Node type | Description |
|---|---|---|
| 0 | Edge nodes | Reified relations (Contains, Similar, HasProperty, LeadsTo, Causes, Prevents, BecauseOf, Before, After, During, ValidFrom, ValidTo, **Grounds**) |
| 0 | FunctionAnchor | The axes' exemplar embeddings; re-seeding deactivates old rows (anchor history preserved) |
| 0 | SchemaMeta | The schema's version marker |
| 1 | Entity | Universal base type -- everything in the graph is an entity at its core |
| 1 | Time / AbstractTime | Concrete temporal spans; fuzzy anchors ("future", "soon", "never") |
| 2 | Fact | Declarative knowledge with predicate + certainty |
| 3 | Event | Something that happened or is planned, with status |
| 3 | DreamRun | The dreamer's per-run choice ledger (considered/dropped, keeps per function) |
| 4 | Memory | Experiences worth keeping -- significance, emotions, reflection, function tag, run provenance |
| 4 | SelfState | One point in function space per subject (`user` / `wunicorm`) per window |

Plain rel tables: `TIME_HIERARCHY` (Time → Time) and `SELF_TRAJECTORY` (SelfState → SelfState, consecutive same-subject windows).

## Getting started

```bash
cargo run                 # the resident menu-bar daemon (dreams at the configured time)
cargo run --example demo  # the live §8 demo dream: seeds the axes and prints the self reading
cargo test                # integration tests (serial: cargo tst)
cargo clippy              # lint (CI: --all-targets -- -D warnings)
```

The daemon reads `~/Library/Application Support/memory-proto/config.toml` on
macOS and creates it with a default dream time of `22:00` on first run. The
daemon needs Mac Notes and the local LLM servers for a real dream. The demo
and live gate also need those servers (`WUNICORM_EMBED_URL` and the chat
endpoint). The **test suites** are hermetic except for explicitly enabled live
probes.

## macOS app and DMG

Build a self-contained app and drag-to-install DMG for the current Mac
architecture:

```bash
./scripts/package-macos.sh
```

The artifact is written to `target/packaging/Wunicorm-<version>-<arch>.dmg`.
Mount it and drag `Wunicorm.app` to `Applications`. The app runs as a menu-bar
agent without a Dock icon. On first use, allow the macOS prompts for Notes
automation and notifications.

The packaging script vendors LadybugDB's OpenSSL dependencies into the app,
rewrites their load paths, includes the OpenSSL license, verifies the bundle,
and ad-hoc signs it. The DMG is intended for local use; it is not Developer ID
signed or notarized, so Gatekeeper may warn if it is copied to another Mac.

Configuration, state, the graph database, and dream artifacts remain in
`~/Library/Application Support/memory-proto/`; reinstalling the app does not
replace them. Quit any running CLI daemon before launching the app so two
processes do not share the database.

## Project layout

```
src/
  main.rs           # daemon entry: config, DB, worker thread, macOS tray event loop
  daemon.rs         # sleep → schedule → ingest → dream → output
  config.rs         # TOML configuration and platform data paths
  ingest.rs         # Mac Notes osascript bridge, HTML stripping, change detection
  scheduler.rs      # dream-time check, once-per-day guard, persisted state
  output.rs         # macOS notifications and the Wunicorm Notes writer
  tray.rs           # tao event loop, menu, idle/dreaming/notification icons
  axes.rs           # the function axes: corpus, pole centroids, coordinates, the gate metrics
  dreamer.rs        # run_dream: judge → write → choice ledger → self pass → reflect → speak
  observability.rs  # DreamReport + SelfPassReport, the JSONL dream ledger, nightmare detector
  llm/              # client layer (UreqClient, RecordedClient), LlmJudge, LlmSermonWriter
  schema/           # versioned migrations: meta, batches v1-v3, DDL per area
  mutations/        # write path: notes, content, self_ops, edge_links
  queries/          # read path: fetch, traversal, drift, psyche, snapshot, temporal
  message.rs        # the outbox artifact + shelf check
  vector.rs         # HNSW index init + semantic search
examples/demo.rs    # the original live §8 demo dream
packaging/macos/    # Info.plist and the generated app-icon source
scripts/            # package-macos.sh: app bundle, vendored dylibs, ad-hoc sign, DMG
prompts/            # extract-v1.md, sermon-v1.md, schema-v1.json, anchors-v1.md (the frozen corpus)
tests/              # per-area suites + full_eval (the end-to-end story) + anchor_eval (the gate)
```

## Status

The prototype build-out (old Tasks 1-19) and the function space (Tasks 20-31, see AGILE.md) are complete:

- [x] Schema, mutations, queries, vector search, drift reads, snapshot/similarity
- [x] The dreamer: run_dream, Judge/SermonWriter seams, choice ledger, the self pass
- [x] The axes: corpus, anisotropy-removed geometry, the separability gate (live-verified)
- [x] The psyche: positions, rigidity (straightness ≥ 3 windows), function drift, W decomposition
- [x] The sermon speaks from the self (movement language only; numbers stripped at construction)
- [x] Versioned migrations (idempotent batches, in-place upgrades, transactional-DDL semantics pinned)
- [x] Full eval: the §8 story on the axes, SelfState rebuildability
- [x] Resident daemon components: Mac Notes ingress, scheduler, notifications, Notes writer
- [x] Menu-bar tray on a real macOS event loop, with idle/dreaming/notification states; appearance visually confirmed
- [x] Local app bundle and verified DMG packaging (vendored OpenSSL, ad-hoc signature)
- [x] 177 tests passing and Clippy clean

### Verification boundary

The memory/full-eval suites, daemon components, DMG mount/signature/checksum,
and tray appearance are verified. **A complete live scheduled dream against
real Mac Notes and the local LLM servers has not yet been accepted.** That
check still needs to prove scheduled ingestion, graph writes, notification,
Notes output, quiet-day behavior, and the once-per-day/next-day guard.

Task CTAs and the general task engine, launch at login, continuous/working-day
ingestion, Developer ID signing/notarization, and universal Intel/Apple Silicon
builds are deferred. Real redaction is required before configuring a remote
LLM; the current target is local model servers.
