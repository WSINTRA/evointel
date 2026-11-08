# Wunicorm v2

> A notes and planner that uses evolving intelligence to augment daily life and work.

**v1** was a self-running engine that evolved a *bot's* identity from its own reflections. **v2** points that same engine at *you*: your Mac Notes become the input, the model being built is a model of your life, and the output is **augmentation** — a daily sermon, reflections, and eventually proposed tasks you confirm. The engine stays; the subject of the engine changes.

## The current app

Wunicorm is now a **local-only, single-user macOS daemon**. You keep writing in Mac Notes. A menu-bar app reads your notes on a daily schedule, runs the memory pipeline, and returns the result through a macOS notification and a note in the `Wunicorm` folder.

```text
Mac Notes
   │  osascript, on schedule
   ▼
Ingress ── new/changed notes by Core Data ID + modification date
   ▼
Scheduler ── once per day, configurable; default 22:00
   ▼
Dreamer ── LLM judge → LadybugDB graph → reflection → sermon
   ▼
macOS notification + sermon in the Wunicorm Notes folder

Menu-bar tray: idle · dreaming · notification
```

There is **no web app, localhost server, Tailscale, Tiptap editor, or separate notes database**. Configuration is a TOML file; Mac Notes is the input and output surface.

## Current status

### Implemented and verified

- **Memory layer** — semantic space-time graph on LadybugDB, typed nodes and four-pillar edges, idempotent mutations, traversal/temporal/drift/snapshot queries, and HNSW semantic search.
- **Function space** — Jungian axes, corpus-mean-centered geometry, user and Wunicorm positions, rigidity, compensation, and the deterministic self pass.
- **Demo dream pipeline** — the §8 job-search story runs end to end through the judge, graph, reflection, and sermon; the full hermetic eval covers rebuildability.
- **Notes ingress** — reads Mac Notes through osascript, skips `Wunicorm` and `Recently Deleted`, strips HTML to text, and queues only new or changed notes.
- **Daemon shell** — persistent scheduler and state, once-per-day guard, notification and Notes-writer output, and graceful shutdown.
- **Menu-bar tray** — a working `tao` macOS event loop with idle, dreaming, and notification states. The tray's appearance has been visually confirmed.
- **macOS packaging** — `Wunicorm.app` and a current-architecture DMG with vendored OpenSSL libraries, an ad-hoc signature, and verified bundle, signature, and image checksum.
- **Tests** — 177 tests pass; Clippy is clean.

### Not yet verified or deferred

- **A complete live scheduled dream against real Mac Notes and the local LLM servers is still pending.** Component tests, the demo pipeline, packaging checks, and tray acceptance do not prove that full cycle.
- **Task proposals, note CTAs, and the general task engine** are deferred.
- **Launch at login** is not configured by the app or DMG.
- **Developer ID signing, notarization, and universal Intel/Apple Silicon builds** are deferred. The current DMG is ad-hoc signed for the build machine's architecture.
- **Working-day/continuous ingestion, email, attention cards, and other output channels** remain design work.
- **Redaction before remote LLM use** is still a stub; the current target is local LLM servers.

## Run and install

### Development

```bash
cd memory-proto
cargo run                 # resident menu-bar daemon
cargo run --example demo  # live §8 demo against the local LLM servers
cargo test -- --test-threads=1
cargo clippy --all-targets -- -D warnings
```

The daemon creates its configuration on first run at:

```text
~/Library/Application Support/memory-proto/config.toml
```

The default dream time is `22:00`. The LLM defaults expect chat on `:9931` and embeddings on `:9932`; override them with `WUNICORM_CHAT_URL`, `WUNICORM_CHAT_MODEL`, `WUNICORM_EMBED_URL`, and `WUNICORM_EMBED_MODEL`, or use the `[llm]` config section. The demo writes to `memory-proto/demo-data/`; the daemon uses `~/Library/Application Support/memory-proto/`.

### macOS DMG

```bash
cd memory-proto
./scripts/package-macos.sh
```

The script writes `target/packaging/Wunicorm-<version>-<arch>.dmg`. Mount it and drag `Wunicorm.app` to `Applications`. On first use, allow Notes automation and notification access. Quit any CLI daemon before launching the app so two processes do not share the database.

See [`memory-proto/README.md`](memory-proto/README.md) for packaging internals and data-preservation details.

## What's in this repo

| Path | What it is |
|---|---|
| [`plan.md`](plan.md) | Current design, implementation boundaries, decisions, and open threads. |
| [`index.html`](index.html) | Visual companion to the design overview. |
| [`memory-proto/`](memory-proto) | The Rust daemon, memory layer, Dreamer, tray, and macOS packaging. |
| [`memory-proto/README.md`](memory-proto/README.md) | Daemon, demo, test, and DMG instructions. |
| [`memory-proto/AGILE.md`](memory-proto/AGILE.md) | Work breakdown and verification status. |
| [`memory-proto/prompts/`](memory-proto/prompts) | Extraction, schema, sermon, and anchor prompts — data, not code. |
| [`AGENTS.md`](AGENTS.md) | Conventions for AI coding agents working on this project. |
| [`PAPER_TRAIL.md`](PAPER_TRAIL.md) | Append-only record of concrete planning and development decisions. |
| [`PROJECT_TRACKER.md`](PROJECT_TRACKER.md) | How the GitHub project board is run. |

CI runs Clippy and tests on push/PR to `main` via [`.github/workflows/ci.yml`](.github/workflows/ci.yml).
