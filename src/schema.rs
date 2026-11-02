use lbug::Connection;

use crate::db::DbResult;

pub const EMBEDDING_DIM: usize = 518;

pub const CREATE_NODE_TABLE_ENTITY: &str = r#"
CREATE NODE TABLE Entity(
    id              STRING PRIMARY KEY,
    label           STRING,
    label_resolved  STRING,
    label_embedding FLOAT[518],
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 1,
    context         STRING
)
"#;
pub const CREATE_NODE_TABLE_TIME: &str = r#"
CREATE NODE TABLE Time(
    id              STRING PRIMARY KEY,
    granularity     STRING,
    starts_at       TIMESTAMP,
    ends_at         TIMESTAMP,
    label           STRING,
    label_resolved  STRING,
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 1,
    context         STRING
)
"#;
// Examples include, 'future', 'soon', 'never'. Let the agent anchor events to fuzzy temporal events
pub const CREATE_NODE_TABLE_ABSTRACT_TIME: &str = r#"
CREATE NODE TABLE AbstractTime(
    id              STRING PRIMARY KEY,
    semantics       STRING,
    label           STRING,
    label_resolved  STRING,
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 1,
    context         STRING
)
"#;
pub fn init(conn: &Connection) -> DbResult<()> {
    for ddl in [
        CREATE_NODE_TABLE_ENTITY,
        CREATE_NODE_TABLE_TIME,
        CREATE_NODE_TABLE_ABSTRACT_TIME,
    ] {
        let mut stmt = conn.prepare(ddl)?;
        conn.execute(&mut stmt, vec![])?;
    }
    Ok(())
}
