# memory-proto

Prototype for Wunicorm v2's memory layer — a semantic space-time knowledge graph on [LadybugDB](https://ladybugdb.com/), plus **the Dreamer**: the overnight pass that reads your notes, structures them into the graph, reflects on it, and writes the morning's message.

## What this is

The memory layer is the most novel part of Wunicorm v2. It models concepts as a multipartite graph where nodes carry both **semantic position** (embedding vectors) and **temporal position** (timestamps), and edges are typed by four pillars:

| Pillar | Relationship |
|---|---|
| **Similarity** | semantic proximity |
| **Containment** | hierarchical / categorical |
| **Property Attribution** | a concept carrying a value |
| **Causality** | one tends to produce/motivate another |

Because time is a dimension of position, a thought can be seen *drifting* across its neighborhood — which is how the system infers habits and thinking patterns. [`queries/drift.rs`](src/queries/drift.rs) implements this read directly: a concept's position in a time window is the centroid of its neighbors' embeddings there, and `concept_drift` compares windows. This is the primitive behind an insight like *"your job search is shifting from anxious to strategic."*

The prototype now runs the full §8 story end to end: notes in one end, a sermon with citations and drift evidence out the other (see [The Dreamer](#the-dreamer)).

## The graph

### Ontology

Seven node types across five layers, plus the input seam:

| Layer | Node type | Description |
|---|---|---|
| — | Note | The user's writing — the input the graph is derived from. Carries `content` + `content_embedding`. |
| 0 | Edge nodes | Edges are *nodes*: 12 types with their own properties (see below) |
| 1 | Entity | Universal base type — everything in the graph is an entity at its core |
| 1 | Time | Concrete temporal spans (granularity, starts_at, ends_at) |
| 1 | AbstractTime | Fuzzy temporal anchors ("future", "soon", "never") |
| 2 | Fact | Declarative knowledge with predicate + certainty |
| 3 | Event | Something that happened or is planned, with status |
| 4 | Memory | Experiences worth keeping — significance, emotions, reflection |

### Edge nodes and polymorphic relations

An edge is not a plain rel — it is its own node, so it can carry properties (`Similar` has `similarity`, `LeadsTo` has `probability` and `strength`, `HasProperty` has `property_name`/`property_value`). The **twelve edge types**:

`Contains` · `Similar` · `HasProperty` · `LeadsTo` · `Causes` · `Prevents` · `BecauseOf` · `Before` · `After` · `During` · `ValidFrom` · `ValidTo`

Each is joined to its endpoints by two polymorphic rel tables (`FROM_<Type>`, `TO_<Type>`) declared as multiple `FROM ... TO ...` pairs — the legal source/target node types per edge type live in [`schema/poly_relations.rs`](src/schema/poly_relations.rs). The write path hides this entirely: callers link nodes with typed structs ([`mutations/edge_links.rs`](src/mutations/edge_links.rs)) and never see the three-step MERGE + FROM + TO dance underneath.

The time tree is a plain `TIME_HIERARCHY` one-to-many rel between `Time` nodes ([`mutations/time_ops.rs`](src/mutations/time_ops.rs) wires it).

### Embeddings

Content nodes (Note, Entity, Fact, Event, Memory) carry `label_embedding FLOAT[518]` for semantic search. The width must match the live embedding model — currently **EmbeddingGemma-2** on a local `llama.cpp` server — so the DDL is *rendered from `EMBEDDING_DIM`*, not static strings (tests run at small dims). The llm client fails loudly if the model's vector width disagrees with the schema, instead of corrupting graph writes downstream.

Vector search uses LadybugDB's HNSW index ([`vector.rs`](src/vector.rs)): `init_vector_indexes` plus `semantic_search` over any node type / embedding field.

## The Dreamer

[`dreamer.rs`](src/dreamer.rs) — one run of the overnight pass, end to end:

```
queue of Notes → Judge (extraction) → idempotent writes → snapshot delta → reflect → Message → outbox
```

1. **Consume the queue** of notes.
2. **Judge each note** through the `Judge` seam — the live judge is the LLM ([`llm/judge.rs`](src/llm/judge.rs)); a deterministic `FixedJudge` exists for tests.
3. **Write nodes and edges idempotently**, validating endpoints against the poly-rel DDL. Node ids are deterministic slugs of the semantic key (`e-job-search`, `t-2026-11-03`), so a concept re-judged next night MERGEs onto the same node instead of duplicating. Entities go through create-vs-link: a label that already resolves in the graph keeps its existing id. The dreamer is the graph's *only writer*.
4. **Observe the delta** against a pre-dream snapshot ([`queries/snapshot.rs`](src/queries/snapshot.rs)) — nodes/edges added, structural similarity to the previous graph.
5. **Reflect** — semantic search for anchor concepts → citations back to source notes → drift reads → notable events.
6. **Speak the morning's message** through the `SermonWriter` seam: `MechanicalSermon` is the floor that cannot fail; `LlmSermonWriter` is the craft above it, and a writer error falls back to the floor, recorded. The result is a `Message` ([`message.rs`](src/message.rs)) — sermon, notification, or attention card — written as JSON to an outbox directory and passed through a **shelf check** (the artifact must exist, be non-empty, and carry *this run's* id, not a stale one).

Every step is observed into a `DreamReport` ([`observability.rs`](src/observability.rs)); the report and its JSONL log live *outside* the graph, because the graph is 100% derived while the run's observations are their own source of truth. `is_nightmare` encodes the durable-core invariants — didn't finish, hit errors, consumed notes but wrote nothing, failed the shelf check, or structural similarity cratering below a configurable floor. Recovery is always a re-run (every write is a MERGE), and the outbox artifact is the handoff to the future email / notification service layer — the dreamer never delivers anything itself.

## The LLM layer

[`llm/`](src/llm/mod.rs) — one dialect, the OpenAI-compatible REST API spoken by `llama.cpp` and most local-model servers. Two endpoints: `/v1/chat/completions` for judgment, `/v1/embeddings` for meaning. Synchronous on purpose — the dreamer is a nightly batch pass, not a request path.

- **The seam** is the `LlmClient` trait: `UreqClient` talks to real servers; `RecordedClient` replays canned responses so tests stay hermetic.
- **Judgment is grammar-constrained**: chat calls carry the extraction schema as `response_format: json_schema`, so structurally invalid JSON cannot occur. The LLM decides *meaning* (labels, pillars, dates); the code decides *mechanics* (ids, timestamps, embeddings).
- **Prompts are data, not code** — `prompts/extract-v1.md`, `schema-v1.json`, `sermon-v1.md`, loaded from the `prompts/` directory at runtime (iterate without recompiling) with a copy baked in at build time.
- **Sampling defaults are the verified recipe** for the local servers: chat `unsloth/Qwen3.8-27B-GGUF` on `localhost:9931`, embeddings `embeddinggemma-2-GGUF` on `localhost:9932`, thinking ON, an 80k-token budget so chain-of-thought cannot starve the content, and a 90-minute timeout (a thinking-heavy judgment runs many minutes at ~18 tok/s).
- **Environment overrides**: `WUNICORM_CHAT_URL`, `WUNICORM_CHAT_MODEL`, `WUNICORM_EMBED_URL`, `WUNICORM_EMBED_MODEL`.
- **Redaction seam**: everything that leaves the memory layer for a model passes through `llm::redact` — currently the identity, because the configured models are fully local. A real redactor must land before *any* remote endpoint is configured (plan.md §4).

## Stack

- **LadybugDB** (`lbug`) — embedded graph database, Cypher query language, property graph model, HNSW vector index
- **Rust** — type safety, memory safety, zero-cost performance
- **Local `llama.cpp` servers** — chat + embeddings, reached over OpenAI-compatible REST via `ureq`

## Getting started

`cargo run` is the §8 demo: three dated notes (the anxious-strategic job-search arc) through the real judge against the real local servers, one sermon out the other side.

```bash
# prerequisite: local llama.cpp servers — chat on :9931, embeddings on :9932
# (or point elsewhere with WUNICORM_CHAT_URL / WUNICORM_EMBED_URL)

cargo run          # the live demo dream (~1-2 min per judgment; writes demo-data/)
cargo test         # hermetic suites — no servers touched (llm_live is opt-in)
cargo clippy       # lint (pedantic)
```

The demo writes `demo-data/dream.db`, `demo-data/outbox/<run>-sermon.json`, and the append-only `demo-data/dream-log.jsonl`. **Re-running the demo is the idempotency demo**: the second dream re-judges the same notes and the graph converges instead of duplicating.

Live probes against the real servers are opt-in and off by default:

```bash
WUNICORM_LLM_LIVE=1 cargo test --test llm_live   # slow — thinking is on, budget is 80k tokens
```

## Project layout

```
src/
  main.rs           # the live demo dream (the §8 story)
  lib.rs            # re-exports
  db.rs             # MemoryDb — connection management (parameterized by embedding dim)
  nodes.rs          # typed structs: Note, Entity, Time, AbstractTime, Fact, Event, Memory
  edges.rs          # typed structs: the 12 edge-node types
  dreamer.rs        # run_dream — the overnight pass; Judge + SermonWriter seams
  message.rs        # Message (sermon/notification/attention card), outbox, shelf check
  observability.rs  # DreamReport, is_nightmare, append-only JSONL log
  extensions.rs     # loads the vector + FTS extensions
  vector.rs         # HNSW index init, semantic_search
  llm/
    mod.rs          # LlmClient seam: UreqClient (live), RecordedClient (hermetic); prompts
    judge.rs        # LlmJudge — the extraction brain (RawJudgment → DreamJudgment)
    sermon.rs       # LlmSermonWriter — the sermon's living voice
  mutations/        # the write path (MERGE-based, idempotent)
    notes.rs        # insert_note
    content.rs      # insert_entity/fact/event/memory
    time_ops.rs     # insert_time, insert_abstract_time, wire_time_hierarchy
    edge_links.rs   # link_* functions — the 3-step edge-node pattern, hidden from callers
  queries/          # the read path
    fetch.rs        # get_note/entity/fact/event/memory by id
    traversal.rs    # neighbors (BFS over edge-node rels), connected_notes
    temporal.rs     # events_in_range
    drift.rs        # concept_activity, concept_drift — habit inference
    snapshot.rs     # graph_snapshot, structural_similarity (Jaccard)
  schema/           # DDL — rendered from EMBEDDING_DIM, not static strings
    entity_nodes.rs # the 5-layer node tables
    edge_nodes.rs   # the 12 edge-node tables
    poly_relations.rs # FROM_*/TO_* rel tables per edge type + time tree
    time_tree.rs    # time tree schema
prompts/            # extract-v1.md, schema-v1.json, sermon-v1.md — data, not code
demo-data/          # created by `cargo run`: dream.db, outbox/, dream-log.jsonl
tests/              # dreamer · drift · full_eval · integration · llm_judge · llm_live (opt-in)
                    # · message · mutations · observability · queries · sermon · snapshot
                    # · vector_search
.github/workflows/  # ci.yml — clippy + hermetic tests on push/PR
```

## Status

### Done

- [x] Project scaffold (cargo, deps, CI)
- [x] Schema: 7 node tables, 12 edge-node tables, polymorphic rel tables, time tree — all parameterized by embedding dim
- [x] Typed structs (`nodes.rs`, `edges.rs`) matching the DDL
- [x] Mutations: notes, content, time nodes + hierarchy wiring, edge linking (the hidden 3-step pattern)
- [x] Queries: fetch-by-id, traversal (neighbors, connected_notes), temporal (events_in_range)
- [x] Vector indexes (HNSW) + semantic search
- [x] Drift reads: concept_activity, concept_drift — the habit-inference primitive
- [x] Graph snapshots + structural similarity
- [x] LLM layer: client seam (live + hermetic), grammar-constrained judge, sermon writer, prompts as data, redaction seam (stub)
- [x] The Dreamer: run_dream end to end — idempotent writes, snapshot delta, reflection with citations, message outbox with shelf check
- [x] Observability: DreamReport, nightmare detector, JSONL log
- [x] Test suites: mutations, queries, vector search, drift, snapshot, message, observability, dreamer, llm_judge, sermon, full_eval, live probes (opt-in)
- [x] Demo in `main.rs`: the §8 story against the real local servers

### Todo

- [ ] Delivery behind the outbox: the email / notification service layer consumes `Message` artifacts
- [ ] Real redactor (emails, phones, names) before *any* remote endpoint is configured
- [ ] Dreamer coverage of the remaining edge types (Causes, Prevents, BecauseOf, the temporal edges) in judgment output
- [ ] The two note spaces (`source: user` vs `source: wunicorm`) — currently one Note input seam
- [ ] The rest of the v2 system: notes app UI, daemon/scheduler, sorter, task engine — designed in [plan.md](../plan.md), out of proto scope
