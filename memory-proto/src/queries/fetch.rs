use chrono::{DateTime, Utc};
use lbug::{Connection, NodeVal, Value};

use crate::db::DbResult;
use crate::nodes::{Entity, Event, Fact, Memory, Note};

/// Converts a lbug `time::OffsetDateTime` back to `chrono::DateTime<Utc>`.
pub(crate) fn time_to_chrono(dt: &time::OffsetDateTime) -> DateTime<Utc> {
    DateTime::from_timestamp(dt.unix_timestamp(), 0).expect("valid timestamp")
}

/// Extracts a property value from a `NodeVal`'s properties by key.
pub(crate) fn get_prop<'a>(props: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    props.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn get_string(props: &[(String, Value)], key: &str) -> String {
    match get_prop(props, key) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// An optional string column: `None` when absent or NULL.
fn get_string_opt(props: &[(String, Value)], key: &str) -> Option<String> {
    match get_prop(props, key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn get_f64(props: &[(String, Value)], key: &str) -> f64 {
    match get_prop(props, key) {
        Some(Value::Double(d)) => *d,
        Some(Value::Float(f)) => f64::from(*f),
        _ => 0.0,
    }
}

fn get_i16(props: &[(String, Value)], key: &str) -> i16 {
    match get_prop(props, key) {
        Some(Value::Int16(v)) => *v,
        #[allow(clippy::cast_possible_truncation)]
        Some(Value::Int32(v)) => *v as i16,
        #[allow(clippy::cast_possible_truncation)]
        Some(Value::Int64(v)) => *v as i16,
        _ => 0,
    }
}

fn get_bool(props: &[(String, Value)], key: &str) -> bool {
    matches!(get_prop(props, key), Some(Value::Bool(true)))
}

fn get_timestamp(props: &[(String, Value)], key: &str) -> DateTime<Utc> {
    match get_prop(props, key) {
        Some(Value::Timestamp(t)) => time_to_chrono(t),
        _ => DateTime::from_timestamp(0, 0).unwrap(),
    }
}

fn get_timestamp_opt(props: &[(String, Value)], key: &str) -> Option<DateTime<Utc>> {
    match get_prop(props, key) {
        Some(Value::Timestamp(t)) => Some(time_to_chrono(t)),
        _ => None,
    }
}

pub(crate) fn get_embedding(props: &[(String, Value)], key: &str) -> Option<Vec<f32>> {
    match get_prop(props, key) {
        Some(Value::Array(_, elems)) => {
            let floats: Vec<f32> = elems
                .iter()
                .filter_map(|v| match v {
                    Value::Float(f) => Some(*f),
                    _ => None,
                })
                .collect();
            if floats.is_empty() {
                None
            } else {
                Some(floats)
            }
        }
        _ => None,
    }
}

fn get_string_list(props: &[(String, Value)], key: &str) -> Vec<String> {
    match get_prop(props, key) {
        Some(Value::List(_, elems) | Value::Array(_, elems)) => elems
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

/// Executes a fetch query and returns the first row as a `NodeVal`, or None.
fn fetch_node(conn: &Connection, table: &str, id: &str) -> DbResult<Option<NodeVal>> {
    let cypher = format!("MATCH (n:{table} {{id: $id}}) RETURN n;");
    let mut stmt = conn.prepare(&cypher)?;
    let mut result = conn.execute(&mut stmt, vec![("id", Value::String(id.to_string()))])?;
    match result.next() {
        Some(row) => match &row[0] {
            Value::Node(node) => Ok(Some(node.clone())),
            _ => Ok(None),
        },
        None => Ok(None),
    }
}

pub fn get_note(conn: &Connection, id: &str) -> DbResult<Option<Note>> {
    let node = fetch_node(conn, "Note", id)?;
    Ok(node.map(|n| {
        let props = n.get_properties();
        Note {
            id: get_string(props, "id"),
            content: get_string(props, "content"),
            content_embedding: get_embedding(props, "content_embedding"),
            created_at: get_timestamp(props, "created_at"),
            updated_at: get_timestamp(props, "updated_at"),
            layer: get_i16(props, "layer"),
            context: get_string(props, "context"),
        }
    }))
}

pub fn get_entity(conn: &Connection, id: &str) -> DbResult<Option<Entity>> {
    let node = fetch_node(conn, "Entity", id)?;
    Ok(node.map(|n| {
        let props = n.get_properties();
        Entity {
            id: get_string(props, "id"),
            label: get_string(props, "label"),
            label_resolved: get_string(props, "label_resolved"),
            label_embedding: get_embedding(props, "label_embedding"),
            learned_at: get_timestamp(props, "learned_at"),
            expire_at: get_timestamp_opt(props, "expire_at"),
            created_at: get_timestamp(props, "created_at"),
            updated_at: get_timestamp(props, "updated_at"),
            layer: get_i16(props, "layer"),
            context: get_string(props, "context"),
        }
    }))
}

pub fn get_fact(conn: &Connection, id: &str) -> DbResult<Option<Fact>> {
    let node = fetch_node(conn, "Fact", id)?;
    Ok(node.map(|n| {
        let props = n.get_properties();
        Fact {
            id: get_string(props, "id"),
            predicate: get_string(props, "predicate"),
            certainty: get_f64(props, "certainty"),
            source: get_string(props, "source"),
            label: get_string(props, "label"),
            label_resolved: get_string(props, "label_resolved"),
            label_embedding: get_embedding(props, "label_embedding"),
            learned_at: get_timestamp(props, "learned_at"),
            expire_at: get_timestamp_opt(props, "expire_at"),
            created_at: get_timestamp(props, "created_at"),
            updated_at: get_timestamp(props, "updated_at"),
            layer: get_i16(props, "layer"),
            context: get_string(props, "context"),
        }
    }))
}

pub fn get_event(conn: &Connection, id: &str) -> DbResult<Option<Event>> {
    let node = fetch_node(conn, "Event", id)?;
    Ok(node.map(|n| {
        let props = n.get_properties();
        Event {
            id: get_string(props, "id"),
            predicate: get_string(props, "predicate"),
            certainty: get_f64(props, "certainty"),
            source: get_string(props, "source"),
            status: get_string(props, "status"),
            is_ongoing: get_bool(props, "is_ongoing"),
            label: get_string(props, "label"),
            label_resolved: get_string(props, "label_resolved"),
            label_embedding: get_embedding(props, "label_embedding"),
            learned_at: get_timestamp(props, "learned_at"),
            expire_at: get_timestamp_opt(props, "expire_at"),
            created_at: get_timestamp(props, "created_at"),
            updated_at: get_timestamp(props, "updated_at"),
            layer: get_i16(props, "layer"),
            context: get_string(props, "context"),
        }
    }))
}

pub fn get_memory(conn: &Connection, id: &str) -> DbResult<Option<Memory>> {
    let node = fetch_node(conn, "Memory", id)?;
    Ok(node.map(|n| {
        let props = n.get_properties();
        Memory {
            id: get_string(props, "id"),
            predicate: get_string(props, "predicate"),
            certainty: get_f64(props, "certainty"),
            source: get_string(props, "source"),
            status: get_string(props, "status"),
            is_ongoing: get_bool(props, "is_ongoing"),
            significance: get_string(props, "significance"),
            emotions: get_string_list(props, "emotions"),
            reflection: get_string(props, "reflection"),
            function_tag: get_string_opt(props, "function_tag"),
            run_id: get_string_opt(props, "run_id"),
            label: get_string(props, "label"),
            label_resolved: get_string(props, "label_resolved"),
            label_embedding: get_embedding(props, "label_embedding"),
            learned_at: get_timestamp(props, "learned_at"),
            expire_at: get_timestamp_opt(props, "expire_at"),
            created_at: get_timestamp(props, "created_at"),
            updated_at: get_timestamp(props, "updated_at"),
            layer: get_i16(props, "layer"),
            context: get_string(props, "context"),
        }
    }))
}
