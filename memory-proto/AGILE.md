# Memory-Proto — Work Breakdown

## Where we are

The prototype build-out — Tasks 1-19 (schema, mutations, queries, vector search,
drift reads, snapshot/similarity, message + observability, the dreamer, the LLM
layer, the live judge, the sermon's living voice) — is **complete**. Its record
lives in git history: the previous version of this file and the commit sequence
through `chore: updated docs`. This list replaces it.

## What this list is: the Function Space

The Jungian functions move out of the prompts and into the graph itself:

- **Two bipolar axes** — judgment (thinking ↔ feeling) and perception
  (sensing ↔ intuiting) — built from contrastive anchor pairs seeded into the
  graph. Every `Note` projects to a signed 2-dim coordinate; every dream's
  choices project Wunicorm into the same frame.
- **Two points, one frame.** **U** (the user's psyche) is the windowed centroid
  of note coordinates. **W** (Wunicorm's voice) is the position implied by its
  *choices* — what it kept, dropped, and emphasized, tagged by function. W is a
  damped core (hysteresis: personality, not mood) plus a fast compensation
  component gated by U's **rigidity** (straightness of trajectory, not
  direction).
- **The self pass** — a new deterministic stage in the dream cycle ("working
  the self") between writing judgments and reflection. The sermon speaks from
  it in movement language, from day one.
- **Schema evolution** — all of it lands through a versioned migration layer.
  lbug 0.22.1 natively supports `IF NOT EXISTS`, `ALTER TABLE`, and
  transactions; the prototype used none of them.

The guards, one line each: rigidity-not-direction · damped prescription · the
auditor is audited · no claimed effects · the discarded is data · no revision
without observation. The full concept is recorded in `PAPER_TRAIL.md` (D-002)
and `plan.md` with Task 32.

## Design principles

- **High cohesion**: each module does one thing. `nodes.rs`/`edges.rs` = data.
  `mutations/` = write path. `queries/` = read path. `vector.rs` = search.
  `axes.rs` = function-space math. `queries/psyche.rs` = position/rigidity reads.
- **Low coupling**: mutations and queries depend on types + `db.rs`, not on
  each other.
- **Hidden complexity**: the caller never sees Cypher, never knows edges are
  nodes, never converts `time` ↔ `chrono`.
- **Derived, never authoritative**: coordinates and positions are derived from
  notes + recorded choices. Every positional claim traces to nodes via
  `Grounds` edges.
- **Failures are recorded, not thrown**: the dreamer's contract extends to the
  self pass — a missing anchor or unmeasurable window degrades the dream,
  never kills it.

## Key lbug API notes

- `Connection::query(&str)` — Cypher; returns `QueryResult` (iterator of
  `Vec<Value>`)
- `Connection::prepare(&str)` + `execute(&mut stmt, Vec<(&str, Value)>)` —
  parameterized `$name` queries
- `Value` enum: `String`, `Double`, `Float`, `Timestamp(time::OffsetDateTime)`,
  `Array(LogicalType, Vec<Value>)`, `Bool`, `Null(LogicalType)`, `Node(NodeVal)`
- **Timestamps**: lbug uses `time::OffsetDateTime`; structs keep `chrono`;
  private conversion helpers do the swap
- **Embeddings**: `Value::Array(LogicalType::Float, ...)` for `FLOAT[768]`
  columns — width is `EMBEDDING_DIM` (entity_nodes.rs), the single source of
  truth
- **DDL (verified in lbug 0.22.1 source; unused until Task 20):**
  `CREATE NODE|REL TABLE IF NOT EXISTS`, `ALTER TABLE ... ADD|DROP|RENAME
  [IF [NOT] EXISTS]` (one column per statement), rel-table connection
  extension, `BEGIN TRANSACTION` / `COMMIT`. **No `SHOW TABLES`** — existence
  is probed by query-and-error.
- **Test runs must be serial**: `cargo tst` (alias for
  `test -- --test-threads=1`) — lbug maps a ~8 TiB virtual region per open DB

---

## Tasks

### Task 20: Schema evolution — versioned migrations, IF NOT EXISTS retrofit

**What:** The DB can be updated in place. A version marker plus ordered
idempotent DDL batches replace the create-only `init()`, and every existing
DDL gains `IF NOT EXISTS`.

**Details:**
- `src/schema/meta.rs` — `SchemaMeta(id STRING PRIMARY KEY, version INT64,
  applied_at TIMESTAMP)`, created `IF NOT EXISTS`. `schema_version(conn)` reads
  the `schema` row; binder error (table missing) or empty result → 0.
- `src/schema/migrations.rs` — `batches() -> Vec<(i64, Vec<String>)>`: v1 is
  the complete current schema (entity tables, edge-node tables, relation node
  tables, poly-rels, time-tree MERGE seeds — MERGE is already idempotent) with
  `IF NOT EXISTS` added to every CREATE.
- `schema::init(conn)` becomes: ensure `SchemaMeta` → read version → apply each
  pending batch → write the new version. Every DDL is idempotent, so a
  partially-applied batch converges on re-run; the marker moves only after a
  full batch succeeds. Wrap a batch in `BEGIN TRANSACTION`/`COMMIT` only if
  lbug proves DDL-safe inside a transaction — verify empirically here; if not,
  idempotence is the safety.
- `src/main.rs` — drop the `is_new` file-existence gate; always init (now
  migrate). Re-running the demo against an old `dream.db` upgrades it.
- `init_vector_indexes` already tolerates "already exists" — unchanged.

**Files:** `src/schema/meta.rs` (new), `src/schema/migrations.rs` (new),
`src/schema/mod.rs`, `src/schema/entity_nodes.rs`, `src/schema/edge_nodes.rs`,
`src/schema/relations.rs`, `src/schema/poly_relations.rs`, `src/main.rs`

**Tests (`tests/migrations.rs`):** fresh init reaches the latest version;
re-init is a no-op; a hand-built partial schema (some tables, no marker)
converges; the marker is written once. Task 23 extends this file: a v1 DB with
data migrates to v2 with data intact.

**Depends on:** nothing

---

### Task 21: The axes — anchor corpus + projection math

**What:** `prompts/anchors-v1.md` — the frozen exemplar corpus — and
`src/axes.rs` — pole centroids and signed coordinates. Pure math, no DB.

**Details:**
- The corpus: ~10 first-person exemplar texts per pole (thinking, feeling,
  sensing, intuiting). LLM-drafted, human-reviewed, frozen. Apprehension
  *register*, not description: a thinking-pole text *does* thinking, it never
  says "this is thinking."
- `src/axes.rs`:
  - `Function` enum (Thinking, Feeling, Sensing, Intuiting) with
    `axis() -> Judgment | Perception`
  - `AxisFrame { t_pole, f_pole, s_pole, n_pole }` — pole centroids: the mean
    of a pole's exemplar embeddings, L2-normalized. `AxisFrame::from_exemplars`
    builds it; `coordinates(embedding) -> (judgment, perception)` computes
    `cos(x, own_pole) − cos(x, opposite_pole)` per axis. A local `cosine`
    helper (drift.rs's is private; no cross-module coupling).
  - The constants block is born here and grows with Task 25 — one documented
    place, every value logged per run.

**Files:** `prompts/anchors-v1.md` (new), `src/axes.rs` (new), `src/lib.rs`

**Tests:** unit tests with synthetic vectors — known geometry → known
coordinates; orthogonal exemplars → clean axes; empty exemplar set →
`AxisFrame` is `None` (the graceful degradation case).

**Depends on:** nothing (runs parallel to Task 20)

---

### Task 22: The anchor separability gate

**What:** `tests/anchor_eval.rs` — the go/no-go experiment for pure geometry.

**Details:**
- Live-gated (`WUNICORM_LLM_LIVE=1`, the `llm_live.rs` pattern): embed every
  exemplar from `prompts/anchors-v1.md` via the real server; compute
  (a) opposite-pole centroid cosine (T̂·F̂, Ŝ·N̂), (b) cross-axis centroid
  cosines, (c) within-pole coherence, (d) leave-one-out discriminability — per
  exemplar, does the sign of its coordinate on the owning axis place it in its
  own pole?
- **Pass (proposed; tune after the first real numbers):** opposite-pole cosine
  ≤ 0.75 on both axes and leave-one-out ≥ 8/10 per pole. Fail → the fallback
  (a learned probe over embeddings) is recorded in PAPER_TRAIL and the design
  is revisited before Task 25.
- Hermetic variant (always runs): synthetic exemplar embeddings with known
  geometry assert the *metric code* is correct; the live variant asserts the
  *world* is right.
- The numbers, whatever they are, land in D-002's Context (Task 32).

**Files:** `tests/anchor_eval.rs` (new), `src/axes.rs` (metrics helpers)

**Tests:** the eval is the test.

**Depends on:** Task 21. **Gate:** go/no-go for pure geometry.

---

### Task 23: Migration v2 — the self tables + coordinate columns

**What:** Batch v2: `FunctionAnchor`, `SelfState`, `DreamRun` node tables;
derived-coordinate columns on `Note`; function/provenance columns on `Memory`.
Typed structs + insert mutations.

**Details (DDL sketch):**

```sql
CREATE NODE TABLE IF NOT EXISTS FunctionAnchor(
    id STRING PRIMARY KEY, function STRING, axis STRING, exemplar STRING,
    embedding FLOAT[768], active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP, updated_at TIMESTAMP, layer INT16 DEFAULT 0, context STRING)
CREATE NODE TABLE IF NOT EXISTS SelfState(
    id STRING PRIMARY KEY, subject STRING, window_start TIMESTAMP, window_end TIMESTAMP,
    judgment_coord DOUBLE, perception_coord DOUBLE,
    judgment_core DOUBLE, perception_core DOUBLE,
    judgment_comp DOUBLE, perception_comp DOUBLE,
    rigidity_judgment DOUBLE, rigidity_perception DOUBLE,
    evidence_kept INT64, evidence_dropped INT64,
    learned_at TIMESTAMP, created_at TIMESTAMP, updated_at TIMESTAMP,
    layer INT16 DEFAULT 4, context STRING)
CREATE NODE TABLE IF NOT EXISTS DreamRun(
    id STRING PRIMARY KEY, considered STRING[], dropped STRING[],
    kept_t INT64, kept_f INT64, kept_s INT64, kept_n INT64, dropped_total INT64,
    started_at TIMESTAMP, finished_at TIMESTAMP,
    created_at TIMESTAMP, updated_at TIMESTAMP, layer INT16 DEFAULT 3, context STRING)
ALTER TABLE Note ADD IF NOT EXISTS judgment_coord DOUBLE
ALTER TABLE Note ADD IF NOT EXISTS perception_coord DOUBLE
ALTER TABLE Memory ADD IF NOT EXISTS function_tag STRING
ALTER TABLE Memory ADD IF NOT EXISTS run_id STRING
```

- New DDL consts live in `src/schema/self_nodes.rs` (not stuffed into
  entity_nodes); the batch joins `migrations.rs` as v2.
- Structs in `nodes.rs` mirror the DDL 1:1 (house rule). `SelfState.subject` ∈
  {`user`, `wunicorm`}; `Memory.function_tag` ∈ the four function names —
  enum-as-string, validated at the struct layer.
- `src/mutations/self_ops.rs`: `insert_self_state`, `insert_dream_run`,
  `seed_function_anchors` (deactivates old rows first — anchor history is
  preserved; the auditor is audited).
- Extend `tests/migrations.rs`: a v1 DB with data → v2; ALTERed columns exist
  (probed via `RETURN n.col` — binder error if absent); old rows readable with
  NULL coords/tags.

**Files:** `src/schema/self_nodes.rs` (new), `src/schema/migrations.rs`,
`src/nodes.rs`, `src/mutations/self_ops.rs` (new), `src/mutations/mod.rs`,
`src/lib.rs`

**Tests:** v1→v2 migration with data intact; roundtrip + idempotency per
insert; anchor re-seed deactivates old rows.

**Depends on:** Tasks 20, 21

---

### Task 24: Grounds edges + self trajectory

**What:** The provenance edge (`Grounds`: SelfState → Note/Memory) and the
trajectory rel (`SELF_TRAJECTORY`: SelfState → SelfState).

**Details:**
- `Grounds` goes through the full 7-touchpoint checklist (house pattern):
  `edge_nodes.rs` DDL const, `poly_relations.rs` FROM/TO pair, `edges.rs`
  struct + `EdgeType::Grounds` + `ALL`, `traversal.rs` `EDGES` entry,
  `edge_links.rs` `link_grounds`, `mutations/mod.rs` re-export. `snapshot.rs`
  picks it up automatically via `EdgeType::ALL`.
- `SELF_TRAJECTORY` is a plain rel — `CREATE REL TABLE IF NOT EXISTS
  SELF_TRAJECTORY(FROM SelfState TO SelfState)` — the `TIME_HIERARCHY`
  precedent; `wire_self_trajectory(prev, next)` MERGE mutation in `self_ops.rs`.

**Files:** `src/schema/edge_nodes.rs`, `src/schema/poly_relations.rs`,
`src/schema/self_nodes.rs` (the rel), `src/edges.rs`, `src/nodes.rs` (structs),
`src/queries/traversal.rs`, `src/mutations/edge_links.rs`,
`src/mutations/self_ops.rs`

**Tests:** Grounds edge-node + both rels + idempotent; dangling-ref fence
applies; trajectory wiring; both visible to `graph_snapshot`.

**Depends on:** Task 23

---

### Task 25: Psyche reads — positions, rigidity, function drift

**What:** `src/queries/psyche.rs` — U and W positions, the straightness index,
the 2-dim drift mirror, and the W decomposition math.

**Details:**
- `user_position(conn, window)` — centroid of Note coordinates in the window;
  unmeasurable → `None` (drift's "can't tell" stance).
- `wunicorm_observed(conn, run_id)` — from DreamRun counts:
  `judgment = (kept_t − kept_f) / (kept_t + kept_f + δ·dropped_total)`,
  perception likewise. Drops *dilute* (shrink toward zero), never push.
- `rigidity(conn, subject, axis, span)` — the straightness index: net
  displacement ÷ path length over the trailing SelfStates; 0 when unmeasurable.
- `function_drift(conn, subject, from, to)` — per-axis displacement between two
  window positions; the 2-dim mirror of `queries/drift.rs`.
- The W decomposition as pure functions, wired into the dream in Task 28:
  `core ← α·core + (1−α)·observed`; `comp = −k_eff · U_position` per axis;
  `k_eff = k_base · max(0, rigidity − θ)`; `position = core + comp`, comp
  clamped.
- Constants, one documented block: α (core damping) 0.9, k_base 0.25, θ
  (rigidity threshold) 0.7, δ (drop dilution) 0.5. All logged per run; all
  subject to no-revision-without-observation.

**Files:** `src/queries/psyche.rs` (new), `src/queries/mod.rs`, `src/axes.rs`
(constants)

**Tests:** synthetic fixtures — known coordinates → known centroids; straight
vs zigzag trajectories → rigidity ~1 vs ~0; drops dilute (1 keep + 9 drops
lands near zero, not at the keep's pole); decomposition — rigid one-sided U →
nonzero comp toward balance, balanced U → comp 0, non-rigid one-sided U →
comp 0.

**Depends on:** Tasks 21, 23

---

### Task 26: The extraction seam — memories carry their function

**What:** The extractor names the four functions; every kept memory arrives
tagged.

**Details:**
- `prompts/extract-v1.md`: the reflection question becomes the four functions —
  what to make of it through the lens it actually engages: thinking (where
  this leads), feeling (what it's worth), sensing (what concretely happened),
  intuiting (what pattern it hints at). Each memory carries `function`: the
  dominant lens, required, one of the four.
- `prompts/schema-v1.json`: memories items gain
  `"function": { "enum": ["thinking","feeling","sensing","intuiting"] }` —
  required; `additionalProperties` stays false.
- `src/llm/judge.rs`: `RawMemory` gains `function`; the mapping writes it
  through to `Memory` → `function_tag`.

**Files:** `prompts/extract-v1.md`, `prompts/schema-v1.json`,
`src/llm/judge.rs`, `src/nodes.rs` (field), `src/mutations/content.rs`

**Tests:** hermetic `llm_judge.rs` extension — canned judgments with tags →
`Memory.function_tag` set; a judgment with an unknown tag fails validation at
the mapping step and is recorded (house pattern).

**Depends on:** Task 23 (the column exists)

---

### Task 27: The dreamer's choice ledger

**What:** `run_dream` records what it was offered versus what it kept — per
function.

**Details:**
- The judge/write loop tracks: considered note ids, per-function kept counts,
  and dropped note ids (judged, nothing kept).
- One idempotent `insert_dream_run` per run (id = run_id) — re-runs converge.
- `DreamReport` is untouched here; the self pass owns its report fields
  (Task 28).

**Files:** `src/dreamer.rs`, `src/mutations/self_ops.rs`

**Tests:** `tests/dreamer.rs` with `FixedJudge` — DreamRun written with
correct counts; judged-but-nothing-kept lands in `dropped`; re-dream with the
same run_id → counts stable.

**Depends on:** Tasks 23, 26

---

### Task 28: The self pass — `work_the_self` in the dream

**What:** The seam: a deterministic stage in `run_dream` between the
judge/write loop and observe/reflect. The dream works the self.

**Details:**
- Inputs: this run's choice tallies, the notes, the active `AxisFrame`. Steps,
  all record-don't-throw:
  1. Batch-SET `judgment_coord`/`perception_coord` on the run's notes.
  2. U: window position → `SelfState(subject=user)` +
     `wire_self_trajectory` from the previous user state + `link_grounds` to
     the window's notes.
  3. W: observed position → core update → rigidity-gated compensation →
     `SelfState(subject=wunicorm)` + trajectory + grounds to the run's kept
     memories.
  4. Assemble `SelfReading` — per-axis from→to movement, W's lean, tension
     (axis oscillation) — movement language; numbers stay in the report.
- Unseeded anchors or unmeasurable windows → recorded error,
  `self_pass: None`, the dream continues (it never dies for the self's sake).
- `DreamReport` gains `self_pass: Option<SelfPassReport>` (positions,
  rigidities, k_eff, evidence counts) — the auditor, observed. `is_nightmare`
  unchanged.

**Files:** `src/dreamer.rs`, `src/observability.rs`, `src/queries/psyche.rs`,
`src/mutations/self_ops.rs`

**Tests:** `tests/dreamer.rs` extension — both SelfStates written; trajectory
on the second run; comp = 0 for balanced U and for non-rigid one-sided U;
comp ≠ 0 for rigid one-sided U; anchors missing → `None` + recorded error;
`DreamReport` carries the report.

**Depends on:** Tasks 24, 25, 27

---

### Task 29: The sermon speaks from the self

**What:** The self reading enters the sermon material — day one, movement
language only.

**Details:**
- `reflect_and_speak` passes `SelfReading` into the writer material as a
  `"self"` block: "toward X, away from Y" per axis, W's lean, tension —
  coordinates and counts stripped at material construction (the "numbers
  belong to the ledger" rule moves from prompt to code).
- `prompts/sermon-v1.md`: one new paragraph — the self material is measured
  from the notes and the dreamer's own choices; speak movement and tension;
  **never attribute the user's movement to the system** (no flattery,
  promoted to architecture); never invent function language the material
  doesn't support.
- `MechanicalSermon` gains one self line — the floor speaks what the dreamer
  measured, even when the living voice fails.
- `shelf_check` unchanged — self insights are Insights.

**Files:** `src/llm/sermon.rs`, `src/dreamer.rs`, `prompts/sermon-v1.md`

**Tests:** `tests/sermon.rs` — the writer receives the self block
(RecordedClient); the floor includes the self line; redaction applies; the
self block contains no digits (the strip is enforced, then tested).

**Depends on:** Task 28

---

### Task 30: Full eval — the §8 story grows a self

**What:** `tests/full_eval.rs` extends the job-search story with the self pass.

**Details:**
- Anxious cluster (feeling/sensing-tagged judgments) → strategic cluster
  (thinking/intuiting): assert U moves on both axes between windows; W's core
  creeps complementward; a rigid one-sided fixture trips compensation; the
  quiet-day path leaves self states untouched.
- **Rebuildability**: rebuild from notes + the DreamRun ledger into a second
  DB → identical SelfState positions — the hysteresis survives because the
  choices are recorded.

**Files:** `tests/full_eval.rs`

**Tests:** the eval is the test.

**Depends on:** Tasks 28, 29

---

### Task 31: Demo + README

**What:** `main.rs` prints the self reading after the dream report (U/W
positions, lean, rigidity, comp — numbers are fine here; this is the ledger's
window). README: the ontology table gains the three new tables + Grounds +
SELF_TRAJECTORY; status refresh; fix the stale `FLOAT[518]` → `FLOAT[768]`.

**Files:** `src/main.rs`, `README.md`

**Tests:** existing suite stays green; the demo is manual.

**Depends on:** Tasks 28, 30

---

### Task 32: Paper trail + plan

**What:** `PAPER_TRAIL.md` D-002 — "A two-point Jungian function space in the
memory graph": Decision / Context (the concept and the gate's real numbers) /
Reasoning (the six guards) / What stays / Revisit triggers (separability below
threshold → probe fallback; sermon self-language review after the first weeks
of real dreams). `plan.md`: the memory-layer design subsection — the
consolidated concept, at home in the design conversation.

**Files:** `PAPER_TRAIL.md`, `plan.md`

**Tests:** none (docs).

**Depends on:** Task 22 (the numbers); effectively all.

---

## Commit sequence

```
Task 20 → feat: add schema evolution (versioned migrations, IF NOT EXISTS retrofit)
Task 21 → feat: add the axes (anchor corpus, pole centroids, signed coordinates)
Task 22 → test: add the anchor separability gate
Task 23 → feat: add the self tables (anchors, self states, dream runs, columns)
Task 24 → feat: add grounds edges + self trajectory
Task 25 → feat: add psyche reads (positions, rigidity, function drift)
Task 26 → feat: tag memories by function (the extractor seam)
Task 27 → feat: record the dreamer's choices (dream run ledger)
Task 28 → feat: add the self pass (working the self in the dream)
Task 29 → feat: let the sermon speak from the self
Task 30 → test: extend the full eval (self pass, rebuildable self states)
Task 31 → feat: demo the self reading; refresh the readme
Task 32 → docs: record D-002 (two-point function space) + plan.md subsection
```

Each commit: `cargo clippy --all-targets -- -D warnings` clean, `cargo tst`
green.

---

## Resolved decisions

| Question | Decision |
|----------|----------|
| `chrono` vs `time` | Keep `chrono` in structs; private conversion helpers where lbug needs `time::OffsetDateTime` |
| Embedding dim in tests | Parameterize schema DDL by dim so tests can use small dims |
| `emotions STRING[]` | `Value::Array(LogicalType::String, ...)`; smoke test in mutations |
| Axes construction | Contrastive pole centroids; coordinate = cos(x, own-pole) − cos(x, opposite-pole), per axis |
| Anchor corpus | LLM-drafted, human-reviewed, frozen in `prompts/anchors-v1.md`; seeded + versioned as `FunctionAnchor` rows |
| Function attribution | The extractor tags each memory with one required function enum — the four questions become the four functions |
| W's evidence | Choices only — kept/dropped per function; drops dilute confidence (δ), never push position |
| Choice ledger | Per-run aggregates on one `DreamRun` node (considered/dropped ids, per-function kept counts) |
| Rigidity | Straightness index (net displacement ÷ path length); compensation is gated on rigidity, never direction |
| Compensation | W = damped core (α) + fast component (k_eff, rigidity-gated) targeting dyad balance; constants in one block, logged per run |
| Sermon voice | Speaks from the self from day one; movement language only; numbers stripped at material construction, not by prompt |
| Migration style | Idempotent DDL batches + version marker written after each batch; transactional DDL only if proven safe |
| Anisotropy | Corpus-mean centering (all-but-the-top) before pole geometry — the raw space failed the gate at 0.94 opposite-pole cosine; centered, T·F −0.35, S·N −0.27 |
| Corpus calibration | The gate is the review instrument: v1's first draft failed on 7 cross-register exemplars (contrasts, diagnostics, mood); the revision keeps every pole pure register — LOO 1.0/1.0/1.0/0.9 |
