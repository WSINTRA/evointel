use chrono::DateTime;
use memory_proto::mutations::{insert_entity, insert_note};
use memory_proto::nodes::{Entity, Note};
use memory_proto::queries::{get_entity, get_note};
use memory_proto::schema::EMBEDDING_DIM;
use memory_proto::MemoryDb;

fn ts() -> DateTime<chrono::Utc> {
    DateTime::from_timestamp(1700000000, 0).unwrap()
}

#[test]
fn get_entity_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let entity = Entity {
        id: "ent-fetch".to_string(),
        label: "Tokyo".to_string(),
        label_resolved: "Tokyo, Japan".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "travel".to_string(),
    };
    insert_entity(&conn, &entity).unwrap();

    let fetched = get_entity(&conn, "ent-fetch")
        .unwrap()
        .expect("entity should exist");
    assert_eq!(fetched.id, "ent-fetch");
    assert_eq!(fetched.label, "Tokyo");
    assert_eq!(fetched.label_resolved, "Tokyo, Japan");
    assert_eq!(fetched.context, "travel");
    assert_eq!(fetched.layer, 1);
    assert!(fetched.expire_at.is_none());
    assert!(fetched.label_embedding.is_none());
}

#[test]
fn get_entity_missing_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let fetched = get_entity(&conn, "nonexistent-id").unwrap();
    assert!(fetched.is_none());
}

#[test]
fn get_note_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let note = Note {
        id: "note-fetch".to_string(),
        content: "Remember to buy milk".to_string(),
        content_embedding: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "groceries".to_string(),
    };
    insert_note(&conn, &note).unwrap();

    let fetched = get_note(&conn, "note-fetch")
        .unwrap()
        .expect("note should exist");
    assert_eq!(fetched.id, "note-fetch");
    assert_eq!(fetched.content, "Remember to buy milk");
    assert_eq!(fetched.context, "groceries");
    assert_eq!(fetched.layer, 0);
    assert!(fetched.content_embedding.is_none());
}
