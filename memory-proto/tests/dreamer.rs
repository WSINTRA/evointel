use chrono::{DateTime, Utc};
use memory_proto::MemoryDb;
use memory_proto::dreamer::{DreamJudgment, DreamSpec, FixedJudge, MechanicalSermon, run_dream};
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
        &MechanicalSermon,
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
        &MechanicalSermon,
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
        &MechanicalSermon,
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

// ─── The choice ledger ───────────────────────────────────────────────

fn test_memory(id: &str, function: &str) -> memory_proto::nodes::Memory {
    memory_proto::nodes::Memory {
        id: id.to_string(),
        predicate: "felt".to_string(),
        certainty: 1.0,
        source: "ledger-test".to_string(),
        status: "occurred".to_string(),
        is_ongoing: false,
        significance: "why it matters".to_string(),
        emotions: vec![],
        reflection: "what to make of it".to_string(),
        function_tag: Some(function.to_string()),
        run_id: None,
        label: id.to_string(),
        label_resolved: id.to_string(),
        label_embedding: None,
        learned_at: ts(T),
        expire_at: None,
        created_at: ts(T),
        updated_at: ts(T),
        layer: 4,
        context: "dreamer-test".to_string(),
    }
}

fn dream_run_row(conn: &lbug::Connection, id: &str) -> (i64, i64, i64, i64, i64) {
    let mut stmt = conn
        .prepare("MATCH (r:DreamRun {id: $id}) RETURN r.kept_t, r.kept_f, r.kept_s, r.kept_n, r.dropped_total;")
        .unwrap();
    let result = conn
        .execute(&mut stmt, vec![("id", lbug::Value::String(id.to_string()))])
        .unwrap();
    for row in result {
        if let [
            lbug::Value::Int64(t),
            lbug::Value::Int64(f),
            lbug::Value::Int64(s),
            lbug::Value::Int64(n),
            lbug::Value::Int64(d),
        ] = row.as_slice()
        {
            return (*t, *f, *s, *n, *d);
        }
    }
    panic!("no dream run row {id}")
}

fn dream_run_lists(conn: &lbug::Connection, id: &str) -> (Vec<String>, Vec<String>) {
    let mut stmt = conn
        .prepare("MATCH (r:DreamRun {id: $id}) RETURN r.considered, r.dropped;")
        .unwrap();
    let result = conn
        .execute(&mut stmt, vec![("id", lbug::Value::String(id.to_string()))])
        .unwrap();
    for row in result {
        let to_ids = |v: &lbug::Value| match v {
            lbug::Value::List(_, items) => items
                .iter()
                .filter_map(|i| match i {
                    lbug::Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        };
        return (to_ids(&row[0]), to_ids(&row[1]));
    }
    panic!("no dream run row {id}")
}

#[test]
fn the_ledger_records_what_was_kept_and_what_was_let_go() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    // Two notes: one keeps a thinking and a feeling memory; one is
    // judged honestly and keeps nothing — the discarded is data.
    let rich = DreamJudgment {
        note_id: "n1".to_string(),
        entities: vec![test_entity("e1", "job search", one_hot(0))],
        facts: vec![],
        events: vec![],
        memories: vec![test_memory("m1", "thinking"), test_memory("m2", "feeling")],
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
    let lean = DreamJudgment {
        note_id: "n2".to_string(),
        entities: vec![],
        facts: vec![],
        events: vec![],
        memories: vec![],
        times: vec![],
        hierarchy: vec![],
        contains: vec![],
        similar: vec![],
        has_property: vec![],
        leads_to: vec![],
    };
    let judge = FixedJudge::new(vec![rich, lean]);

    run_dream(
        &conn,
        &judge,
        &MechanicalSermon,
        &spec(
            "run-ledger",
            vec![
                test_note("n1", "the pipeline analysis"),
                test_note("n2", "nothing worth keeping here"),
            ],
            one_hot(0),
            &outbox,
            &log,
        ),
    );

    assert_eq!(
        dream_run_row(&conn, "run-ledger"),
        (1, 1, 0, 0, 1),
        "one thinking keep, one feeling keep, one dropped note"
    );
    let (considered, dropped) = dream_run_lists(&conn, "run-ledger");
    assert_eq!(considered, vec!["n1".to_string(), "n2".to_string()]);
    assert_eq!(dropped, vec!["n2".to_string()]);

    // And the kept memories carry the run's stamp — the provenance of
    // the choice.
    let mut stmt = conn
        .prepare("MATCH (m:Memory) WHERE m.run_id IS NOT NULL RETURN m.id, m.run_id ORDER BY m.id;")
        .unwrap();
    let result = conn.execute(&mut stmt, vec![]).unwrap();
    let mut stamped = vec![];
    for row in result {
        if let [lbug::Value::String(id), lbug::Value::String(run)] = row.as_slice() {
            stamped.push((id.clone(), run.clone()));
        }
    }
    assert_eq!(
        stamped,
        vec![
            ("m1".to_string(), "run-ledger".to_string()),
            ("m2".to_string(), "run-ledger".to_string()),
        ]
    );
}

#[test]
fn the_ledger_is_idempotent_across_re_dreams() {
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
        memories: vec![test_memory("m1", "sensing")],
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
    let spec = spec(
        "run-again",
        vec![test_note("n1", "the concrete detail log")],
        one_hot(0),
        &outbox,
        &log,
    );

    run_dream(&conn, &judge, &MechanicalSermon, &spec);
    run_dream(&conn, &judge, &MechanicalSermon, &spec);

    let result = conn.query("MATCH (r:DreamRun) RETURN count(r);").unwrap();
    for row in result {
        assert_eq!(row[0], lbug::Value::Int64(1), "one row, converged");
    }
    assert_eq!(dream_run_row(&conn, "run-again"), (0, 0, 1, 0, 0));
}

#[test]
fn a_quiet_day_writes_no_ledger_row() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    run_dream(
        &conn,
        &FixedJudge::new(vec![]),
        &MechanicalSermon,
        &spec("run-silent", vec![], one_hot(0), &outbox, &log),
    );

    let result = conn.query("MATCH (r:DreamRun) RETURN count(r);").unwrap();
    for row in result {
        assert_eq!(
            row[0],
            lbug::Value::Int64(0),
            "nothing considered, nothing owed"
        );
    }
}
