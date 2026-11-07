use lbug::Connection;

use crate::db::DbResult;

const EXTENSIONS: &[&str] = &["vector", "fts"];

pub fn load_extensions(conn: &Connection) -> DbResult<()> {
    for ext in EXTENSIONS {
        conn.query(&format!("INSTALL {ext};"))?;
        conn.query(&format!("LOAD {ext};"))?;
    }
    Ok(())
}
