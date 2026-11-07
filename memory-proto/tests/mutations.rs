use chrono::DateTime;
use lbug::Value;
use memory_proto::mutations::insert_note;
use memory_proto::nodes::Note;
use memory_proto::schema::EMBEDDING_DIM;
use memory_proto::MemoryDb;

fn test_note() -> Note {
    let ts = DateTime::from_timestamp(1700000000, 0).unwrap();
    Note {
        id: "test-note-1".to_string(),
        content: "Hello world".to_string(),
        content_embedding: None,
        created_at: ts,
        updated_at: ts,
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
