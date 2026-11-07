use lbug::{Connection, Value};

use crate::db::DbResult;
use crate::nodes::{AbstractTime, Time};

use super::{chrono_to_time, chrono_to_time_opt};

// ─── Time ───────────────────────────────────────────────────────────

const INSERT_TIME: &str = "MERGE (n:Time {id: $id}) SET \
    n.granularity = $granularity, \
    n.starts_at = $starts_at, \
    n.ends_at = $ends_at, \
    n.label = $label, \
    n.label_resolved = $label_resolved, \
    n.learned_at = $learned_at, \
    n.expire_at = $expire_at, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

/// Inserts or updates a Time node.
pub fn insert_time(conn: &Connection, time: &Time) -> DbResult<()> {
    let mut stmt = conn.prepare(INSERT_TIME)?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(time.id.clone())),
            ("granularity", Value::String(time.granularity.clone())),
            (
                "starts_at",
                Value::Timestamp(chrono_to_time(&time.starts_at)),
            ),
            ("ends_at", chrono_to_time_opt(time.ends_at.as_ref())),
            ("label", Value::String(time.label.clone())),
            ("label_resolved", Value::String(time.label_resolved.clone())),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&time.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(time.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&time.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&time.updated_at)),
            ),
            ("layer", Value::Int16(time.layer)),
            ("context", Value::String(time.context.clone())),
        ],
    )?;
    Ok(())
}

// ─── AbstractTime ───────────────────────────────────────────────────

const INSERT_ABSTRACT_TIME: &str = "MERGE (n:AbstractTime {id: $id}) SET \
    n.semantics = $semantics, \
    n.label = $label, \
    n.label_resolved = $label_resolved, \
    n.learned_at = $learned_at, \
    n.expire_at = $expire_at, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

/// Inserts or updates an `AbstractTime` node.
pub fn insert_abstract_time(conn: &Connection, at: &AbstractTime) -> DbResult<()> {
    let mut stmt = conn.prepare(INSERT_ABSTRACT_TIME)?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(at.id.clone())),
            ("semantics", Value::String(at.semantics.clone())),
            ("label", Value::String(at.label.clone())),
            ("label_resolved", Value::String(at.label_resolved.clone())),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&at.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(at.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&at.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&at.updated_at)),
            ),
            ("layer", Value::Int16(at.layer)),
            ("context", Value::String(at.context.clone())),
        ],
    )?;
    Ok(())
}

// ─── Time hierarchy wiring ──────────────────────────────────────────

const WIRE_TIME_HIERARCHY: &str = "MATCH (p:Time {id: $parent_id}), (c:Time {id: $child_id}) \
    MERGE (p)-[:TIME_HIERARCHY]->(c);";

/// Wires a parent→child relationship in the time hierarchy.
pub fn wire_time_hierarchy(conn: &Connection, parent_id: &str, child_id: &str) -> DbResult<()> {
    let mut stmt = conn.prepare(WIRE_TIME_HIERARCHY)?;
    conn.execute(
        &mut stmt,
        vec![
            ("parent_id", Value::String(parent_id.to_string())),
            ("child_id", Value::String(child_id.to_string())),
        ],
    )?;
    Ok(())
}
