# memory-proto

Prototype for Wunicorm v2's memory layer -- a semantic space-time knowledge graph built on [LadybugDB](https://ladybugdb.com/).

## What this is

The memory layer is the most novel part of Wunicorm v2. It models concepts as a multipartite graph where nodes carry both **semantic position** (embedding vectors) and **temporal position** (timestamps), and edges are typed by four pillars:

| Pillar | Relationship |
|---|---|
| **Similarity** | semantic proximity |
| **Containment** | hierarchical / categorical |
| **Property Attribution** | a concept carrying a value |
| **Causality** | one tends to produce/motivate another |

Because time is a dimension of position, a thought can be seen *drifting* across its neighborhood -- which is how the system infers habits and thinking patterns.

## Stack

- **LadybugDB** (`lbug`) -- embedded graph database, Cypher query language, property graph model
- **Rust** -- type safety, memory safety, zero-cost performance
- **Vector search** -- native HNSW index over `FLOAT[518]` embedding properties

## Ontology

Six node types across five layers:

| Layer | Node type | Description |
|---|---|---|
| 0 | Edge nodes | (TBD) |
| 1 | Entity | Universal base type -- everything in the graph is an entity at its core |
| 1 | Time | Concrete temporal spans (granularity, starts_at, ends_at) |
| 1 | AbstractTime | Fuzzy temporal anchors ("future", "soon", "never") |
| 2 | Fact | Declarative knowledge with predicate + certainty |
| 3 | Event | Something that happened or is planned, with status |
| 4 | Memory | Experiences worth keeping -- significance, emotions, reflection |

Content nodes (Entity, Fact, Event, Memory) carry `label_embedding FLOAT[518]` for semantic search.

## Getting started

```bash
cargo run          # opens memory-proto.db, runs schema init
cargo test         # integration tests
cargo clippy       # lint (pedantic)
```

## Project layout

```
src/
  main.rs           # CLI demo
  lib.rs            # re-exports
  db.rs             # MemoryDb -- connection management
  nodes.rs          # typed structs (TBD)
  edges.rs          # typed structs (TBD)
  query.rs          # query helpers (TBD)
  vector.rs         # vector search (TBD)
  schema/
    mod.rs          # init() -- chains all DDL
    entity_nodes.rs # CREATE NODE TABLE: Entity, Time, AbstractTime, Fact, Event, Memory
    edge_nodes.rs   # CREATE NODE TABLE: edge nodes (TBD)
    relations.rs    # CREATE REL TABLE: the four pillars (TBD)
    time_tree.rs    # time tree schema (TBD)
tests/
  integration.rs    # smoke tests
.github/workflows/
  ci.yml            # clippy + test on push/PR
```

## Status

### Done

- [x] Project scaffold (cargo, deps, CI)
- [x] `MemoryDb` -- open/connect to LadybugDB
- [x] Entity node table
- [x] Time node table
- [x] AbstractTime node table
- [x] Fact node table
- [x] Event node table
- [x] Memory node table
- [x] Schema module structure (`entity_nodes`, `edge_nodes`, `relations`, `time_tree`)

### Todo

- [ ] Edge node tables (layer 0)
- [ ] Space-time relations (the four pillars as `CREATE REL TABLE`)
- [ ] Time tree schema (temporal hierarchy)
- [ ] Typed structs in `nodes.rs` / `edges.rs` matching the DDL
- [ ] Vector index init + semantic search in `vector.rs`
- [ ] Query helpers in `query.rs` (neighbors, traversal)
- [ ] Integration evals: populate, traverse, vector search, rebuildability
- [ ] Demo in `main.rs` (insert sample graph, query it)
