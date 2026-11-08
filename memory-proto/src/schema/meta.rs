//! The schema's version marker — the one row that remembers which
//! migration batch the DB has seen. lbug has no `SHOW TABLES` and no
//! catalog introspection, so the marker *is* the catalog's memory.
//!
//! An absent marker means the DB predates migrations entirely (every DB
//! opened before Task 20): version 0, everything still to apply — the
//! batches are idempotent, so re-applying v1 to an existing schema is a
//! no-op, not an error.

use chrono::{DateTime, Utc};
use lbug::{Connection, Value};
use time::OffsetDateTime;

use crate::db::DbResult;

/// The single marker row's id. One marker per DB — the schema is global.
pub const MARKER_ID: &str = "schema";

/// The marker table. `IF NOT EXISTS` is load-bearing: this is the first
/// statement every open runs, on DBs that have nothing else.
pub const SCHEMA_META_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS SchemaMeta(
    id         STRING PRIMARY KEY,
    version    INT64,
    applied_at TIMESTAMP
)
";

/// chrono → `time::OffsetDateTime`, private per the conversion-helper
/// convention (schema never reaches into mutations).
fn to_lbug_time(dt: &DateTime<Utc>) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(dt.timestamp()).expect("valid unix timestamp")
}

/// Reads the DB's schema version. No marker row → 0 (a pre-migration DB
/// or a brand-new one; both get every batch).
pub fn schema_version(conn: &Connection) -> DbResult<i64> {
    let cypher = format!("MATCH (m:SchemaMeta {{id: '{MARKER_ID}'}}) RETURN m.version;");
    let result = conn.query(&cypher)?;
    for row in result {
        if let Value::Int64(v) = row[0] {
            return Ok(v);
        }
    }
    Ok(0)
}

/// Writes the marker to `version` with the apply time. A MERGE, so it is
/// idempotent and creates the row on first write.
pub fn set_schema_version(conn: &Connection, version: i64) -> DbResult<()> {
    let mut stmt = conn.prepare(
        "MERGE (m:SchemaMeta {id: $id}) SET m.version = $version, m.applied_at = $applied_at;",
    )?;
    conn.execute(
        &mut stmt,
        vec![
            ("id", Value::String(MARKER_ID.to_string())),
            ("version", Value::Int64(version)),
            ("applied_at", Value::Timestamp(to_lbug_time(&Utc::now()))),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_id_is_the_single_row_key() {
        assert_eq!(MARKER_ID, "schema");
    }
}
