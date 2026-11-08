//! The self layer's node tables (migration batch v2): the anchors the
//! function axes are seeded from, the self's positions over time, and
//! the dreamer's per-run choice ledger.
//!
//! Everything here is **derived, never authoritative** — coordinates and
//! positions are computed from notes and recorded choices, and every
//! positional claim traces to nodes via `Grounds` edges (Task 24). The
//! `FLOAT[{dim}]` width renders from [`EMBEDDING_DIM`], same as every
//! other embedding column — one source of truth.

use super::entity_nodes::EMBEDDING_DIM;

/// One exemplar of one pole of one axis. Seeded from
/// `prompts/anchors-v1.md`; re-seeding deactivates old rows (never
/// deletes — anchor history is preserved; the auditor is audited).
pub const FUNCTION_ANCHOR_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS FunctionAnchor(
    id         STRING PRIMARY KEY,
    function   STRING,
    axis       STRING,
    exemplar   STRING,
    embedding  FLOAT[{dim}],
    active     BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    layer      INT16 DEFAULT 0,
    context    STRING
)
";

/// One point in function space for one subject (`user` | `wunicorm`) in
/// one window: the observed position (`*_coord`), the Wunicorm
/// decomposition (`*_core`, the damped hysteresis; `*_comp`, the
/// rigidity-gated fast component), and the rigidity reading the
/// compensation was gated on. User states carry only `*_coord` (read,
/// never steered); Wunicorm states carry the full decomposition.
pub const SELF_STATE_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS SelfState(
    id                  STRING PRIMARY KEY,
    subject             STRING,
    window_start        TIMESTAMP,
    window_end          TIMESTAMP,
    judgment_coord      DOUBLE,
    perception_coord    DOUBLE,
    judgment_core       DOUBLE,
    perception_core     DOUBLE,
    judgment_comp       DOUBLE,
    perception_comp     DOUBLE,
    rigidity_judgment   DOUBLE,
    rigidity_perception DOUBLE,
    evidence_kept       INT64,
    evidence_dropped    INT64,
    learned_at          TIMESTAMP,
    created_at          TIMESTAMP,
    updated_at          TIMESTAMP,
    layer               INT16 DEFAULT 4,
    context             STRING
)
";

/// The dreamer's per-run choice ledger: what it was offered versus what
/// it kept, per function. The "discarded is data" channel — Wunicorm's
/// position is measured from these choices, and drops dilute its
/// confidence rather than pushing its position.
pub const DREAM_RUN_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS DreamRun(
    id            STRING PRIMARY KEY,
    considered    STRING[],
    dropped       STRING[],
    kept_t        INT64 DEFAULT 0,
    kept_f        INT64 DEFAULT 0,
    kept_s        INT64 DEFAULT 0,
    kept_n        INT64 DEFAULT 0,
    dropped_total INT64 DEFAULT 0,
    started_at    TIMESTAMP,
    finished_at   TIMESTAMP,
    created_at    TIMESTAMP,
    updated_at    TIMESTAMP,
    layer         INT16 DEFAULT 3,
    context       STRING
)
";

/// v2's additions to existing tables — the derived note coordinates
/// (written only by the self pass, never by `insert_note`) and the
/// memory's function attribution + provenance (written by the judge
/// seam, Task 26). One column per statement: lbug's ALTER grammar.
pub const NOTE_ADD_JUDGMENT: &str = "ALTER TABLE Note ADD IF NOT EXISTS judgment_coord DOUBLE;";
pub const NOTE_ADD_PERCEPTION: &str = "ALTER TABLE Note ADD IF NOT EXISTS perception_coord DOUBLE;";
pub const MEMORY_ADD_FUNCTION: &str = "ALTER TABLE Memory ADD IF NOT EXISTS function_tag STRING;";
pub const MEMORY_ADD_RUN: &str = "ALTER TABLE Memory ADD IF NOT EXISTS run_id STRING;";

/// The full v2 batch: the three new tables plus the column additions.
pub fn self_batch() -> Vec<String> {
    [
        FUNCTION_ANCHOR_T.replace("{dim}", &EMBEDDING_DIM.to_string()),
        SELF_STATE_T.to_string(),
        DREAM_RUN_T.to_string(),
        NOTE_ADD_JUDGMENT.to_string(),
        NOTE_ADD_PERCEPTION.to_string(),
        MEMORY_ADD_FUNCTION.to_string(),
        MEMORY_ADD_RUN.to_string(),
    ]
    .into_iter()
    .collect()
}
