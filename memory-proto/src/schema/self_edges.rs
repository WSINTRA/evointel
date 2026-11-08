//! The self layer's edge schema (migration batch v3): the `Grounds`
//! provenance edge-node and the `SELF_TRAJECTORY` rel.
//!
//! `Grounds` is how a positional claim traces to its evidence — a
//! `SelfState` (either subject) linked to the notes that filled its
//! window (user) or the memories its run chose to keep (wunicorm).
//! `evidence_kind` records which: `window` or `choice`.
//!
//! `SELF_TRAJECTORY` is a plain rel (the `TIME_HIERARCHY` precedent):
//! consecutive same-subject `SelfState`s, the line the self has walked.
//!
//! New edge types join the schema as **new batches** — the `ALL_*`
//! arrays in `edge_nodes.rs` / `poly_relations.rs` are the frozen v1
//! world and are never appended to; a correction is a new batch.

/// The provenance edge-node: a `SelfState` grounded in its evidence.
pub const GROUNDS_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS Grounds(
    id             STRING PRIMARY KEY,
    evidence_kind  STRING,
    label          STRING,
    label_resolved STRING,
    learned_at     TIMESTAMP,
    expire_at      TIMESTAMP,
    created_at     TIMESTAMP,
    updated_at     TIMESTAMP,
    layer          INT16 DEFAULT 0,
    context        STRING
)
";

/// `Grounds` poly-rels: `SelfState` → Grounds → Note/Memory.
pub const GROUNDS_RELS: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_Grounds(
    FROM SelfState TO Grounds,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_Grounds(
    FROM Grounds TO Note,
    FROM Grounds TO Memory,
    role STRING DEFAULT 'target'
);
";

/// Consecutive same-subject self states — the trajectory.
pub const SELF_TRAJECTORY_T: &str =
    "CREATE REL TABLE IF NOT EXISTS SELF_TRAJECTORY(FROM SelfState TO SelfState);";

/// The full v3 batch.
pub fn batch() -> Vec<String> {
    [
        GROUNDS_T.to_string(),
        GROUNDS_RELS.to_string(),
        SELF_TRAJECTORY_T.to_string(),
    ]
    .into_iter()
    .collect()
}
