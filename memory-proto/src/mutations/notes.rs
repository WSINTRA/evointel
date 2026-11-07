use lbug::{Connection, Value};

use crate::db::DbResult;
use crate::nodes::Note;

use super::{chrono_to_time, embedding_to_value};

const INSERT_NOTE: &str = "MERGE (n:Note {id: $id}) SET \
    n.content = $content, \
    n.content_embedding = $embedding, \
    n.created_at = $created_at, \
    n.updated_at = $updated_at, \
    n.layer = $layer, \
    n.context = $context;";

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
