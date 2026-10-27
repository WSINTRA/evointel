# Wunicorm v2 — Design Overview

> **A notes and planner that uses evolving intelligence to augment daily life and work.**

- **Status:** draft · working document
- **Date:** October 2026
- **Companions in this directory:** `index.html` (the visual version) · the full session export
- **Scope:** overview and architecture. Deliberately **not** a tech-stack spec.

---

## The move, in three lines

- **v1** is a self-running engine that evolves a *bot's* identity from its own reflections.
- **v2** points that same engine at *you*: your notes become the input, the model being built is a model of your life, and the output is **augmentation** — not a chat, not a self-portrait.
- The engine stays. The **subject** of the engine changes.

The through-line of v1 that must survive: **the ability to add new tasks is a first-class feature**, not an accident. The notes pivot must not consume it. Tasks in v2 are still declared units of work that produce an artifact and are proven to have run — but the intelligence can now also *propose* them, derived from your notes, and you confirm them.

---

## Table of contents

1. [North star — what we keep](#1-north-star--what-we-keep)
2. [The shape](#2-the-shape)
3. [The notes app](#3-the-notes-app)
4. [Security & privacy — the key model](#4-security--privacy--the-key-model)
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

v1 earned trust through properties that are about *how* it runs, not *what* it talks about. v2 re-homes all of them inside the daemon. None are replaced.

| Kept | What it is |
|---|---|
| **The task engine** | A task = a declared unit of work → an artifact it must produce → a **shelf check** that proves it actually ran. A green exit code never stands in for a result. |
| **Two kinds of truth** | *Declaration* (what should happen) is stored apart from *observation* (what did). This split is what stopped the "stamped success while writing no file" failure. |
| **Guards in code, not in a prompt** | Shelf check, freshness, outbound gate, fault ledger, redaction — enforced by code, never requested of the model. |
| **Two channels, deliberately split** | The reflective **letter** (about itself, omits a stale artifact and says so) and the mechanical **attention card** (only when a human is needed). |
| **Encrypted at rest** | Memories, reflections — and now *everything* — sit under a user-held key. |
| **The Jungian framework, as a lens** | Shadow, tensions, archetypes, the six evolution rules — repurposed to read *you*, not discarded. In v1 the SELF was a closed loop with no outside anchor; v2 makes **your notes the external evidence base**, so "no revision without observation" finally has real observations to draw on. |

---

## 2. The shape

In the **Jellyfin** sense: it runs on your Mac, serves a web app on `localhost`, and is a **resident process** — not a script that fires and exits. Exposed to your own devices over **Tailscale**; never to the public internet.

```
[ Mac menu-bar icon ]  --launch / config-->  [ localhost web app in the browser ]
[ Tailscale ]  --------------------------------->  [ your other devices, on a private mesh ]
```

- The **Mac app** is a thin shell over the localhost web UI; the note-taking UX is built as a **web app** for fast iteration. The tray icon is the daemon's face (status, config, launch).
- **Config lives in the app**, not in files edited by hand.
- The **daemon is the app's heart**: it owns the schedule and runs the intelligence in the background.
- Tailscale is for *you to reach the app* from elsewhere — **access, not a third party processing your data.**

---

## 3. The notes app

You have it open and you type into it the way you already do. It holds the heterogeneous, real stuff — todos, project notes, house tasks, thinking, poetry. **This is the new primary input for the intelligence.** You chose notes deliberately: it's always open, and what you type there is exactly what you later want refined (a job-search feed, a coding question, a company to research).

**Layout**
- **Side panel:** notes ordered by last-edited, grouped by **Month** — the familiar shape. Sort options deferred.
- **Bodies:** Tiptap rich text (headings, lists, links, images, code).
- **Storage:** a Notes DB (SQLite or Postgres — left open, see §11/§12).

**Features**
- **Two note spaces** in one store — `source: user` and `source: wunicorm`. Your notes, and its notes-on-you (see §7).
- **Task CTAs live on notes.** When wunicorm proposes a task derived from a note, a button appears on that note. You decide.
- **In-app config surface** (see §5, the cross-cutting Config layer).

---

## 4. Security & privacy — the key model

First-class. The password is **not a login the server checks** — it is a **key-derivation input**. It unlocks a key derived in your browser, and the server never sees it. This inverts v1, where the decryption password was a server secret in `.env` (`PGP_PASSWORD`).

1. **First run — set a password.** You choose a master password. The app derives a **recovery seed phrase** you store somewhere safe (wallet, password manager, paper).
2. **Later runs — the password unlocks.** It re-derives the key on-device and decrypts existing content. Forget it, and the seed re-derives the key.
3. **At rest — everything encrypted.** Notes, memories, reflections and the graph all sit under the same key (e.g. `AES-GCM`) as ciphertext. **No server master key.**
4. **At the model — redaction first.** Whatever is handed to an LLM is scrubbed before it leaves the machine, and you can see exactly what was hidden.

**Client-side, zero-knowledge.** The key is derived on your device (password → KDF → key). Note content is fetched, decrypted and rendered **in the browser** — the shell is server-rendered, the content is not. "Wrong password" is detected client-side, not by a server 401.

> **The price, stated plainly.** Key management is yours, exactly like a crypto wallet. That is the cost of "privacy is first class" — so the key UX has to be right: the seed shown once, no silent re-derivation, a way to rotate.

---

## 5. The architecture

Your notes flow down a spine: stored, watched, structured into a graph, read for insight, and returned as augmentation. Three cross-cutting layers — **scheduling, config, security** — span the whole thing.

```
  NOTES APP          Mac tray -> localhost web app (Tiptap, month-grouped, CTAs)
      |  writes
      v
  NOTES DB           your notes  +  wunicorm's notes-on-you   (encrypted at rest)
      |  watches continuously  +  full runs on schedule
      v
  SORTER             one job: pull new/changed content, queue it for processing
      |  hands off
      v
  PROCESSING         an LLM judges & structures content into nodes + 4 pillar-edges,
      |               placed in semantic space-time
      v
  GRAPH DB           the Intelligence DB — the memory (semantic space-time, 4 pillars)
      |  read + analyze
      v
  INSIGHT SERVICES   the daily sermon · reflections · task proposals · the passes
      |  via the channels you chose
      v
  OUTPUTS            email · notification · in-app helpdesk · note CTA · attention card · visual

  SCHEDULER  drives the sorter cadence, the full runs, and the insight services (working-day vs overnight)
  CONFIG     in the app: AI backend (any OpenAPI) · cadence · output channels · visual toggle
  SECURITY   password + seed · client-side zero-knowledge · redaction before any LLM call
```

**Layer notes**

- **Notes App** — the input surface (§3).
- **Notes DB** — the source of truth. Holds both note spaces, encrypted at rest.
- **Sorter** — a constantly-running watcher. Its *one* job: pull new/changed content from the Notes DB and hand it to processing. It is cheap and continuous.
- **Processing** — the LLM (LangGraph or similar) that makes the judgments: what the concepts are, which of the four pillars connect them, and where each sits in semantic space-time. This is where the OpenAPI-agnostic AI wrapper lives.
- **Graph DB** — the Intelligence DB / the memory (§6).
- **Insight services** — the "front" of the intelligence: they read notes + graph and produce the daily sermon, reflections, memories, and task proposals (§8).
- **Outputs** — the channels you configure.
- **Scheduler / Config / Security** — cross-cutting; they are not a stage in the flow, they span it.

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

> "Casual" in the original notes is read as **causal**. Correct here if that's not the intent.

**How it works:** the processing layer reads the notes and makes these judgments — deciding the nodes, the pillar-edges, and each node's position in semantic space-time. Nodes carry an origin marker: *in your notes* vs *inferred by wunicorm*.

> **This is the v1 representation.** Deeper concepts on the semantic space-time model are coming (position, embedding, time, and how drift is measured) — the graph section should be revisited when they land.

---

## 7. The two note spaces

There are two note spaces in one store. You write the first; the intelligence writes the second. Both are encrypted under the same key, and **the model of you is inspectable** — you can read what it has learned about you.

- **Your notes** (`source: user`) — the input; the living stream you type.
- **Wunicorm's notes-on-you** (`source: wunicorm`) — the model; inferred, evolving, inspectable. ("What I noticed: your job search is shifting from anxious to strategic." "Habit: you plan house tasks on Sunday evenings.")

The daemon is simply **another writer** to the same store — the ports/adapters data layer makes adding that actor mechanical. No hidden backchannel; the model is yours to read and correct.

---

## 8. The augmentation loop

Wunicorm does **not** wait to be asked. It runs its passes and reaches out — a daily **sermon** on your notes (v1's Persona, aimed at you, sprinkled with the Jungian lens), a notification, or a suggestion. It is a relationship with a cadence, not a Q&A box.

**The clearest expression of "evolving intelligence augmenting daily work"** — it reads a note and offers to do the next thing:

1. **You write a note** — five companies, links to their sites.
2. **Sorter detects** the change; it is queued.
3. **Graph structures it** — concepts + the four pillars.
4. **Insight pass** reads it overnight.
5. **Morning:** "Want me to research these 5?"
6. **The note shows a CTA** — you tap it; the task runs through the durable engine and the shelf check proves it.

**Output channels (your choice, configurable):**
- **The sermon** — the daily address on your own notes. Reflective, pointed, sometimes affirming, sometimes a light scolding. The letter channel, about itself.
- **A notification** — "Wunicorm has some thoughts on the matter."
- **In-app helpdesk** — one optional channel at the bottom of the notes app (this is the only place chat-like interaction lives; it is not the primary mode).
- **The attention card** — mechanical, sent only when something needs a human (open faults, blocked sends, failed tasks). No model writes it.
- **Optional daily visual** — generated from the reflections, on a toggle.

---

## 9. The durable core, re-homed

v2 is agentic — it proposes and runs tasks, sends, generates. The trust does not come from it being *unable* to act; it comes from its acting being **fenced by code**. The v1 guard library moves inside the daemon and keeps its job unchanged.

| Guard | What it enforces |
|---|---|
| **Shelf check** | A task is a failure unless its artifact exists, is non-empty, was written by this run, and passes its assertions. A green exit code never stands in for a result. |
| **Two kinds of truth** | Declaration is stored apart from observation. |
| **Outbound gate** | Nothing sent to a recipient not on the allowlist; no body containing a credential; one send per (task, date). |
| **Fault ledger** | A known fault is closed by a fix, not by prose; a fault is not re-described past its mention budget. |
| **Redaction** | Credential and personal values never reach the log or the model. |
| **Two channels** | The letter and the attention card are different channels — v1's 2026 failure was one channel carrying the other's news. |

---

## 10. The pipeline's rhythm

A continuous sorter watching every keystroke is cheap; running the LLM on every keystroke is not. So the two are **split**: the **sorter detects and queues** continuously, and the **scheduler** triggers **processing** at intervals. That single split makes "always watching" and "an overnight full run" one coherent flow.

```
[ Watch — continuous ]  -->  [ Queue ]  -->  [ Process — scheduled, LLM ]
```

**Two cadences, one graph:**

- **Working-day** — light, frequent. A quick pass over recent notes; not the full treatment, just enough to keep the model current through the day.
- **Overnight dreamer** — thorough, nightly. The deep pass: full reflection, memories, the Jungian lens, and the material for the morning's sermon.

Both write to the **same Graph DB** — they are the same pipeline at different depth, not two systems.

> **The graph is derived, so it is rebuildable.** It is computed from the notes, not the source of truth. If it ever goes wrong, a full re-run regenerates it — the v1 "no stale artifact" principle, re-homed in the memory layer.

---

## 11. Foundation & stack

The notes app grows out of **`tees_valley_digital`** — a small, clean full-stack app (Bun + TanStack Start + Mantine + Tiptap + Drizzle). Notes and blog posts are close enough cousins that the shell, auth, editor and data pattern carry over.

**The honest effort split:**

| Reuse | Refactor / strip | Net-new |
|---|---|---|
| App shell + Mantine theme | Remove the public SSR surface, SEO, slug | Client-side zero-knowledge encryption |
| Tiptap rich-text editor | Remove the publish workflow | Password + recovery seed flow |
| List / detail views | Invert auth to private-by-default | The daemon API (bearer token + sync) |
| Ports & adapters + DI data layer | Add the notes / todos / planner domain | The sorter / processing / graph layer |
| Single-user auth as the unlock gate | SSR the shell, render content client-side | The insight & sermon services |
| Ops: Docker, CI, error/security plumbing | | |

**The crux tension:** the fork's auth is *server-side* (prove identity to the server; data plaintext in Postgres). v2 needs *client-side zero-knowledge* (key derived in the browser; server stores ciphertext only; seed is user-held recovery). So the existing login/session machinery is reused as the app's *unlock/lock UX and session layer*, but a **client crypto layer is built from scratch** (WebCrypto KDF → AES-GCM, encrypt at the editor boundary, decrypt at render), and the server's ability to ever hold the key is removed. This is the single biggest net-new piece.

**Open tech choices (decide at build time):**
- **Database** — SQLite (single-file, truly local-first) vs. Postgres (keep the fork as-is). Drizzle covers both.
- **Processing framework** — "LangGraph or similar"; not pinned.
- **Graph store** — open (embedded graph, or a graph store over the DB).

---

## 12. What still folds

This is a rough plan on purpose. The places where the architecture is still soft:

- **Daemon key access** *(security)* — how a background daemon decrypts zero-knowledge notes when the app is closed. A secure OS slot (Keychain / Secure Enclave) while the Mac is unlocked, or a defined "unlocked-for-the-daemon" window. The seed stays user-held either way.
- **Database choice** *(stack)* — SQLite vs. Postgres. A portability call.
- **The space-time model** *(graph)* — the deeper concepts behind semantic space-time. To come.
- **"Overnight" = machine-on** *(honest limit)* — on a local daemon the thorough pass runs when the Mac is awake or idle. An accepted consequence of local-first + Mac-first; stated, not hidden.
- **The daemon API surface** *(contract)* — bearer-token auth + sync endpoints: read notes, write notes, read notes-on-you, tag a CTA, propose / trigger tasks.
- **How far a task may reach** *(trust)* — read-only research vs. something with side effects (a deploy). Confirmation per-task or per-capability, and how the user sets that boundary.

---

## Appendix A — decisions log

Decisions made during the planning session:

| Decision | Choice | Rationale |
|---|---|---|
| Input model | Notes app (web, browser) | Always open; what you type is what you later refine. |
| SELF | Repurposed as a lens on *you*, not the bot's self-portrait | Your notes are the external anchor the v1 loop lacked. |
| Output | Proactive augmentation (sermon / notification / CTA), not a primary chat | A cadence, not a Q&A box. Chat only as an optional in-app channel. |
| Run model | Local-first daemon + localhost web app + Tailscale (Jellyfin-style) | Privacy; your machine is the server. |
| AI backend | Configurable against any OpenAPI spec; own thin wrapper | Decouple the engine from any one model/CLI. |
| Memory | Semantic space-time graph, 4 pillars, an agentic memory layer | Beyond append-only dated files. |
| Cadence | Overnight dreamer (thorough) + working-day pass (light) | Configurable processing frequency. |
| Security | Password + recovery seed, client-side zero-knowledge | Privacy is first class; the key lives with the user. |
| Extensibility | Adding/proposing tasks stays first-class | The property that made v1 good software. |
| Foundation | Fork `tees_valley_digital` | Reuse shell, auth, editor, data pattern; strip the blog/SEO layer. |
| Document: database | **Left open** | Both presented; decide at build. |
| Document: audience | **Dense working doc for the owner** | Carries the why, the stress-test, and open threads. |
| Document: Elephas | **Omitted** | Referenced for insight only; not in the doc. |

---

## Appendix B — glossary

- **Sorter** — the continuous watcher that pulls new/changed content from the Notes DB and queues it for processing.
- **Processing** — the LLM pass that structures content into graph nodes and the four pillar-edges, in semantic space-time.
- **Graph DB / Intelligence DB** — the memory; a semantic space-time knowledge graph with four relationship pillars.
- **Insight services** — the intelligence's front: daily sermon, reflections, memories, task proposals.
- **Two note spaces** — `source: user` (your notes) and `source: wunicorm` (its notes-on-you).
- **Sermon** — the daily reflective address on your notes; v1's Persona aimed at you, with the Jungian lens.
- **Shelf check** — the guard that proves a task produced a real artifact before anything is filed or sent.
- **Two kinds of truth** — declaration (what should happen) vs. observation (what did).
- **Attention card** — the mechanical channel sent only when a human is needed.
- **Semantic space-time** — a memory model where each concept has a position by meaning *and* time, so drift over time is visible.
- **Four pillars** — near/similar (proximity), leads to (causal), contains (hierarchical), expresses property (attributive).
