# Wunicorm v2 — Design Overview

> **A notes and planner that uses evolving intelligence to augment daily life and work.**

- **Status:** v1 daemon implemented · live scheduled-dream verification pending
- **Date:** November 2026
- **Companions in this directory:** `index.html` (the visual version)
- **Scope:** architecture, current implementation boundaries, and open threads — not a full API specification.

---

## The move, in three lines

- **v1** is a self-running engine that evolves a *bot's* identity from its own reflections.
- **v2** points that same engine at *you*: your notes become the input, the model being built is a model of your life, and the output is **augmentation** — not a chat, not a self-portrait.
- The engine stays. The **subject** of the engine changes.

The through-line of v1 that must survive: **the ability to add new tasks is a first-class feature**, not an accident. The notes pivot must not consume it. Task proposals and the general task engine are deferred, but they remain part of the target design: declared units of work that produce an artifact and are proven to have run, with a human confirming what the intelligence may do.

---

## Table of contents

1. [North star — what we keep](#1-north-star--what-we-keep)
2. [The shape](#2-the-shape)
3. [The notes app](#3-the-notes-app)
4. [Security & privacy](#4-security--privacy)
5. [The architecture](#5-the-architecture)
6. [The knowledge graph](#6-the-knowledge-graph)
7. [The two note spaces](#7-the-two-note-spaces)
8. [The augmentation loop](#8-the-augmentation-loop)
9. [The durable core, re-homed](#9-the-durable-core-re-homed)
10. [The pipeline's rhythm](#10-the-pipelines-rhythm)
11. [Foundation & stack](#11-foundation--stack)
12. [What still folds](#12-what-still-folds)
- [Appendix A — decisions log](#appendix-a--decisions-log)
- [Appendix B — glossary](#appendix-b--glossary)

---

## 1. North star — what we keep

v1 earned trust through properties that are about *how* it runs, not *what* it talks about. v2 carries those properties forward inside the daemon. Some are implemented in the current memory pipeline; others remain requirements for the deferred agentic task layer.

| Kept | Current realization |
|---|---|
| **The shelf check** | Implemented. A sermon is accepted only when its artifact exists, is non-empty, belongs to the current run, and passes its assertions. |
| **Two kinds of truth** | Implemented. Scheduling and declarations stay apart from the `DreamReport`, the append-only dream log, and the observed graph delta. |
| **Guards in code, not in a prompt** | Implemented for the shelf check, graph writes, freshness, and the self pass. The outbound gate and full task guard library arrive with tasks. |
| **Two channels, deliberately split** | A design invariant. The current daemon has reflective sermon output and mechanical notifications; a separate attention-card channel is not yet implemented. |
| **The Jungian framework, as a lens** | Implemented as geometry in the graph. Your notes are the external evidence base the v1 closed SELF loop lacked. |
| **Extensible tasks** | Deferred, not discarded. No CTA or general task-execution surface is exposed yet. |

---

## 2. The shape

Wunicorm is a **local-only, single-user macOS daemon** with a menu-bar icon. It is a resident process, not a script that fires and exits. Mac Notes is the input surface and the place where the sermon is returned.

```
[ Mac Notes app ]  <---- you write, as usual
       |
       |  osascript, on schedule
       v
[ Wunicorm daemon ]  ---- menu-bar icon: idle / dreaming / notification
       |
       +--> [ LadybugDB memory graph + Dreamer ]
       |
       +--> [ macOS notification ]
       +--> [ sermon in the Wunicorm Notes folder ]
```

- **No web app or localhost server.** The earlier browser/Tiptap/Tailscale design was abandoned.
- **The tray is the only custom UI.** It shows daemon state, opens the Wunicorm Notes folder, and quits the app.
- **Configuration is TOML**, created on first run under `~/Library/Application Support/memory-proto/`.
- **The daemon owns the schedule.** Its default is one thorough dream per day at 22:00 local time.
- **Distribution is a native app bundle and DMG.** The current build is ad-hoc signed for the build machine's architecture.

---

## 3. The notes app

You use **Mac Notes**, not a Wunicorm editor. It holds the heterogeneous, real stuff — todos, project notes, house tasks, thinking, poetry. This is the primary input for the intelligence because it is already open and what you type there is what you later want refined.

**Current ingress**

- Reads every folder except `Wunicorm` and `Recently Deleted` through osascript.
- Uses the note's Core Data URI as its stable identity.
- Compares modification dates against persisted `last_seen` state to queue only new or changed notes.
- Converts the Notes HTML body to plain text for the LLM.

**Current output**

- Creates the `Wunicorm` folder if needed.
- Writes each produced sermon there as an HTML note.
- Sends a macOS notification with a short preview.

**Deferred**

- Task CTAs and task confirmation.
- Continuous or working-day ingestion.
- Any custom note editor, sorting UI, helpdesk, or in-app configuration screen.

---

## 4. Security & privacy

Single-user and local-first. The security model relies on the OS, not on an app-level crypto layer.

- **At rest** — FileVault protects the database and state files. macOS handles screen locks and session security.
- **In transit** — there is no network app layer. No localhost API or Tailscale transport exists.
- **Notes access** — macOS automation permission gates the osascript bridge to Notes.
- **At the model** — the current target is local OpenAI-compatible LLM servers, so note content does not need to leave the machine. The redaction seam currently exists only as a stub; a real redactor is required before configuring a remote endpoint.
- **No app-level encryption** — no client-side zero-knowledge, password, seed, or key derivation. The daemon processes plaintext so it can build the graph and insights.

> **Revisit if the product grows.** A distributed or cloud-hosted service changes the trust model and requires application-layer transport and storage protections.

---

## 5. The architecture

On each scheduled run, notes move through ingress, the established memory pipeline, and the output layer. Configuration, scheduling, and observability span the flow.

```
  MAC NOTES         your folders, except Wunicorm + Recently Deleted
      |  osascript
      v
  INGRESS           Core Data IDs + modification-date change detection
      |  new/changed Note values
      v
  SCHEDULER         daily dream time + once-per-day guard
      |
      v
  PROCESSING        embeddings + LLM judge: nodes, facts, memories,
      |             time, and four-pillar edges
      v
  GRAPH DB          LadybugDB — semantic space-time memory
      |  read + analyze
      v
  DREAMER           drift, events, choice ledger, self pass, reflection,
      |             sermon, shelf check, DreamReport
      v
  OUTPUTS           macOS notification + sermon in Wunicorm Notes folder

  CONFIG            TOML + environment overrides for local LLM endpoints
  TRAY              tao event loop; idle / dreaming / notification icons
  STATE             last_seen + last_dream_date JSON, outside the graph
```

**Layer notes**

- **Ingress** — the osascript bridge and change detector in `src/ingest.rs`.
- **Scheduler/state** — the daily trigger and persisted JSON state in `src/scheduler.rs`.
- **Processing and Dreamer** — the existing judge, write path, reflection, self pass, and sermon pipeline.
- **Graph DB** — LadybugDB (`lbug`), not SQLite/Postgres or a separate notes store.
- **Outputs** — `src/output.rs`; the Dreamer writes an outbox artifact, and the daemon delivers it.
- **Tray** — `src/tray.rs`, using a real `tao` main-thread event loop.
- **Daemon** — `src/daemon.rs` coordinates the worker loop; errors are logged and the loop continues.

---

## 6. The knowledge graph

The graph is **not a keyword index**. Each concept sits at a point in a combined space — **what it means** (semantic proximity / embedding) **and when** (time). Edges are typed by **four pillars**. Because time is a dimension of position, a thought can be seen *drifting* across its neighborhood — which is how it infers habits and thinking.

**The four pillars (edge types)**

| Pillar | Relationship | Reads as |
|---|---|---|
| **Near / similar to** | proximity | these two concepts sit close in meaning |
| **Leads to** | causal | one tends to produce or motivate the other |
| **Contains** | hierarchical | a category holding more specific things |
| **Expresses property** | attributive | a concept carrying a value (salary, status, date) |

The processing layer reads the notes and makes these judgments — deciding the nodes, the pillar-edges, and each node's position in semantic space-time. Nodes carry an origin marker: *in your notes* vs *inferred by wunicorm*.

### The function space (landed — memory-proto, Tasks 20-31)

The Jungian functions are **geometry in the graph itself**, not just prompt language. Two bipolar axes — judgment (thinking ↔ feeling) and perception (sensing ↔ intuiting) — are built from contrastive anchor centroids seeded from a frozen, gate-verified exemplar corpus (`prompts/anchors-v1.md`, embedded and versioned as `FunctionAnchor` rows; re-seeding deactivates, never deletes). Every note projects to a signed 2-dim coordinate. Corpus-mean centering removes anisotropy and is verified by a live separability gate.

**Two points, one frame:**

- **U — the user's psyche.** The windowed centroid of note coordinates. Read, never steered, never claimed.
- **W — Wunicorm's voice.** The position implied by its *choices*: what it kept, dropped, and emphasized, tagged by function. A damped **core** (hysteresis — personality, not mood) plus a fast **compensation** component gated by U's **rigidity** (straightness of trajectory over ≥3 windows — stuckness, never direction). When U is balanced, W accompanies; a stuck user gets a counterweight.

The dream cycle includes a deterministic **self pass** between writing judgments and reflection: it measures notes onto the axes, writes both `SelfState`s, wires `SELF_TRAJECTORY`, and grounds positional claims in evidence notes and kept memories via `Grounds` edges. The sermon speaks from the self reading in **movement language only**; numbers are stripped at material construction, and the user's movement is never attributed to the system.

The standing guards: rigidity-not-direction · damped prescription · the auditor is audited · no claimed effects · the discarded is data (the `DreamRun` choice ledger) · no revision without observation. Full record: `PAPER_TRAIL.md` D-002.

---

## 7. The two note spaces

The two-space concept now lives directly in Mac Notes rather than in a custom `source:*` database.

- **Your folders** — the input; the living stream you type. Every folder except the two exclusions is eligible for ingress.
- **The `Wunicorm` folder** — the output; sermons and reflections written back for inspection. It is excluded from ingress so the daemon does not ingest its own writing.

The daemon is another writer to the same application, not a hidden backchannel. The model remains yours to read and correct. Richer inferred notes-on-you beyond the current sermon remain part of the target design.

---

## 8. The augmentation loop

Wunicorm does **not** wait to be asked. It runs on its schedule and reaches out — a daily **sermon** on your notes, a notification, and a visible tray state. It is a relationship with a cadence, not a Q&A box.

**Current loop**

1. **You write or edit a Mac Note.**
2. **At dream time**, ingress detects the new/changed modification date and queues it.
3. **The Dreamer** embeds and judges the note, writes the graph, reflects, and performs the self pass.
4. **The shelf check** verifies that a real sermon artifact exists.
5. **The daemon** sends a notification and writes the sermon into the `Wunicorm` folder.
6. **The tray** reports dreaming and then notification/idle state.

**Target extension — tasks, not yet implemented**

A note containing five companies could produce “Want me to research these 5?” A future confirmation surface would run that work through the durable task engine and prove the artifact with a shelf check. There is currently no CTA, task proposal, or task execution.

---

## 9. The durable core, re-homed

The trust model is still that action is **fenced by code**. The current daemon is not generally agentic, so the task-specific guards are requirements for the next layer rather than claims about the present app.

| Guard | Current status |
|---|---|
| **Shelf check** | Implemented for the message artifact. |
| **Two kinds of truth** | Implemented through `DreamReport`, the dream log, and pre/post graph observation. |
| **Outbound gate** | Deferred with tasks and external sends. Current outputs are local notifications and Notes writes. |
| **Fault ledger** | The dream log and nightmare detector exist; the complete v1 task-fault workflow is not yet re-homed. |
| **Redaction** | Stub only. Mandatory before any remote LLM endpoint. |
| **Two channels** | Sermon and mechanical notification exist; a separate attention-card implementation is deferred. |

---

## 10. The pipeline's rhythm

The implemented cadence is deliberately simple: **one scheduled, thorough pass per day**.

```
[ Sleep / tick every 30s ] --> [ Is it dream time, and not dreamed today? ]
                                      |
                                      v
                         [ Pull → detect changes → dream ]
```

- **Default:** 22:00 local time, configurable in TOML.
- **Once-per-day guard:** `last_dream_date` prevents repeated dreams on the same date.
- **Quiet day:** no changed notes means no LLM dream and no sermon.
- **State updates:** `last_seen` and `last_dream_date` persist after the run.
- **Working-day/continuous passes:** future work. The current daemon does not watch keystrokes.

> **The graph is derived, so it is rebuildable.** It is computed from notes, not the source of truth. A full re-run can regenerate it; state and dream reports remain outside the graph.

---

## 11. Foundation & stack

The app is built directly in Rust around the proven memory layer. The earlier plan to fork `tees_valley_digital` and reuse a web stack was discarded.

| Area | Current implementation |
|---|---|
| Language/process | One Rust process; daemon worker thread + main-thread macOS UI |
| Notes bridge | `osascript` via `std::process::Command` |
| Graph/memory | LadybugDB (`lbug`) with Cypher and HNSW vector indexes |
| LLM | OpenAI-compatible synchronous client (`ureq`), local chat + embedding servers |
| Scheduling/state | `chrono`, TOML config, JSON state |
| Tray | `tray-icon` + `tao` event loop; Accessory activation policy |
| Packaging | `Wunicorm.app`, `Info.plist`, `LSUIElement=true`, generated icons |
| DMG | `hdiutil`, current architecture, ad-hoc `codesign` |
| Native dependencies | OpenSSL dylibs vendored into the app and rewritten to `@rpath`; license included |
| Testing | Hermetic Rust suites plus opt-in live LLM probes |

The old SQLite-vs-Postgres and graph-store choices are resolved for this implementation: **Mac Notes for source content, LadybugDB for derived memory**.

---

## 12. What still folds

The remaining work is now explicit rather than architectural guesswork:

- **Live daemon acceptance** *(verification)* — run a complete scheduled dream against real Mac Notes and the local LLM servers; verify ingestion, graph writes, notification, Notes output, quiet-day behavior, and next-day scheduling.
- **“Overnight” = machine-on** *(honest limit)* — the thorough pass runs only while the Mac is awake. This is an accepted consequence of local-first design.
- **Task CTAs and task reach** *(product + trust)* — how tasks are proposed and confirmed, and whether a capability is read-only research or has side effects.
- **Launch at login** *(operations)* — deferred; the current DMG does not install a LaunchAgent.
- **Distribution** *(packaging)* — Developer ID, notarization, and universal Intel/Apple Silicon builds are deferred.
- **Remote-model safety** *(security)* — implement real redaction before any non-local LLM endpoint is allowed.
- **Additional channels and cadences** *(design)* — attention cards, email, working-day ingestion, and richer notes-on-you.

---

## Appendix A — decisions log

| Decision | Current choice | Rationale |
|---|---|---|
| Input model | **Mac Notes via osascript** | Already in daily use; no custom editor or notes DB required. |
| Output | **Notification + sermon in the Wunicorm Notes folder + tray state** | Local, inspectable, proactive; no chat surface. |
| Run model | **Local-only Rust daemon and menu-bar app** | Privacy and simplicity; no web app or Tailscale. |
| SELF | Repurposed as a lens on *you* | Your notes are the external anchor the v1 loop lacked. |
| AI backend | OpenAI-compatible local endpoints with env/TOML overrides | Decouple the engine from one model or CLI. |
| Memory | LadybugDB semantic space-time graph, four pillars | Implemented and verified by the memory/full-eval suites. |
| Cadence | **One configurable daily dream, default 22:00** | Simple v1 scheduling; continuous/working-day passes deferred. |
| Security | OS-level local security; real redaction required before remote LLMs | Match the actual single-user threat model. |
| Extensibility | Tasks remain first-class but are deferred | Avoid pretending the current daemon is agentic. |
| Foundation | Rust + LadybugDB + osascript + tao | The working implementation, not a web-app fork. |
| Packaging | Current-architecture app/DMG, vendored OpenSSL, ad-hoc signing | Reproducible local install without Apple Developer credentials. |
| Verification | Components, tests, tray, and DMG accepted; full live scheduled dream pending | Keep the evidence boundary explicit. |

---

## Appendix B — glossary

- **Ingress** — the scheduled osascript bridge that reads Mac Notes, detects new/changed notes, and builds the dream queue.
- **Scheduler** — the daily dream-time check and once-per-day guard.
- **Dreamer** — the memory pipeline: judge, idempotent graph writes, observation, reflection, self pass, sermon, and shelf check.
- **Graph DB / Intelligence DB** — LadybugDB; the derived semantic space-time memory with four relationship pillars.
- **Two note spaces** — your eligible Mac Notes folders and the excluded `Wunicorm` output folder.
- **Sermon** — the daily reflective address on your notes, with the Jungian lens and movement language.
- **Shelf check** — the guard that proves a real artifact exists before it is accepted or delivered.
- **Two kinds of truth** — declaration (what should happen) vs observation (what did), represented by state/config versus reports and logs.
- **Tray** — the menu-bar UI showing idle, dreaming, and notification states.
- **Semantic space-time** — a memory model where each concept has a position by meaning *and* time, so drift is visible.
- **Four pillars** — near/similar (proximity), leads to (causal), contains (hierarchical), expresses property (attributive).
