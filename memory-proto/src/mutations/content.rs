use lbug::{Connection, Value};

use crate::db::DbResult;
use crate::nodes::{Entity, Event, Fact, Memory};

use super::{
    chrono_to_time, chrono_to_time_opt, embedding_to_value, opt_string_to_value,
    string_list_to_value,
};

// ─── Entity ─────────────────────────────────────────────────────────

const INSERT_ENTITY: &str = "MERGE (n:Entity {id: $id}) SET \
    n.label = $label, \
    n.label_resolved = $label_resolved, \
    n.label_embedding = $embedding, \
    n.learned_at = $learned_at, \
    n.expire_at = $expire_at, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

pub fn insert_entity(conn: &Connection, entity: &Entity) -> DbResult<()> {
    let mut stmt = conn.prepare(INSERT_ENTITY)?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(entity.id.clone())),
            ("label", Value::String(entity.label.clone())),
            (
                "label_resolved",
                Value::String(entity.label_resolved.clone()),
            ),
            (
                "embedding",
                embedding_to_value(entity.label_embedding.as_ref()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&entity.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(entity.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&entity.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&entity.updated_at)),
            ),
            ("layer", Value::Int16(entity.layer)),
            ("context", Value::String(entity.context.clone())),
        ],
    )?;
    Ok(())
}

// ─── Fact ───────────────────────────────────────────────────────────

const INSERT_FACT: &str = "MERGE (n:Fact {id: $id}) SET \
    n.predicate = $predicate, \
    n.certainty = $certainty, \
    n.source = $source, \
    n.label = $label, \
    n.label_resolved = $label_resolved, \
    n.label_embedding = $embedding, \
    n.learned_at = $learned_at, \
    n.expire_at = $expire_at, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

pub fn insert_fact(conn: &Connection, fact: &Fact) -> DbResult<()> {
    let mut stmt = conn.prepare(INSERT_FACT)?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(fact.id.clone())),
            ("predicate", Value::String(fact.predicate.clone())),
            ("certainty", Value::Double(fact.certainty)),
            ("source", Value::String(fact.source.clone())),
            ("label", Value::String(fact.label.clone())),
            ("label_resolved", Value::String(fact.label_resolved.clone())),
            (
                "embedding",
                embedding_to_value(fact.label_embedding.as_ref()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&fact.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(fact.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&fact.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&fact.updated_at)),
            ),
            ("layer", Value::Int16(fact.layer)),
            ("context", Value::String(fact.context.clone())),
        ],
    )?;
    Ok(())
}

// ─── Event ──────────────────────────────────────────────────────────

const INSERT_EVENT: &str = "MERGE (n:Event {id: $id}) SET \
    n.predicate = $predicate, \
    n.certainty = $certainty, \
    n.source = $source, \
    n.status = $status, \
    n.is_ongoing = $is_ongoing, \
    n.label = $label, \
    n.label_resolved = $label_resolved, \
    n.label_embedding = $embedding, \
    n.learned_at = $learned_at, \
    n.expire_at = $expire_at, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

pub fn insert_event(conn: &Connection, event: &Event) -> DbResult<()> {
    let mut stmt = conn.prepare(INSERT_EVENT)?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(event.id.clone())),
            ("predicate", Value::String(event.predicate.clone())),
            ("certainty", Value::Double(event.certainty)),
            ("source", Value::String(event.source.clone())),
            ("status", Value::String(event.status.clone())),
            ("is_ongoing", Value::Bool(event.is_ongoing)),
            ("label", Value::String(event.label.clone())),
            (
                "label_resolved",
                Value::String(event.label_resolved.clone()),
            ),
            (
                "embedding",
                embedding_to_value(event.label_embedding.as_ref()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&event.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(event.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&event.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&event.updated_at)),
            ),
            ("layer", Value::Int16(event.layer)),
            ("context", Value::String(event.context.clone())),
        ],
    )?;
    Ok(())
}

// ─── Memory ─────────────────────────────────────────────────────────

const INSERT_MEMORY: &str = "MERGE (n:Memory {id: $id}) SET \
    n.predicate = $predicate, \
    n.certainty = $certainty, \
    n.source = $source, \
    n.status = $status, \
    n.is_ongoing = $is_ongoing, \
    n.significance = $significance, \
    n.emotions = $emotions, \
    n.reflection = $reflection, \
    n.function_tag = $function_tag, \
    n.run_id = $run_id, \
    n.label = $label, \
    n.label_resolved = $label_resolved, \
    n.label_embedding = $embedding, \
    n.learned_at = $learned_at, \
    n.expire_at = $expire_at, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

pub fn insert_memory(conn: &Connection, memory: &Memory) -> DbResult<()> {
    let mut stmt = conn.prepare(INSERT_MEMORY)?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(memory.id.clone())),
            ("predicate", Value::String(memory.predicate.clone())),
            ("certainty", Value::Double(memory.certainty)),
            ("source", Value::String(memory.source.clone())),
            ("status", Value::String(memory.status.clone())),
            ("is_ongoing", Value::Bool(memory.is_ongoing)),
            ("significance", Value::String(memory.significance.clone())),
            ("emotions", string_list_to_value(&memory.emotions)),
            ("reflection", Value::String(memory.reflection.clone())),
            (
                "function_tag",
                opt_string_to_value(memory.function_tag.as_deref()),
            ),
            ("run_id", opt_string_to_value(memory.run_id.as_deref())),
            ("label", Value::String(memory.label.clone())),
            (
                "label_resolved",
                Value::String(memory.label_resolved.clone()),
            ),
            (
                "embedding",
                embedding_to_value(memory.label_embedding.as_ref()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&memory.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(memory.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&memory.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&memory.updated_at)),
            ),
            ("layer", Value::Int16(memory.layer)),
            ("context", Value::String(memory.context.clone())),
        ],
    )?;
    Ok(())
}
