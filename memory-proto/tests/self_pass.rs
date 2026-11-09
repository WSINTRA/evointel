//! The self pass — working the self inside the dream.
//!
//! Fixtures build a synthetic but honest axes world: anchors seeded on
//! orthogonal poles (thinking = basis 0, feeling = basis 1, sensing =
//! basis 2, intuiting = basis 3), notes embedded near a pole, judgments
//! carrying function-tagged memories. The assertions read the graph
//! back — states, trajectories, grounds — and the report.

use chrono::{DateTime, TimeZone, Utc};
use lbug::Value;
use memory_proto::MemoryDb;
use memory_proto::dreamer::{DreamJudgment, DreamSpec, FixedJudge, MechanicalSermon, NoDelivery, run_dream};
use memory_proto::edges::Contains;
use memory_proto::mutations::{NodeRef, seed_function_anchors};
use memory_proto::nodes::{Entity, FunctionAnchor, Memory, Note};
use memory_proto::schema::EMBEDDING_DIM;

fn ts(y: i32, m: u32, d: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 9, 0, 0).single().unwrap()
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

/// Seeds the orthogonal axes: two exemplars per pole, each on its basis
/// vector. The corpus mean subtracts cleanly; the poles face away.
fn seed_axes(conn: &lbug::Connection) {
    let now = ts(2026, 11, 1);
    let anchors: Vec<FunctionAnchor> = ["thinking", "feeling", "sensing", "intuiting"]
        .iter()
        .enumerate()
        .flat_map(|(idx, function)| {
            let axis = if idx < 2 { "judgment" } else { "perception" };
            (1..=2).map(move |k| FunctionAnchor {
                id: format!("fa-{function}-{k}"),
                function: function.to_string(),
                axis: axis.to_string(),
                exemplar: format!("exemplar {k} of {function}"),
                embedding: Some(one_hot(idx)),
                active: true,
                created_at: now,
                updated_at: now,
                layer: 0,
                context: "self-pass-test".to_string(),
            })
        })
        .collect();
    seed_function_anchors(conn, &anchors).unwrap();
}

fn note_at(id: &str, content: &str, embedding: Vec<f32>, day: u32) -> Note {
    note_dated(id, content, embedding, ts(2026, 11, day))
}

fn note_dated(id: &str, content: &str, embedding: Vec<f32>, at: DateTime<Utc>) -> Note {
    Note {
        id: id.to_string(),
        content: content.to_string(),
        content_embedding: Some(embedding),
        created_at: at,
        updated_at: at,
        layer: 0,
        context: "self-pass-test".to_string(),
    }
}

fn tagged_memory(id: &str, function: &str) -> Memory {
    Memory {
        id: id.to_string(),
        predicate: "felt".to_string(),
        certainty: 1.0,
        source: "self-pass-test".to_string(),
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
        learned_at: ts(2026, 11, 30),
        expire_at: None,
        created_at: ts(2026, 11, 30),
        updated_at: ts(2026, 11, 30),
        layer: 4,
        context: "self-pass-test".to_string(),
    }
}

fn judgment_for(note_id: &str, memories: Vec<Memory>) -> DreamJudgment {
    DreamJudgment {
        note_id: note_id.to_string(),
        entities: vec![Entity {
            id: "e-anchor".to_string(),
            label: "job search".to_string(),
            label_resolved: "job search".to_string(),
            label_embedding: Some(one_hot(0)),
            learned_at: ts(2026, 11, 30),
            expire_at: None,
            created_at: ts(2026, 11, 30),
            updated_at: ts(2026, 11, 30),
            layer: 1,
            context: "self-pass-test".to_string(),
        }],
        facts: vec![],
        events: vec![],
        memories,
        times: vec![],
        hierarchy: vec![],
        contains: vec![(
            NodeRef::new(note_id.to_string(), "Note"),
            NodeRef::new("e-anchor".to_string(), "Entity"),
            Contains {
                id: format!("c-{note_id}"),
                containment_types: "mention".to_string(),
                weight: 1.0,
                label: "contains".to_string(),
                label_resolved: "note mentions concept".to_string(),
                learned_at: ts(2026, 11, 30),
                expire_at: None,
                created_at: ts(2026, 11, 30),
                updated_at: ts(2026, 11, 30),
                layer: 1,
                context: "self-pass-test".to_string(),
            },
        )],
        similar: vec![],
        has_property: vec![],
        leads_to: vec![],
    }
}

fn spec_with_windows(
    run_id: &str,
    queue: Vec<Note>,
    outbox: &std::path::Path,
    log: &std::path::Path,
    from: (DateTime<Utc>, DateTime<Utc>),
    to: (DateTime<Utc>, DateTime<Utc>),
) -> DreamSpec {
    DreamSpec {
        run_id: run_id.to_string(),
        queue,
        anchor_embedding: one_hot(0),
        drift_windows: Some((from, to)),
        outbox: outbox.to_path_buf(),
        dream_log: log.to_path_buf(),
    }
}

fn count(conn: &lbug::Connection, cypher: &str) -> i64 {
    let result = conn.query(cypher).unwrap();
    for row in result {
        if let Value::Int64(n) = row[0] {
            return n;
        }
    }
    panic!("no count row for: {cypher}")
}

fn wunicorm_field(conn: &lbug::Connection, field: &str) -> f64 {
    let result = conn
        .query(&format!(
            "MATCH (s:SelfState {{subject: 'wunicorm'}}) RETURN s.{field};"
        ))
        .unwrap();
    for row in result {
        if let Value::Double(x) = row[0] {
            return x;
        }
    }
    panic!("no wunicorm state")
}

fn nov_window() -> (DateTime<Utc>, DateTime<Utc>) {
    (ts(2026, 11, 1), ts(2026, 11, 30))
}

fn feb_window() -> (DateTime<Utc>, DateTime<Utc>) {
    (ts(2027, 2, 1), ts(2027, 2, 28))
}

#[test]
fn self_pass_writes_both_states_with_provenance() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    seed_axes(&conn);
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    // November's notes sit feeling-ward (basis 1); the judgment keeps
    // feeling-tagged memories.
    let queue = vec![
        note_at("n1", "anxious about the search", one_hot(1), 3),
        note_at("n2", "still worried about it", one_hot(1), 5),
    ];
    let judge = FixedJudge::new(vec![
        judgment_for("n1", vec![tagged_memory("m1", "feeling")]),
        judgment_for("n2", vec![tagged_memory("m2", "feeling")]),
    ]);

    let report = run_dream(
        &conn,
        &judge,
        &MechanicalSermon,
        &NoDelivery,
        &spec_with_windows("run-1", queue, &outbox, &log, nov_window(), nov_window()),
    );

    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    let self_pass = report.self_pass.expect("the self was worked");

    // U: feeling-ward is negative judgment — clearly, not marginally.
    let user = self_pass.user_position.expect("measured window");
    assert!(user.0 < -0.5, "feeling-ward judgment, got {}", user.0);
    assert!(self_pass.drift.is_none(), "first state — no drift yet");

    // Both states exist, once each.
    assert_eq!(
        count(
            &conn,
            "MATCH (s:SelfState {subject: 'user'}) RETURN count(s);"
        ),
        1
    );
    assert_eq!(
        count(
            &conn,
            "MATCH (s:SelfState {subject: 'wunicorm'}) RETURN count(s);"
        ),
        1
    );

    // First window: no trajectory yet, and no compensation — one state
    // is not a stuck one.
    assert_eq!(
        count(&conn, "MATCH ()-[r:SELF_TRAJECTORY]->() RETURN count(r);"),
        0
    );
    assert_eq!(wunicorm_field(&conn, "judgment_comp"), 0.0);

    // Provenance: the user state grounds in its two window notes; the
    // wunicorm state grounds in the memories its run chose.
    assert_eq!(
        count(
            &conn,
            "MATCH (:SelfState {subject: 'user'})-[:FROM_Grounds]->(g:Grounds {evidence_kind: 'window'})-[:TO_Grounds]->(:Note) RETURN count(g);"
        ),
        2
    );
    assert_eq!(
        count(
            &conn,
            "MATCH (:SelfState {subject: 'wunicorm'})-[:FROM_Grounds]->(g:Grounds {evidence_kind: 'choice'})-[:TO_Grounds]->(:Memory) RETURN count(g);"
        ),
        2
    );

    // The notes carry their measured coordinates.
    let result = conn
        .query("MATCH (n:Note {id: 'n1'}) RETURN n.judgment_coord;")
        .unwrap();
    for row in result {
        let Value::Double(j) = row[0] else {
            panic!("measured")
        };
        assert!(j < -0.5, "note coordinate written, got {j}");
    }
}

#[test]
fn the_second_dream_wires_the_trajectory_and_reads_drift() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    seed_axes(&conn);
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    // November: feeling-ward. February: thinking-ward (inside the
    // February window — note ids match the judgment's contains edge).
    let nov = vec![note_at("n1", "anxious november", one_hot(1), 3)];
    let feb = vec![note_dated(
        "n2f",
        "2027-02-10: strategic february",
        one_hot(0),
        ts(2027, 2, 10),
    )];
    let nov_judge = FixedJudge::new(vec![judgment_for(
        "n1",
        vec![tagged_memory("m1", "feeling")],
    )]);
    let feb_judge = FixedJudge::new(vec![judgment_for(
        "n2f",
        vec![tagged_memory("m2", "thinking")],
    )]);

    run_dream(
        &conn,
        &nov_judge,
        &MechanicalSermon,
        &NoDelivery,
        &spec_with_windows("run-nov", nov, &outbox, &log, nov_window(), nov_window()),
    );
    let report = run_dream(
        &conn,
        &feb_judge,
        &MechanicalSermon,
        &NoDelivery,
        &spec_with_windows("run-feb", feb, &outbox, &log, nov_window(), feb_window()),
    );

    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    let self_pass = report.self_pass.expect("the self was worked");

    // The user moved from feeling-ward to thinking-ward: positive drift.
    let drift = self_pass.drift.expect("two states — drift is readable");
    assert!(drift.0 > 0.5, "moved toward thinking, got {}", drift.0);

    // The trajectory wires both subjects' consecutive states.
    assert_eq!(
        count(
            &conn,
            "MATCH (:SelfState {subject: 'user'})-[r:SELF_TRAJECTORY]->(:SelfState {subject: 'user'}) RETURN count(r);"
        ),
        1
    );
    assert_eq!(
        count(
            &conn,
            "MATCH (:SelfState {subject: 'wunicorm'})-[r:SELF_TRAJECTORY]->(:SelfState {subject: 'wunicorm'}) RETURN count(r);"
        ),
        1
    );
    // Two user states, two wunicorm states.
    assert_eq!(
        count(
            &conn,
            "MATCH (s:SelfState {subject: 'user'}) RETURN count(s);"
        ),
        2
    );
}

#[test]
fn compensation_leans_only_against_a_rigid_one_sided_user() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    seed_axes(&conn);
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    // A rigid user: three prior states in a straight feeling-ward line.
    let straight = [(0.0, 0.0), (-0.3, 0.0), (-0.6, 0.0)];
    for (i, position) in straight.iter().enumerate() {
        memory_proto::mutations::insert_self_state(
            &conn,
            &memory_proto::nodes::SelfState {
                id: format!("ss-prior-{i}"),
                subject: "user".to_string(),
                window_start: ts(2026, 8, 1 + u32::try_from(i * 7).unwrap()),
                window_end: ts(2026, 8, 7 + u32::try_from(i * 7).unwrap()),
                judgment_coord: position.0,
                perception_coord: position.1,
                judgment_core: 0.0,
                perception_core: 0.0,
                judgment_comp: 0.0,
                perception_comp: 0.0,
                rigidity_judgment: 0.0,
                rigidity_perception: 0.0,
                evidence_kept: 1,
                evidence_dropped: 0,
                learned_at: ts(2026, 8, 28),
                created_at: ts(2026, 8, 28),
                updated_at: ts(2026, 8, 28),
                layer: 4,
                context: "rigid-fixture".to_string(),
            },
        )
        .unwrap();
    }

    // The dream's window continues the line: still feeling-ward.
    let queue = vec![note_at("n1", "rigidly anxious", one_hot(1), 3)];
    let judge = FixedJudge::new(vec![judgment_for(
        "n1",
        vec![tagged_memory("m1", "feeling")],
    )]);
    let report = run_dream(
        &conn,
        &judge,
        &MechanicalSermon,
        &NoDelivery,
        &spec_with_windows(
            "run-rigid",
            queue,
            &outbox,
            &log,
            nov_window(),
            nov_window(),
        ),
    );

    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    let self_pass = report.self_pass.expect("the self was worked");
    assert!(
        self_pass.rigidity.0 > 0.9,
        "straight line, got {}",
        self_pass.rigidity.0
    );
    // The fast component leans against the one-sided user: thinking-ward
    // (positive) against a feeling-ward (negative) stance.
    assert!(
        self_pass.wunicorm_comp.0 > 0.0,
        "compensation leans against rigidity, got {}",
        self_pass.wunicorm_comp.0
    );
    assert!(wunicorm_field(&conn, "judgment_comp") > 0.0);
}

#[test]
fn a_wandering_user_gets_no_compensation() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    seed_axes(&conn);
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    // A wandering user: zigzag on judgment — one-sided this window is
    // not stuck, and medicine scales to stuckness, never direction.
    let zigzag = [(0.3, 0.0), (-0.3, 0.0), (0.3, 0.0)];
    for (i, position) in zigzag.iter().enumerate() {
        memory_proto::mutations::insert_self_state(
            &conn,
            &memory_proto::nodes::SelfState {
                id: format!("ss-zig-{i}"),
                subject: "user".to_string(),
                window_start: ts(2026, 8, 1 + u32::try_from(i * 7).unwrap()),
                window_end: ts(2026, 8, 7 + u32::try_from(i * 7).unwrap()),
                judgment_coord: position.0,
                perception_coord: position.1,
                judgment_core: 0.0,
                perception_core: 0.0,
                judgment_comp: 0.0,
                perception_comp: 0.0,
                rigidity_judgment: 0.0,
                rigidity_perception: 0.0,
                evidence_kept: 1,
                evidence_dropped: 0,
                learned_at: ts(2026, 8, 28),
                created_at: ts(2026, 8, 28),
                updated_at: ts(2026, 8, 28),
                layer: 4,
                context: "wandering-fixture".to_string(),
            },
        )
        .unwrap();
    }

    let queue = vec![note_at("n1", "anxious again this month", one_hot(1), 3)];
    let judge = FixedJudge::new(vec![judgment_for(
        "n1",
        vec![tagged_memory("m1", "feeling")],
    )]);
    let report = run_dream(
        &conn,
        &judge,
        &MechanicalSermon,
        &NoDelivery,
        &spec_with_windows(
            "run-wander",
            queue,
            &outbox,
            &log,
            nov_window(),
            nov_window(),
        ),
    );

    let self_pass = report.self_pass.expect("the self was worked");
    assert!(
        self_pass.wunicorm_comp.0.abs() < 1e-6,
        "no compensation against a wanderer, got {}",
        self_pass.wunicorm_comp.0
    );
}

#[test]
fn unseeded_anchors_leave_the_dream_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    // No anchors seeded: the self layer is opt-in, and a pre-self DB
    // dreams exactly as before — no fault, no states, message written.
    let queue = vec![note_at("n1", "a plain old dream", one_hot(1), 3)];
    let judge = FixedJudge::new(vec![judgment_for(
        "n1",
        vec![tagged_memory("m1", "feeling")],
    )]);
    let report = run_dream(
        &conn,
        &judge,
        &MechanicalSermon,
        &NoDelivery,
        &spec_with_windows(
            "run-plain",
            queue,
            &outbox,
            &log,
            nov_window(),
            nov_window(),
        ),
    );

    assert!(report.self_pass.is_none());
    assert!(
        report.errors.is_empty(),
        "unseeded is quiet, not a fault: {:?}",
        report.errors
    );
    assert!(report.message_written, "the dream itself is whole");
    assert_eq!(
        count(&conn, "MATCH (s:SelfState) RETURN count(s);"),
        0,
        "the self stays unworked"
    );
}

#[test]
fn a_quiet_day_does_not_work_the_self() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("dream.db"));
    let conn = db.connect().unwrap();
    seed_axes(&conn);
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    let report = run_dream(
        &conn,
        &FixedJudge::new(vec![]),
        &MechanicalSermon,
        &NoDelivery,
        &spec_with_windows(
            "run-silent",
            vec![],
            &outbox,
            &log,
            nov_window(),
            nov_window(),
        ),
    );

    assert!(report.self_pass.is_none());
    assert_eq!(report.errors, Vec::<String>::new());
    assert_eq!(count(&conn, "MATCH (s:SelfState) RETURN count(s);"), 0);
}
