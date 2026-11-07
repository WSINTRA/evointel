use chrono::{DateTime, Utc};
use lbug::{Connection, LogicalType, Value};

use crate::db::DbResult;
use crate::nodes::Note;
use crate::schema::EMBEDDING_DIM;

const INSERT_NOTE: &str = "MERGE (n:Note {id: $id}) SET \
    n.content = $content, \
    n.content_embedding = $embedding, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

/// Converts a chrono `DateTime<Utc>` to `time::OffsetDateTime` (what lbug expects).
fn chrono_to_time(dt: &DateTime<Utc>) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(dt.timestamp()).expect("valid unix timestamp")
}

/// Converts an `Option<Vec<f32>>` embedding to the appropriate lbug Value.
fn embedding_to_value(embedding: Option<&Vec<f32>>) -> Value {
    match embedding {
        Some(vec) => Value::Array(
            LogicalType::Float,
            vec.iter().map(|f| Value::Float(*f)).collect(),
        ),
        None => Value::Null(LogicalType::Array {
            child_type: Box::new(LogicalType::Float),
            num_elements: EMBEDDING_DIM as u64,
        }),
    }
}

/// Inserts or updates a Note node using MERGE (idempotent).
pub fn insert_note(conn: &Connection, note: &Note) -> DbResult<()> {
    let mut stmt = conn.prepare(INSERT_NOTE)?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(note.id.clone())),
            ("content", Value::String(note.content.clone())),
            (
                "embedding",
                embedding_to_value(note.content_embedding.as_ref()),
            ),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&note.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&note.updated_at)),
            ),
            ("layer", Value::Int16(note.layer)),
            ("context", Value::String(note.context.clone())),
        ],
    )?;
    Ok(())
}
