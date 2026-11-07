use lbug::{Connection, Error};

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
                return Err(Error::FailedQuery(e.to_string()).into());
            }
        }
    }

    Ok(())
}
