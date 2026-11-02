use lbug::Connection;

use crate::db::DbResult;

pub const EMBEDDING_DIM: usize = 518;
//A fact like 'a dog belongs to the k9 family' would have predicate "belongs_to", certainty: 1.0,
//source: 'general_knowledge'.
pub const CREATE_NODE_TABLE_FACT: &str = r#"
CREATE NODE TABLE Fact(
    id              STRING PRIMARY KEY,
    predicate       STRING,
    certainty       DOUBLE DEFAULT 1.0,
    source          STRING,
    label           STRING,
    label_resolved  STRING,
    label_embedding FLOAT[518],
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 2,
    context         STRING
)
"#;
// the status field captures, the state of event, 'occured', 'planned', 'cancelled', 'potential',
// the is_going marks in progress
pub const CREATE_NODE_TABLE_EVENT: &str = r#"
CREATE NODE TABLE Event(
    id              STRING PRIMARY KEY,
    predicate       STRING, 
    certainty       DOUBLE DEFAULT 1.0,
    source          STRING,
    status          STRING DEFAULT 'occurred',
    is_ongoing      BOOLEAN DEFAULT FALSE,
    label           STRING,
    label_resolved  STRING,
    label_embedding FLOAT[518],
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 3,
    context         STRING
)
"#;
//Memory - experiences worth keeping!
pub const CREATE_NODE_TABLE_MEMORY: &str = r#"
CREATE NODE TABLE Memory(
    id              STRING PRIMARY KEY,
    predicate       STRING, 
    certainty       DOUBLE DEFAULT 1.0,
    source          STRING,
    status          STRING DEFAULT 'occurred',
    is_ongoing      BOOLEAN DEFAULT FALSE,
    significance    STRING,
    emotions        STRING[],
    reflection      STRING,
    label           STRING,
    label_resolved  STRING,
    label_embedding FLOAT[518],
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 4,
    context         STRING
)
"#;

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
        CREATE_NODE_TABLE_FACT,
        CREATE_NODE_TABLE_EVENT,
        CREATE_NODE_TABLE_MEMORY,
    ] {
        let mut stmt = conn.prepare(ddl)?;
        conn.execute(&mut stmt, vec![])?;
    }
    Ok(())
}
