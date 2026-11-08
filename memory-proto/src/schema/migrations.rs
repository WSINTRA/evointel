//! Versioned schema migrations.
//!
//! Atomicity here is **idempotence, not transactions** — pinned
//! empirically against lbug 0.22.1 (see `tests/migrations.rs`): DDL does
//! commit and roll back transactionally, but a table created inside an
//! open transaction is invisible to later statements in that same
//! transaction (the binder reads the committed catalog). A batch's
//! statements reference each other's tables — CREATE REL TABLE binds the
//! node tables created moments before — so batches must run
//! auto-committed.
//!
//! The safety is that every statement is idempotent by construction:
//! every CREATE carries `IF NOT EXISTS`, every seed is a MERGE. A
//! partially-applied batch converges on re-run, and the version marker
//! only advances after a batch completes — a crash mid-batch leaves the
//! DB at the previous version with a half-built schema the next open
//! finishes building.
//!
//! Adding to the schema = appending a batch here. Never edit an old
//! batch's meaning; a correction is a new batch.

use lbug::Connection;

use crate::db::DbResult;

use super::meta;

/// Every migration batch, in order: the version it produces and the
/// statements that produce it. v1 is the prototype schema, whole.
pub fn batches() -> Vec<(i64, Vec<String>)> {
    let v1: Vec<String> = super::entity_nodes::entity_node_ddls()
        .into_iter()
        .chain(
            super::edge_nodes::ALL_EDGE_NODE_DDLS
                .iter()
                .map(|s| (*s).to_string()),
        )
        .chain(
            super::relations::ALL_RELATION_DDLS
                .iter()
                .map(|s| (*s).to_string()),
        )
        .chain(
            super::poly_relations::ALL_POLY_RELATIONS
                .iter()
                .map(|s| (*s).to_string()),
        )
        .chain(
            super::time_tree::ALL_TIME_TREE_DDLS
                .iter()
                .map(|s| (*s).to_string()),
        )
        .collect();
    vec![(1, v1)]
}

/// The version a fully-migrated DB carries.
pub fn latest_version() -> i64 {
    batches().last().map_or(0, |(v, _)| *v)
}

/// Brings the DB up to [`latest_version`]: creates the marker, applies
/// every batch newer than the marker, advances the marker after each
/// batch. Idempotent — safe to run on every open, fresh or old.
pub fn migrate(conn: &Connection) -> DbResult<i64> {
    conn.query(meta::SCHEMA_META_T)?;
    let mut current = meta::schema_version(conn)?;
    for (version, ddls) in batches() {
        if version <= current {
            continue;
        }
        for ddl in &ddls {
            conn.query(ddl)?;
        }
        meta::set_schema_version(conn, version)?;
        current = version;
    }
    Ok(current)
}
