# Wunicorm v2

> A notes and planner that uses evolving intelligence to augment daily life and work.

**v1** was a self-running engine that evolved a *bot's* identity from its own reflections. **v2** points that same engine at *you*: your notes become the input, the model being built is a model of your life, and the output is **augmentation** — a daily sermon, reflections, and proposed tasks you confirm. The engine stays; the subject of the engine changes.

## The shape

Local-first and single-user, Jellyfin-style: a resident daemon on your Mac serving a web app on `localhost`, reachable from your own devices over Tailscale. Never exposed to the public internet.

```
NOTES APP -> NOTES DB -> SORTER -> PROCESSING -> GRAPH DB -> INSIGHT SERVICES
```

Your notes flow down that spine: stored, watched, structured into a knowledge graph, read for insight, and returned as augmentation. Three cross-cutting layers — scheduling, config, security — span the whole thing.

The memory layer is the novel part: a **semantic space-time knowledge graph** where nodes carry both a position by meaning (embedding) and a position in time, and edges are typed by four pillars — *similarity, containment, property attribution, causality*. Because time is a dimension of position, a thought's drift across its neighborhood is visible — that's how habits and thinking patterns are inferred.

## What's in this repo

| Path | What it is |
|---|---|
| [`plan.md`](plan.md) | The full design document — architecture, decisions log, open threads. Start here. |
| [`index.html`](index.html) | The visual version of the plan. |
| [`memory-proto/`](memory-proto) | Active prototype: the memory layer in Rust on LadybugDB. See its [README](memory-proto/README.md). |
| [`AGENTS.md`](AGENTS.md) | Conventions for AI coding agents working on this project. |
| [`PAPER_TRAIL.md`](PAPER_TRAIL.md) | Concrete decisions made during planning and development. |
| [`PROJECT_TRACKER.md`](PROJECT_TRACKER.md) | How the GitHub project board is run. |

## Current status

The project is in the prototyping phase. `memory-proto/` validates the memory layer: schema (entity nodes, edge nodes, polymorphic relations, time tree), mutation API, and extension loading (vector, FTS). The notes app, daemon, and insight services are designed but not yet built.

## Development

```bash
cd memory-proto
cargo run        # opens memory-proto.db, runs schema init
cargo test       # integration tests
cargo clippy     # lint
```

CI (clippy + tests) runs on push/PR to `main` via [`.github/workflows/ci.yml`](.github/workflows/ci.yml).
