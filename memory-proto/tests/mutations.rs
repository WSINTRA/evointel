use chrono::DateTime;
use lbug::Value;
use memory_proto::mutations::{
    insert_abstract_time, insert_entity, insert_event, insert_fact, insert_memory, insert_note,
    insert_time, wire_time_hierarchy,
};
use memory_proto::nodes::{AbstractTime, Entity, Event, Fact, Memory, Note, Time};
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

// ─── Time tests ─────────────────────────────────────────────────────

fn test_time() -> Time {
    Time {
        id: "t-2026-11-03".to_string(),
        granularity: "day".to_string(),
        starts_at: DateTime::from_timestamp(1762214400, 0).unwrap(), // 2026-11-03
        ends_at: Some(DateTime::from_timestamp(1762214400, 0).unwrap()),
        label: "November 3, 2026".to_string(),
        label_resolved: "2026-11-03".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    }
}

#[test]
fn insert_time_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let time = test_time();
    insert_time(&conn, &time).unwrap();

    let mut result = conn
        .query(
            "MATCH (n:Time {id: 't-2026-11-03'}) RETURN n.granularity, n.label, n.label_resolved;",
        )
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::String("day".to_string()));
    assert_eq!(row[1], Value::String("November 3, 2026".to_string()));
    assert_eq!(row[2], Value::String("2026-11-03".to_string()));
}

fn test_abstract_time() -> AbstractTime {
    AbstractTime {
        id: "at-future".to_string(),
        semantics: "unbounded forward".to_string(),
        label: "future".to_string(),
        label_resolved: "future (unbounded)".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    }
}

#[test]
fn insert_abstract_time_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let at = test_abstract_time();
    insert_abstract_time(&conn, &at).unwrap();

    let mut result = conn
        .query("MATCH (n:AbstractTime {id: 'at-future'}) RETURN n.semantics, n.label;")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::String("unbounded forward".to_string()));
    assert_eq!(row[1], Value::String("future".to_string()));
}

#[test]
fn wire_time_hierarchy_creates_rel() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    // Use unique IDs that don't conflict with the time_tree seed data
    let parent = Time {
        id: "t-test-year".to_string(),
        granularity: "year".to_string(),
        starts_at: DateTime::from_timestamp(1767225600, 0).unwrap(),
        ends_at: Some(DateTime::from_timestamp(1798761599, 0).unwrap()),
        label: "2026".to_string(),
        label_resolved: "2026".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    let child = Time {
        id: "t-test-day".to_string(),
        granularity: "day".to_string(),
        starts_at: DateTime::from_timestamp(1762214400, 0).unwrap(),
        ends_at: Some(DateTime::from_timestamp(1762214400, 0).unwrap()),
        label: "Nov 3".to_string(),
        label_resolved: "2026-11-03".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    insert_time(&conn, &parent).unwrap();
    insert_time(&conn, &child).unwrap();

    // Wire them
    wire_time_hierarchy(&conn, "t-test-year", "t-test-day").unwrap();

    // Verify the rel exists
    let mut result = conn
        .query("MATCH (p:Time {id: 't-test-year'})-[:TIME_HIERARCHY]->(c:Time {id: 't-test-day'}) RETURN count(p);")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::Int64(1));
}

// ─── Edge link tests ────────────────────────────────────────────────

fn test_contains() -> memory_proto::edges::Contains {
    memory_proto::edges::Contains {
        id: "edge-contains-1".to_string(),
        containment_types: "entity_mention".to_string(),
        weight: 0.9,
        label: "note mentions paris".to_string(),
        label_resolved: "note→entity:Paris".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "extraction".to_string(),
    }
}

fn test_similar() -> memory_proto::edges::Similar {
    memory_proto::edges::Similar {
        id: "edge-similar-1".to_string(),
        similarity: 0.85,
        sim_context: "same city, different context".to_string(),
        sim_method: "cosine".to_string(),
        label: "paris ~ berlin".to_string(),
        label_resolved: "European capitals".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "inference".to_string(),
    }
}

#[test]
fn link_contains_creates_edge_node_and_rels() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    // Create source and target nodes
    let note = Note {
        id: "note-src".to_string(),
        content: "I visited Paris last summer".to_string(),
        content_embedding: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "travel".to_string(),
    };
    let entity = Entity {
        id: "ent-dest".to_string(),
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

    // Link them
    let from = memory_proto::mutations::NodeRef::new("note-src", "Note");
    let to = memory_proto::mutations::NodeRef::new("ent-dest", "Entity");
    let props = test_contains();
    let edge_id = memory_proto::mutations::link_contains(&conn, &from, &to, &props).unwrap();
    assert_eq!(edge_id, "edge-contains-1");

    // Verify edge node exists with correct props
    let mut result = conn
        .query("MATCH (e:Contains {id: 'edge-contains-1'}) RETURN e.containment_types, e.weight;")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::String("entity_mention".to_string()));
    assert_eq!(row[1], Value::Double(0.9));

    // Verify FROM_Contains rel exists
    let mut result = conn
        .query("MATCH (a:Note {id: 'note-src'})-[:FROM_Contains]->(e:Contains {id: 'edge-contains-1'}) RETURN count(a);")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::Int64(1));

    // Verify TO_Contains rel exists
    let mut result = conn
        .query("MATCH (e:Contains {id: 'edge-contains-1'})-[:TO_Contains]->(b:Entity {id: 'ent-dest'}) RETURN count(b);")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::Int64(1));
}

#[test]
fn link_similar_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    // Create two entities
    let e1 = Entity {
        id: "ent-a".to_string(),
        label: "Paris".to_string(),
        label_resolved: "Paris, France".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "geo".to_string(),
    };
    let e2 = Entity {
        id: "ent-b".to_string(),
        label: "Berlin".to_string(),
        label_resolved: "Berlin, Germany".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "geo".to_string(),
    };
    insert_entity(&conn, &e1).unwrap();
    insert_entity(&conn, &e2).unwrap();

    // Link with Similar
    let from = memory_proto::mutations::NodeRef::new("ent-a", "Entity");
    let to = memory_proto::mutations::NodeRef::new("ent-b", "Entity");
    let props = test_similar();
    memory_proto::mutations::link_similar(&conn, &from, &to, &props).unwrap();

    // Verify edge node
    let mut result = conn
        .query("MATCH (e:Similar {id: 'edge-similar-1'}) RETURN e.similarity, e.sim_method;")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::Double(0.85));
    assert_eq!(row[1], Value::String("cosine".to_string()));

    // Verify both rels
    let mut result = conn
        .query("MATCH (a:Entity {id: 'ent-a'})-[:FROM_Similar]->(e:Similar {id: 'edge-similar-1'})-[:TO_Similar]->(b:Entity {id: 'ent-b'}) RETURN count(e);")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::Int64(1));
}

#[test]
fn link_contains_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let note = Note {
        id: "note-idem".to_string(),
        content: "test".to_string(),
        content_embedding: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "test".to_string(),
    };
    let entity = Entity {
        id: "ent-idem".to_string(),
        label: "Test".to_string(),
        label_resolved: "Test".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    insert_note(&conn, &note).unwrap();
    insert_entity(&conn, &entity).unwrap();

    let from = memory_proto::mutations::NodeRef::new("note-idem", "Note");
    let to = memory_proto::mutations::NodeRef::new("ent-idem", "Entity");
    let props = test_contains();

    // Link twice with same id
    memory_proto::mutations::link_contains(&conn, &from, &to, &props).unwrap();
    memory_proto::mutations::link_contains(&conn, &from, &to, &props).unwrap();

    // Verify only one edge node
    let mut result = conn
        .query("MATCH (e:Contains {id: 'edge-contains-1'}) RETURN count(e);")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::Int64(1));
}

// ─── Causal edge tests ──────────────────────────────────────────────

#[test]
fn link_causes_creates_edge() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    // Create two facts
    let f1 = Fact {
        id: "fact-cause".to_string(),
        predicate: "rains".to_string(),
        certainty: 1.0,
        source: "obs".to_string(),
        label: "it rained".to_string(),
        label_resolved: "rainfall event".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 2,
        context: "weather".to_string(),
    };
    let f2 = Fact {
        id: "fact-effect".to_string(),
        predicate: "floods".to_string(),
        certainty: 0.9,
        source: "obs".to_string(),
        label: "street flooded".to_string(),
        label_resolved: "urban flooding".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 2,
        context: "weather".to_string(),
    };
    insert_fact(&conn, &f1).unwrap();
    insert_fact(&conn, &f2).unwrap();

    // Link with Causes
    let from = memory_proto::mutations::NodeRef::new("fact-cause", "Fact");
    let to = memory_proto::mutations::NodeRef::new("fact-effect", "Fact");
    let props = memory_proto::edges::Causes {
        id: "edge-causes-1".to_string(),
        probability: 0.8,
        strength: 0.9,
        mechanism: "surface runoff".to_string(),
        directness: "direct".to_string(),
        label: "rain causes flooding".to_string(),
        label_resolved: "precipitation→flooding".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "weather".to_string(),
    };
    memory_proto::mutations::link_causes(&conn, &from, &to, &props).unwrap();

    // Verify full path
    let mut result = conn
        .query("MATCH (a:Fact {id: 'fact-cause'})-[:FROM_Causes]->(e:Causes {id: 'edge-causes-1'})-[:TO_Causes]->(b:Fact {id: 'fact-effect'}) RETURN e.probability, e.directness;")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::Double(0.8));
    assert_eq!(row[1], Value::String("direct".to_string()));
}

#[test]
fn link_leads_to_creates_edge() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::schema::init(&conn).unwrap();

    let evt1 = Event {
        id: "evt-lead-src".to_string(),
        predicate: "skipped_meal".to_string(),
        certainty: 1.0,
        source: "note".to_string(),
        status: "occurred".to_string(),
        is_ongoing: false,
        label: "skipped lunch".to_string(),
        label_resolved: "meal skip".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 3,
        context: "health".to_string(),
    };
    let evt2 = Event {
        id: "evt-lead-dst".to_string(),
        predicate: "energy_drop".to_string(),
        certainty: 0.85,
        source: "inference".to_string(),
        status: "potential".to_string(),
        is_ongoing: false,
        label: "afternoon energy crash".to_string(),
        label_resolved: "energy decline".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 3,
        context: "health".to_string(),
    };
    insert_event(&conn, &evt1).unwrap();
    insert_event(&conn, &evt2).unwrap();

    let from = memory_proto::mutations::NodeRef::new("evt-lead-src", "Event");
    let to = memory_proto::mutations::NodeRef::new("evt-lead-dst", "Event");
    let props = memory_proto::edges::LeadsTo {
        id: "edge-leads-1".to_string(),
        probability: 0.7,
        strength: 0.6,
        mechanism: "blood sugar drop".to_string(),
        label: "skipping lunch leads to crash".to_string(),
        label_resolved: "meal skip→energy drop".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "health".to_string(),
    };
    memory_proto::mutations::link_leads_to(&conn, &from, &to, &props).unwrap();

    let mut result = conn
        .query("MATCH (a:Event {id: 'evt-lead-src'})-[:FROM_LeadsTo]->(e:LeadsTo {id: 'edge-leads-1'})-[:TO_LeadsTo]->(b:Event {id: 'evt-lead-dst'}) RETURN e.mechanism;")
        .unwrap();
    let row = result.next().unwrap();
    assert_eq!(row[0], Value::String("blood sugar drop".to_string()));
}
