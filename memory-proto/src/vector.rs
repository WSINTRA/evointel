use lbug::{Connection, LogicalType, Value};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;

/// Creates HNSW vector indexes on all embedding columns.
///
/// Must be called after `schema::init()` and after the `vector` extension
/// is loaded. Idempotent — silently skips indexes that already exist.
pub fn init_vector_indexes(conn: &Connection) -> DbResult<()> {
    let indexes = [
        ("Entity", "entity_label_idx", "label_embedding"),
        ("Fact", "fact_label_idx", "label_embedding"),
        ("Event", "event_label_idx", "label_embedding"),
        ("Memory", "memory_label_idx", "label_embedding"),
        ("Note", "note_content_idx", "content_embedding"),
    ];

    for (table, index_name, property) in indexes {
        let cypher = format!(
            "CALL CREATE_VECTOR_INDEX('{table}', '{index_name}', '{property}', metric := 'cosine');"
        );
        if let Err(e) = conn.query(&cypher) {
            // Idempotency: ignore "already exists"
            if !e.to_string().contains("already exists") {
                return Err(e.into());
            }
        }
    }

    Ok(())
}

/// A single result from a semantic search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub label: String,
    pub score: f32,
}

/// Maps a node table name to its vector index name.
fn index_for_table(table: &str) -> &'static str {
    match table {
        "Entity" => "entity_label_idx",
        "Fact" => "fact_label_idx",
        "Event" => "event_label_idx",
        "Memory" => "memory_label_idx",
        "Note" => "note_content_idx",
        _ => panic!("No vector index defined for table '{table}'"),
    }
}

/// Performs a semantic search over a node table's embedding column.
///
/// Returns results sorted by distance (most similar first).
/// The `node_label` must be one of: Entity, Fact, Event, Memory, Note.
pub fn semantic_search(
    conn: &Connection,
    query_embedding: &[f32],
    top_k: usize,
    node_label: &str,
) -> DbResult<Vec<SearchResult>> {
    let index_name = index_for_table(node_label);
    let vector_value = Value::List(
        LogicalType::Float,
        query_embedding.iter().map(|f| Value::Float(*f)).collect(),
    );

    let cypher = format!(
        "CALL QUERY_VECTOR_INDEX('{node_label}', '{index_name}', $qv, {top_k}) \
         RETURN node.id, distance;"
    );

    let mut stmt = conn.prepare(&cypher)?;
    let result = conn.execute(&mut stmt, vec![("qv", vector_value)])?;

    let mut results = Vec::new();
    for row in result {
        let id = match &row[0] {
            Value::String(s) => s.clone(),
            _ => continue,
        };
        let distance = match &row[1] {
            Value::Float(f) => *f,
            #[allow(clippy::cast_possible_truncation)]
            Value::Double(d) => *d as f32,
            _ => continue,
        };
        results.push(SearchResult {
            id,
            label: node_label.to_string(),
            score: distance,
        });
    }

    Ok(results)
}
