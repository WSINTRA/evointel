# Wunicorm v2

> A notes and planner that uses evolving intelligence to augment daily life and work.

**v1** was a self-running engine that evolved a *bot's* identity from its own reflections. **v2** points that same engine at *you*: your notes become the input, the model being built is a model of your life, and the output is **augmentation** — a daily sermon, reflections, and proposed tasks you confirm. The engine stays; the subject of the engine changes.

## The shape

Local-first and single-user, Jellyfin-style: a resident daemon on your Mac serving a web app on `localhost`, reachable from your own devices over Tailscale. Never exposed to the public internet.

```
NOTES APP -> NOTES DB -> SORTER -> PROCESSING -> GRAPH DB -> INSIGHT SERVICES
```

Your notes flow down that spine: stored, watched, structured into a knowledge graph, read for insight, and returned as augmentation. Three cross-cutting layers — scheduling, config, security — span the whole thing.

Of that spine, the prototype currently realizes the **storage → graph → insight** stages: the memory layer (semantic space-time graph), the **Dreamer** (the overnight pass: judge notes, write the graph idempotently, reflect, write the morning message into an outbox), and the LLM layer behind them (local, OpenAI-compatible). The notes app, daemon/scheduler, sorter, and delivery channels are designed but not yet built.

The memory layer is the novel part: a **semantic space-time knowledge graph** where nodes carry both a position by meaning (embedding) and a position in time, and edges are typed by four pillars — *similarity, containment, property attribution, causality*. Because time is a dimension of position, a thought's drift across its neighborhood is visible — that's how habits and thinking patterns are inferred.

## What's in this repo

| Path | What it is |
|---|---|
| [`plan.md`](plan.md) | The full design document — architecture, decisions log, open threads. Start here. |
| [`index.html`](index.html) | The visual version of the plan. |
| [`memory-proto/`](memory-proto) | Active prototype: the memory layer **and the Dreamer** in Rust on LadybugDB — judge notes with a local LLM, write the graph, reflect, write the sermon. See its [README](memory-proto/README.md). |
| [`memory-proto/AGILE.md`](memory-proto/AGILE.md) | The prototype's work breakdown — design principles, task list, commit sequence. |
| [`memory-proto/prompts/`](memory-proto/prompts) | The Dreamer's prompts and extraction schema — data, not code. |
| [`AGENTS.md`](AGENTS.md) | Conventions for AI coding agents working on this project. |
| [`PAPER_TRAIL.md`](PAPER_TRAIL.md) | Concrete decisions made during planning and development. |
| [`PROJECT_TRACKER.md`](PROJECT_TRACKER.md) | How the GitHub project board is run. |

## Current status

The vertical slice of the overnight pass is **working end to end**. The §8 story runs for real: three dated notes go through the Dreamer, a local LLM judges them under grammar constraint, nodes and four-pillar edges are written idempotently (deterministic slugs — re-judged concepts MERGE onto the same node), the run is observed via pre-dream snapshots and a dream report, and the morning's sermon comes out with citations to source notes and drift evidence ("your job search is shifting from anxious to strategic").

Specifics:

- **Memory layer** — full ontology (7 node types, 12 edge-node types, polymorphic relations, time tree), typed structs, idempotent mutations, traversal/temporal/drift/snapshot queries, and HNSW semantic search.
- **LLM layer** — OpenAI-compatible client against local `llama.cpp` servers (chat + embeddings, no data leaves the machine); prompts as data; hermetic replay client for tests.
- **Not yet built** — the notes app UI, daemon/scheduler, sorter, delivery channels (email / notification), and the task engine. The redaction seam exists but is a stub.

## Development

```bash
cd memory-proto
cargo run        # the live demo dream — the §8 story against the real servers
cargo test       # hermetic suites (no servers touched)
cargo clippy     # lint
```

`cargo run` is not a schema init anymore — it's the §8 demo dream. It needs the local `llama.cpp` servers up (chat on `:9931`, embeddings on `:9932`; override with `WUNICORM_CHAT_URL` / `WUNICORM_EMBED_URL` and the `_MODEL` variants). Each judgment runs ~1-2 minutes of thinking. It writes `demo-data/dream.db`, `demo-data/outbox/`, and `demo-data/dream-log.jsonl` — and re-running converges instead of duplicating (the idempotency demo). `cargo test` is hermetic and CI-safe; the live probes are opt-in via `WUNICORM_LLM_LIVE=1`.

CI (clippy + tests) runs on push/PR to `main` via [`.github/workflows/ci.yml`](.github/workflows/ci.yml).
