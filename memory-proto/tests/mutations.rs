use chrono::DateTime;
use lbug::Value;
use memory_proto::mutations::{
    insert_entity, insert_event, insert_fact, insert_memory, insert_note,
};
use memory_proto::nodes::{Entity, Event, Fact, Memory, Note};
use memory_proto::schema::EMBEDDING_DIM;
use memory_proto::MemoryDb;

fn ts() -> DateTime<chrono::Utc> {
    DateTime::from_timestamp(1700000000, 0).unwrap()
}

// ─── Note tests ─────────────────────────────────────────────────────

fn test_note() -> Note {
    Note {
        id: "test-note-1".to_string(),
        content: "Hello world".to_string(),
        content_embedding: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "test".to_string(),
    }
}

#[test]
fn insert_note_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let note = test_note();
    insert_note(&conn, &note).unwrap();

    let mut result = conn
        .query("MATCH (n:Note {id: 'test-note-1'}) RETURN n.content, n.layer, n.context;")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::String("Hello world".to_string()));
    assert_eq!(row[1], Value::Int16(0));
    assert_eq!(row[2], Value::String("test".to_string()));
}

#[test]
fn insert_note_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let note = test_note();
    insert_note(&conn, &note).unwrap();
    insert_note(&conn, &note).unwrap();

    let mut result = conn
        .query("MATCH (n:Note) WHERE n.id = 'test-note-1' RETURN count(n);")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::Int64(1));
}

// ─── Entity tests ───────────────────────────────────────────────────

fn test_entity() -> Entity {
    Entity {
        id: "ent-1".to_string(),
        label: "Paris".to_string(),
        label_resolved: "Paris, France".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "travel".to_string(),
    }
}

#[test]
fn insert_entity_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let entity = test_entity();
    insert_entity(&conn, &entity).unwrap();

    let mut result = conn
        .query(
            "MATCH (n:Entity {id: 'ent-1'}) RETURN n.label, n.label_resolved, n.layer, n.context;",
        )
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::String("Paris".to_string()));
    assert_eq!(row[1], Value::String("Paris, France".to_string()));
    assert_eq!(row[2], Value::Int16(1));
    assert_eq!(row[3], Value::String("travel".to_string()));
}

// ─── Fact tests ─────────────────────────────────────────────────────

fn test_fact() -> Fact {
    Fact {
        id: "fact-1".to_string(),
        predicate: "belongs_to".to_string(),
        certainty: 0.95,
        source: "general_knowledge".to_string(),
        label: "dog belongs to k9 family".to_string(),
        label_resolved: "canine family".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 2,
        context: "biology".to_string(),
    }
}

#[test]
fn insert_fact_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let fact = test_fact();
    insert_fact(&conn, &fact).unwrap();

    let mut result = conn
        .query("MATCH (n:Fact {id: 'fact-1'}) RETURN n.predicate, n.certainty, n.source, n.layer;")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::String("belongs_to".to_string()));
    assert_eq!(row[1], Value::Double(0.95));
    assert_eq!(row[2], Value::String("general_knowledge".to_string()));
    assert_eq!(row[3], Value::Int16(2));
}

// ─── Event tests ────────────────────────────────────────────────────

fn test_event() -> Event {
    Event {
        id: "evt-1".to_string(),
        predicate: "attended".to_string(),
        certainty: 1.0,
        source: "note-42".to_string(),
        status: "occurred".to_string(),
        is_ongoing: false,
        label: "went to conference".to_string(),
        label_resolved: "tech conference attendance".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 3,
        context: "work".to_string(),
    }
}

#[test]
fn insert_event_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let event = test_event();
    insert_event(&conn, &event).unwrap();

    let mut result = conn
        .query("MATCH (n:Event {id: 'evt-1'}) RETURN n.predicate, n.status, n.is_ongoing, n.layer;")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::String("attended".to_string()));
    assert_eq!(row[1], Value::String("occurred".to_string()));
    assert_eq!(row[2], Value::Bool(false));
    assert_eq!(row[3], Value::Int16(3));
}

// ─── Memory tests ───────────────────────────────────────────────────

fn test_memory() -> Memory {
    Memory {
        id: "mem-1".to_string(),
        predicate: "learned".to_string(),
        certainty: 0.9,
        source: "note-17".to_string(),
        status: "occurred".to_string(),
        is_ongoing: false,
        significance: "first time understanding the concept".to_string(),
        emotions: vec!["excitement".to_string(), "determined".to_string()],
        reflection: "feels like a breakthrough moment".to_string(),
        label: "learned rust ownership".to_string(),
        label_resolved: "rust ownership model comprehension".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 4,
        context: "coding".to_string(),
    }
}

#[test]
fn insert_memory_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let memory = test_memory();
    insert_memory(&conn, &memory).unwrap();

    let mut result = conn
        .query("MATCH (n:Memory {id: 'mem-1'}) RETURN n.predicate, n.significance, n.reflection, n.layer;")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::String("learned".to_string()));
    assert_eq!(
        row[1],
        Value::String("first time understanding the concept".to_string())
    );
    assert_eq!(
        row[2],
        Value::String("feels like a breakthrough moment".to_string())
    );
    assert_eq!(row[3], Value::Int16(4));
}

#[test]
fn insert_memory_with_emotions() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let memory = test_memory();
    insert_memory(&conn, &memory).unwrap();

    let mut result = conn
        .query("MATCH (n:Memory {id: 'mem-1'}) RETURN n.emotions;")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(
        row[0],
        Value::List(
            lbug::LogicalType::String,
            vec![
                Value::String("excitement".to_string()),
                Value::String("determined".to_string()),
            ]
        )
    );
}
