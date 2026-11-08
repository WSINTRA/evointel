//! Write path for the self layer: anchors, self states, dream runs.
//!
//! Derived, never authoritative — the self pass computes, these
//! persist. All inserts are MERGE-on-id upserts (idempotent: re-dreams
//! converge instead of duplicating). Note coordinates are *not* here:
//! they belong to the self pass's batch update (`set_note_coordinates`),
//! never to `insert_note`, so a re-inserted note never loses its
//! measured position.

use lbug::{Connection, Value};

use crate::db::DbResult;
use crate::nodes::{DreamRun, FunctionAnchor, SelfState};

use super::{chrono_to_time, embedding_to_value, string_list_to_value};

// ─── FunctionAnchor ──────────────────────────────────────────────────

const INSERT_FUNCTION_ANCHOR: &str = "MERGE (n:FunctionAnchor {id: $id}) SET \
    n.function = $function, \
    n.axis = $axis, \
    n.exemplar = $exemplar, \
    n.embedding = $embedding, \
    n.active = $active, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

fn insert_function_anchor(conn: &Connection, anchor: &FunctionAnchor) -> DbResult<()> {
    // The schema spellings are validated here, at the write boundary —
    // a typo'd pole name is a programmer error that must fail loudly,
    // not a silent fifth pole in the graph.
    crate::axes::Function::parse(&anchor.function)
        .unwrap_or_else(|| panic!("unknown function: {}", anchor.function));
    crate::axes::Axis::parse(&anchor.axis)
        .unwrap_or_else(|| panic!("unknown axis: {}", anchor.axis));

    let mut stmt = conn.prepare(INSERT_FUNCTION_ANCHOR)?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(anchor.id.clone())),
            ("function", Value::String(anchor.function.clone())),
            ("axis", Value::String(anchor.axis.clone())),
            ("exemplar", Value::String(anchor.exemplar.clone())),
            ("embedding", embedding_to_value(anchor.embedding.as_ref())),
            ("active", Value::Bool(anchor.active)),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&anchor.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&anchor.updated_at)),
            ),
            ("layer", Value::Int16(anchor.layer)),
            ("context", Value::String(anchor.context.clone())),
        ],
    )?;
    Ok(())
}

/// Seeds a corpus version: every currently active anchor is deactivated
/// first, then the new rows land active. Re-seeding is a version bump —
/// old rows stay in the graph (`active = false`) so anchor history is
/// preserved and the auditor can be audited.
pub fn seed_function_anchors(conn: &Connection, anchors: &[FunctionAnchor]) -> DbResult<()> {
    conn.query("MATCH (a:FunctionAnchor) WHERE a.active = true SET a.active = false;")?;
    for anchor in anchors {
        insert_function_anchor(conn, anchor)?;
    }
    Ok(())
}

// ─── SelfState ───────────────────────────────────────────────────────

const INSERT_SELF_STATE: &str = "MERGE (n:SelfState {id: $id}) SET \
    n.subject = $subject, \
    n.window_start = $window_start, \
    n.window_end = $window_end, \
    n.judgment_coord = $judgment_coord, \
    n.perception_coord = $perception_coord, \
    n.judgment_core = $judgment_core, \
    n.perception_core = $perception_core, \
    n.judgment_comp = $judgment_comp, \
    n.perception_comp = $perception_comp, \
    n.rigidity_judgment = $rigidity_judgment, \
    n.rigidity_perception = $rigidity_perception, \
    n.evidence_kept = $evidence_kept, \
    n.evidence_dropped = $evidence_dropped, \
    n.learned_at = $learned_at, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

pub fn insert_self_state(conn: &Connection, state: &SelfState) -> DbResult<()> {
    let mut stmt = conn.prepare(INSERT_SELF_STATE)?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(state.id.clone())),
            ("subject", Value::String(state.subject.clone())),
            (
                "window_start",
                Value::Timestamp(chrono_to_time(&state.window_start)),
            ),
            (
                "window_end",
                Value::Timestamp(chrono_to_time(&state.window_end)),
            ),
            ("judgment_coord", Value::Double(state.judgment_coord)),
            ("perception_coord", Value::Double(state.perception_coord)),
            ("judgment_core", Value::Double(state.judgment_core)),
            ("perception_core", Value::Double(state.perception_core)),
            ("judgment_comp", Value::Double(state.judgment_comp)),
            ("perception_comp", Value::Double(state.perception_comp)),
            ("rigidity_judgment", Value::Double(state.rigidity_judgment)),
            (
                "rigidity_perception",
                Value::Double(state.rigidity_perception),
            ),
            ("evidence_kept", Value::Int64(state.evidence_kept)),
            ("evidence_dropped", Value::Int64(state.evidence_dropped)),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&state.learned_at)),
            ),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&state.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&state.updated_at)),
            ),
            ("layer", Value::Int16(state.layer)),
            ("context", Value::String(state.context.clone())),
        ],
    )?;
    Ok(())
}

// ─── DreamRun ────────────────────────────────────────────────────────

const INSERT_DREAM_RUN: &str = "MERGE (n:DreamRun {id: $id}) SET \
    n.considered = $considered, \
    n.dropped = $dropped, \
    n.kept_t = $kept_t, \
    n.kept_f = $kept_f, \
    n.kept_s = $kept_s, \
    n.kept_n = $kept_n, \
    n.dropped_total = $dropped_total, \
    n.started_at = $started_at, \
    n.finished_at = $finished_at, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

pub fn insert_dream_run(conn: &Connection, run: &DreamRun) -> DbResult<()> {
    let mut stmt = conn.prepare(INSERT_DREAM_RUN)?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(run.id.clone())),
            ("considered", string_list_to_value(&run.considered)),
            ("dropped", string_list_to_value(&run.dropped)),
            ("kept_t", Value::Int64(run.kept_t)),
            ("kept_f", Value::Int64(run.kept_f)),
            ("kept_s", Value::Int64(run.kept_s)),
            ("kept_n", Value::Int64(run.kept_n)),
            ("dropped_total", Value::Int64(run.dropped_total)),
            (
                "started_at",
                Value::Timestamp(chrono_to_time(&run.started_at)),
            ),
            (
                "finished_at",
                Value::Timestamp(chrono_to_time(&run.finished_at)),
            ),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&run.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&run.updated_at)),
            ),
            ("layer", Value::Int16(run.layer)),
            ("context", Value::String(run.context.clone())),
        ],
    )?;
    Ok(())
}

// ─── Note coordinates (the derived columns) ──────────────────────────

/// Batch-sets the derived coordinates on one note. Owned by the self
/// pass — `insert_note` never touches these columns, so a re-inserted
/// note keeps its measured position.
pub fn set_note_coordinates(
    conn: &Connection,
    note_id: &str,
    judgment: f64,
    perception: f64,
) -> DbResult<()> {
    let mut stmt =
        conn.prepare("MATCH (n:Note {id: $id}) SET n.judgment_coord = $judgment, n.perception_coord = $perception;")?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(note_id.to_string())),
            ("judgment", Value::Double(judgment)),
            ("perception", Value::Double(perception)),
        ],
    )?;
    Ok(())
}
