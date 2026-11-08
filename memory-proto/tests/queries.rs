use chrono::DateTime;
use memory_proto::MemoryDb;
use memory_proto::mutations::{
    NodeRef, insert_entity, insert_event, insert_note, link_contains, link_similar,
};
use memory_proto::nodes::{Entity, Event, Note};
use memory_proto::queries::{connected_notes, events_in_range, get_entity, get_note, neighbors};
use memory_proto::schema::EMBEDDING_DIM;

fn ts() -> DateTime<chrono::Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}

fn ts_offset(secs: i64) -> DateTime<chrono::Utc> {
    DateTime::from_timestamp(1_700_000_000 + secs, 0).unwrap()
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

// ─── Traversal tests ────────────────────────────────────────────────

#[test]
fn neighbors_depth_1() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    // Create A (Note), B (Entity), C (Entity)
    // A -[Similar]-> B -[Contains]-> C
    let a = Note {
        id: "node-a".to_string(),
        content: "A".to_string(),
        content_embedding: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "test".to_string(),
    };
    let b = Entity {
        id: "node-b".to_string(),
        label: "B".to_string(),
        label_resolved: "B".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    let c = Entity {
        id: "node-c".to_string(),
        label: "C".to_string(),
        label_resolved: "C".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    insert_note(&conn, &a).unwrap();
    insert_entity(&conn, &b).unwrap();
    insert_entity(&conn, &c).unwrap();

    // A -[Similar]-> B
    let sim = memory_proto::edges::Similar {
        id: "edge-ab".to_string(),
        similarity: 0.9,
        sim_context: "test".to_string(),
        sim_method: "cosine".to_string(),
        label: "a~b".to_string(),
        label_resolved: "a~b".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "test".to_string(),
    };
    link_similar(
        &conn,
        &NodeRef::new("node-a", "Note"),
        &NodeRef::new("node-b", "Entity"),
        &sim,
    )
    .unwrap();

    // B -[Similar]-> C
    // (Entity→Entity Contains isn't allowed by the DDL — FROM_Contains
    // sources are Memory|Event|Fact|Note|Time. Similar allows Entity→Entity.)
    let similar_bc = memory_proto::edges::Similar {
        id: "edge-bc".to_string(),
        similarity: 0.7,
        sim_context: "test".to_string(),
        sim_method: "cosine".to_string(),
        label: "b~c".to_string(),
        label_resolved: "b~c".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "test".to_string(),
    };
    link_similar(
        &conn,
        &NodeRef::new("node-b", "Entity"),
        &NodeRef::new("node-c", "Entity"),
        &similar_bc,
    )
    .unwrap();

    // neighbors(A, 1) should return B
    let result = neighbors(&conn, &NodeRef::new("node-a", "Note"), 1).unwrap();
    let ids: Vec<String> = result
        .iter()
        .map(|(n, _)| {
            n.get_properties()
                .iter()
                .find(|(k, _)| k == "id")
                .map(|(_, v)| {
                    if let lbug::Value::String(s) = v {
                        s.clone()
                    } else {
                        String::new()
                    }
                })
                .unwrap_or_default()
        })
        .collect();
    assert!(
        ids.contains(&"node-b".to_string()),
        "depth 1 should include B, got: {ids:?}"
    );
    assert!(
        !ids.contains(&"node-c".to_string()),
        "depth 1 should NOT include C"
    );

    // neighbors(A, 2) should return B and C
    let result2 = neighbors(&conn, &NodeRef::new("node-a", "Note"), 2).unwrap();
    let ids2: Vec<String> = result2.iter().map(|(n, _)| node_id_of(n)).collect();
    assert!(
        ids2.contains(&"node-b".to_string()) && ids2.contains(&"node-c".to_string()),
        "depth 2 should include B and C, got: {ids2:?}"
    );
}

fn node_id_of(n: &lbug::NodeVal) -> String {
    n.get_properties()
        .iter()
        .find(|(k, _)| k == "id")
        .map(|(_, v)| {
            if let lbug::Value::String(s) = v {
                s.clone()
            } else {
                String::new()
            }
        })
        .unwrap_or_default()
}

#[test]
fn connected_notes_finds_source() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    // Note -[Contains]-> Entity
    let note = Note {
        id: "note-cn".to_string(),
        content: "I love Paris".to_string(),
        content_embedding: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "travel".to_string(),
    };
    let entity = Entity {
        id: "ent-cn".to_string(),
        label: "Paris".to_string(),
        label_resolved: "Paris, France".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "travel".to_string(),
    };
    insert_note(&conn, &note).unwrap();
    insert_entity(&conn, &entity).unwrap();

    let contains = memory_proto::edges::Contains {
        id: "edge-cn".to_string(),
        containment_types: "mention".to_string(),
        weight: 1.0,
        label: "note mentions paris".to_string(),
        label_resolved: "note→paris".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "test".to_string(),
    };
    link_contains(
        &conn,
        &NodeRef::new("note-cn", "Note"),
        &NodeRef::new("ent-cn", "Entity"),
        &contains,
    )
    .unwrap();

    let notes = connected_notes(&conn, "ent-cn").unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].id, "note-cn");
    assert_eq!(notes[0].content, "I love Paris");
}

// ─── Temporal tests ─────────────────────────────────────────────────

#[test]
fn events_in_range_filters_correctly() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    // 3 events at different times
    let mk_event = |id: &str, offset: i64| Event {
        id: id.to_string(),
        predicate: "test".to_string(),
        certainty: 1.0,
        source: "test".to_string(),
        status: "occurred".to_string(),
        is_ongoing: false,
        label: id.to_string(),
        label_resolved: id.to_string(),
        label_embedding: None,
        learned_at: ts_offset(offset),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 3,
        context: "test".to_string(),
    };

    insert_event(&conn, &mk_event("evt-early", 0)).unwrap();
    insert_event(&conn, &mk_event("evt-mid", 3600)).unwrap();
    insert_event(&conn, &mk_event("evt-late", 7200)).unwrap();

    // Query range that includes evt-early and evt-mid but not evt-late
    let start = ts_offset(-100);
    let end = ts_offset(5000);
    let events = events_in_range(&conn, start, end).unwrap();
    let ids: Vec<&str> = events.iter().map(|e| e.id.as_str()).collect();
    assert!(ids.contains(&"evt-early"));
    assert!(ids.contains(&"evt-mid"));
    assert!(!ids.contains(&"evt-late"));
    assert_eq!(events.len(), 2);
}
