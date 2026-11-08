use chrono::{DateTime, Utc};
use memory_proto::MemoryDb;
use memory_proto::dreamer::{DreamJudgment, DreamSpec, FixedJudge, run_dream};
use memory_proto::edges::Contains;
use memory_proto::message::shelf_check;
use memory_proto::mutations::NodeRef;
use memory_proto::nodes::{Entity, Note};
use memory_proto::observability::{is_nightmare, read_dream_reports};
use memory_proto::schema::EMBEDDING_DIM;

const T: i64 = 1_700_000_000;

fn ts(t: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(t, 0).unwrap()
}

fn one_hot(idx: usize) -> Vec<f32> {
    let mut v = vec![0.0; EMBEDDING_DIM];
    v[idx] = 1.0;
    v
}

fn open_test_db(path: &std::path::Path) -> MemoryDb {
    let db = MemoryDb::open(path, EMBEDDING_DIM).unwrap();
    {
        let conn = db.connect().unwrap();
        memory_proto::extensions::load_extensions(&conn).unwrap();
        memory_proto::schema::init(&conn).unwrap();
        memory_proto::vector::init_vector_indexes(&conn).unwrap();
    }
    db
}

fn test_note(id: &str, content: &str) -> Note {
    Note {
        id: id.to_string(),
        content: content.to_string(),
        content_embedding: None,
        created_at: ts(T),
        updated_at: ts(T),
        layer: 0,
        context: "dreamer-test".to_string(),
    }
}

fn test_entity(id: &str, name: &str, embedding: Vec<f32>) -> Entity {
    Entity {
        id: id.to_string(),
        label: name.to_string(),
        label_resolved: name.to_string(),
        label_embedding: Some(embedding),
        learned_at: ts(T),
        expire_at: None,
        created_at: ts(T),
        updated_at: ts(T),
        layer: 1,
        context: "dreamer-test".to_string(),
    }
}

fn contains_edge(id: &str) -> Contains {
    Contains {
        id: id.to_string(),
        containment_types: "mention".to_string(),
        weight: 1.0,
        label: "contains".to_string(),
        label_resolved: "note mentions concept".to_string(),
        learned_at: ts(T),
        expire_at: None,
        created_at: ts(T),
        updated_at: ts(T),
        layer: 1,
        context: "dreamer-test".to_string(),
    }
}

fn spec(
    run_id: &str,
    queue: Vec<Note>,
    anchor: Vec<f32>,
    outbox: &std::path::Path,
    log: &std::path::Path,
) -> DreamSpec {
    DreamSpec {
        run_id: run_id.to_string(),
        queue,
        anchor_embedding: anchor,
        drift_windows: None,
        outbox: outbox.to_path_buf(),
        dream_log: log.to_path_buf(),
    }
}

#[test]
fn dream_writes_judgments_and_speaks() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    let judgment = DreamJudgment {
        note_id: "n1".to_string(),
        entities: vec![test_entity("e1", "job search", one_hot(0))],
        facts: vec![],
        events: vec![],
        memories: vec![],
        times: vec![],
        hierarchy: vec![],
        contains: vec![(
            NodeRef::new("n1", "Note"),
            NodeRef::new("e1", "Entity"),
            contains_edge("c1"),
        )],
        similar: vec![],
        has_property: vec![],
        leads_to: vec![],
    };
    let judge = FixedJudge::new(vec![judgment]);

    let report = run_dream(
        &conn,
        &judge,
        &spec(
            "run-1",
            vec![test_note("n1", "applied to five companies today")],
            one_hot(0),
            &outbox,
            &log,
        ),
    );

    assert_eq!(report.notes_in, 1);
    assert!(report.nodes_written >= 3, "note + entity + edge node");
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    assert!(report.message_written);
    assert!(report.shelf_ok);
    assert!(!is_nightmare(&report, 0.0), "a good dream, floor 0.0");
    assert!(report.nodes_added >= 3);

    // The artifact is real and provable.
    let artifact = outbox.join("run-1-sermon.json");
    assert!(artifact.exists());
    shelf_check(&artifact, "run-1").unwrap();

    // The ledger recorded exactly this run.
    let ledger = read_dream_reports(&log).unwrap();
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0], report);
}

#[test]
fn dangling_edge_reference_is_recorded_not_silently_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    let judgment = DreamJudgment {
        note_id: "n1".to_string(),
        entities: vec![test_entity("e1", "job search", one_hot(0))],
        facts: vec![],
        events: vec![],
        memories: vec![],
        times: vec![],
        hierarchy: vec![],
        contains: vec![
            // Valid: the note mentions the concept.
            (
                NodeRef::new("n1", "Note"),
                NodeRef::new("e1", "Entity"),
                contains_edge("c1"),
            ),
            // Dangling: the judge hallucinated an entity that was never
            // written.
            (
                NodeRef::new("e1", "Entity"),
                NodeRef::new("ghost", "Entity"),
                contains_edge("c2"),
            ),
        ],
        similar: vec![],
        has_property: vec![],
        leads_to: vec![],
    };
    let judge = FixedJudge::new(vec![judgment]);

    let report = run_dream(
        &conn,
        &judge,
        &spec(
            "run-bad",
            vec![test_note("n1", "applied to five companies today")],
            one_hot(0),
            &outbox,
            &log,
        ),
    );

    // The fault is named, not swallowed.
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("dangling reference")),
        "errors: {:?}",
        report.errors
    );
    // Any error makes the dream a nightmare, even though the valid parts
    // were still written and the reflection still ran.
    assert!(is_nightmare(&report, 0.0));
    assert!(report.nodes_written >= 3, "valid writes still landed");
    assert!(report.message_written);
    assert!(report.shelf_ok);
}

#[test]
fn quiet_day_writes_no_message() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    let judge = FixedJudge::new(vec![]);
    let report = run_dream(
        &conn,
        &judge,
        &spec("run-quiet", vec![], one_hot(0), &outbox, &log),
    );

    assert_eq!(report.notes_in, 0);
    assert_eq!(report.nodes_written, 0);
    assert!(!report.message_written, "no empty sermons");
    assert!(report.shelf_ok, "vacuously — nothing was owed");
    assert!(
        !is_nightmare(&report, 0.5),
        "a quiet day is not a nightmare"
    );
    assert!(!outbox.exists(), "no artifact should be written");

    let ledger = read_dream_reports(&log).unwrap();
    assert_eq!(ledger.len(), 1, "quiet days are still logged");
}
