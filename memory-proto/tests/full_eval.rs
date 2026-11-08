//! The full integration eval: one dream-reflect cycle, end to end,
//! telling plan.md §8's own story — a job search shifting from anxious
//! to strategic.
//!
//! Fixed judgments (the mock dreamer) stand in for the LLM; everything
//! downstream of the judgment seam is the real system: writes,
//! validation, observation, reflection reads, the sermon artifact, the
//! shelf check, the ledger, rebuildability, and nightmare recovery.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use memory_proto::MemoryDb;
use memory_proto::dreamer::{DreamJudgment, DreamSpec, FixedJudge, TimeWindows, run_dream};
use memory_proto::edges::{Contains, HasProperty, LeadsTo, Similar};
use memory_proto::message::{Message, MessageKind, shelf_check};
use memory_proto::mutations::NodeRef;
use memory_proto::nodes::{Entity, Event, Fact, Note, Time};
use memory_proto::observability::{is_nightmare, read_dream_reports};
use memory_proto::queries::{get_note, graph_snapshot, structural_similarity};
use memory_proto::schema::EMBEDDING_DIM;

/// Window A: November 2023 — the anxious cluster lives at `one_hot(1)`.
const T_A: i64 = 1_700_000_000;
/// Window B: February 2024 — the strategic cluster lives at `one_hot(2)`.
const T_B: i64 = 1_707_776_000;

fn ts(t: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(t, 0).unwrap()
}

fn one_hot(idx: usize) -> Vec<f32> {
    let mut v = vec![0.0; EMBEDDING_DIM];
    v[idx] = 1.0;
    v
}

fn drift_windows() -> TimeWindows {
    (
        (ts(T_A - 86_400), ts(T_A + 86_400)),
        (ts(T_B - 86_400), ts(T_B + 86_400)),
    )
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

// ─── Fixture builders ───────────────────────────────────────────────

fn note(id: &str, content: &str, embedding: Vec<f32>, at: i64) -> Note {
    Note {
        id: id.to_string(),
        content: content.to_string(),
        content_embedding: Some(embedding),
        created_at: ts(at),
        updated_at: ts(at),
        layer: 0,
        context: "full-eval".to_string(),
    }
}

fn entity(id: &str, name: &str, embedding: Vec<f32>, at: i64) -> Entity {
    Entity {
        id: id.to_string(),
        label: name.to_string(),
        label_resolved: name.to_string(),
        label_embedding: Some(embedding),
        learned_at: ts(at),
        expire_at: None,
        created_at: ts(at),
        updated_at: ts(at),
        layer: 1,
        context: "full-eval".to_string(),
    }
}

fn fact(id: &str, name: &str, embedding: Vec<f32>, at: i64) -> Fact {
    Fact {
        id: id.to_string(),
        predicate: "leads_to".to_string(),
        certainty: 0.9,
        source: "obs".to_string(),
        label: name.to_string(),
        label_resolved: name.to_string(),
        label_embedding: Some(embedding),
        learned_at: ts(at),
        expire_at: None,
        created_at: ts(at),
        updated_at: ts(at),
        layer: 2,
        context: "full-eval".to_string(),
    }
}

fn event(id: &str, name: &str, embedding: Vec<f32>, at: i64) -> Event {
    Event {
        id: id.to_string(),
        predicate: "scheduled".to_string(),
        certainty: 1.0,
        source: "obs".to_string(),
        status: "occurred".to_string(),
        is_ongoing: false,
        label: name.to_string(),
        label_resolved: name.to_string(),
        label_embedding: Some(embedding),
        learned_at: ts(at),
        expire_at: None,
        created_at: ts(at),
        updated_at: ts(at),
        layer: 3,
        context: "full-eval".to_string(),
    }
}

fn time_node(id: &str, label: &str, at: i64) -> Time {
    Time {
        id: id.to_string(),
        granularity: "month".to_string(),
        starts_at: ts(at),
        ends_at: None,
        label: label.to_string(),
        label_resolved: label.to_string(),
        learned_at: ts(at),
        expire_at: None,
        created_at: ts(at),
        updated_at: ts(at),
        layer: 1,
        context: "full-eval".to_string(),
    }
}

fn contains_props(id: &str) -> Contains {
    Contains {
        id: id.to_string(),
        containment_types: "mention".to_string(),
        weight: 1.0,
        label: "contains".to_string(),
        label_resolved: "note mentions concept".to_string(),
        learned_at: ts(T_A),
        expire_at: None,
        created_at: ts(T_A),
        updated_at: ts(T_A),
        layer: 1,
        context: "full-eval".to_string(),
    }
}

fn similar_props(id: &str) -> Similar {
    Similar {
        id: id.to_string(),
        similarity: 0.8,
        sim_context: "semantic".to_string(),
        sim_method: "cosine".to_string(),
        label: "similar".to_string(),
        label_resolved: "concepts sit close".to_string(),
        learned_at: ts(T_A),
        expire_at: None,
        created_at: ts(T_A),
        updated_at: ts(T_A),
        layer: 1,
        context: "full-eval".to_string(),
    }
}

fn has_property_props(id: &str, name: &str, value: &str) -> HasProperty {
    HasProperty {
        id: id.to_string(),
        property_name: name.to_string(),
        property_value: value.to_string(),
        prop_context: "full-eval".to_string(),
        certainty: 1.0,
        label: "has property".to_string(),
        label_resolved: format!("{name} = {value}"),
        learned_at: ts(T_A),
        expire_at: None,
        created_at: ts(T_A),
        updated_at: ts(T_A),
        layer: 1,
        context: "full-eval".to_string(),
    }
}

fn leads_to_props(id: &str) -> LeadsTo {
    LeadsTo {
        id: id.to_string(),
        probability: 0.9,
        strength: 0.7,
        mechanism: "applications open doors".to_string(),
        label: "leads to".to_string(),
        label_resolved: "applications lead to interviews".to_string(),
        learned_at: ts(T_A),
        expire_at: None,
        created_at: ts(T_A),
        updated_at: ts(T_A),
        layer: 2,
        context: "full-eval".to_string(),
    }
}

fn r(id: &str, label: &str) -> NodeRef {
    NodeRef::new(id.to_string(), label.to_string())
}

/// The plan.md §8 story, as a dream batch. `p` prefixes every id — the
/// *names* (semantic keys) are identical across prefixes, which is what
/// rebuildability rides on.
///
/// The anchor concept is "job search" at `one_hot(0)`. Its window-A
/// neighborhood (anxiety note ×2, career planning, the fact) embeds at
/// `one_hot(1)`; its window-B neighborhood (the interview note) embeds
/// at `one_hot(2)` — so drift between the windows is exactly
/// orthogonal: displacement 1.0, +1 / -4 connections.
#[allow(clippy::type_complexity)]
fn job_search_batch(p: &str) -> (Vec<Note>, Vec<DreamJudgment>) {
    let queue = vec![
        note(
            &format!("{p}-n-a1"),
            "applied to five companies today, feeling anxious about it all",
            one_hot(1),
            T_A,
        ),
        note(
            &format!("{p}-n-a2"),
            "revised the CV again, still worried",
            one_hot(1),
            T_A,
        ),
        note(
            &format!("{p}-n-b1"),
            "scheduled interviews with three companies, feeling strategic",
            one_hot(2),
            T_B,
        ),
    ];

    let j_a1 = DreamJudgment {
        note_id: format!("{p}-n-a1"),
        entities: vec![
            entity(&format!("{p}-e-job"), "job search", one_hot(0), T_A),
            entity(&format!("{p}-e-anx"), "anxiety", one_hot(1), T_A),
        ],
        facts: vec![],
        events: vec![],
        memories: vec![],
        times: vec![],
        hierarchy: vec![],
        contains: vec![
            (
                r(&format!("{p}-n-a1"), "Note"),
                r(&format!("{p}-e-job"), "Entity"),
                contains_props(&format!("{p}-c-a1-job")),
            ),
            (
                r(&format!("{p}-n-a1"), "Note"),
                r(&format!("{p}-e-anx"), "Entity"),
                contains_props(&format!("{p}-c-a1-anx")),
            ),
        ],
        similar: vec![],
        has_property: vec![],
        leads_to: vec![],
    };

    let j_a2 = DreamJudgment {
        note_id: format!("{p}-n-a2"),
        entities: vec![entity(
            &format!("{p}-e-car"),
            "career planning",
            one_hot(1),
            T_A,
        )],
        facts: vec![],
        events: vec![],
        memories: vec![],
        times: vec![
            time_node(&format!("{p}-t-nov"), "November 2023", T_A),
            time_node(&format!("{p}-t-week"), "week of November 13", T_A),
        ],
        hierarchy: vec![(format!("{p}-t-nov"), format!("{p}-t-week"))],
        contains: vec![(
            r(&format!("{p}-n-a2"), "Note"),
            r(&format!("{p}-e-job"), "Entity"),
            contains_props(&format!("{p}-c-a2-job")),
        )],
        similar: vec![(
            r(&format!("{p}-e-job"), "Entity"),
            r(&format!("{p}-e-car"), "Entity"),
            similar_props(&format!("{p}-s-job-car")),
        )],
        has_property: vec![],
        leads_to: vec![],
    };

    let j_b1 = DreamJudgment {
        note_id: format!("{p}-n-b1"),
        entities: vec![entity(&format!("{p}-e-str"), "strategy", one_hot(2), T_B)],
        facts: vec![fact(
            &format!("{p}-f-1"),
            "applications lead to interviews",
            one_hot(1),
            T_A,
        )],
        events: vec![event(
            &format!("{p}-ev-1"),
            "interviews scheduled",
            one_hot(2),
            T_B,
        )],
        memories: vec![],
        times: vec![time_node(&format!("{p}-t-feb"), "February 2024", T_B)],
        hierarchy: vec![],
        contains: vec![
            (
                r(&format!("{p}-n-b1"), "Note"),
                r(&format!("{p}-e-job"), "Entity"),
                contains_props(&format!("{p}-c-b1-job")),
            ),
            (
                r(&format!("{p}-n-b1"), "Note"),
                r(&format!("{p}-e-str"), "Entity"),
                contains_props(&format!("{p}-c-b1-str")),
            ),
        ],
        similar: vec![],
        has_property: vec![(
            r(&format!("{p}-f-1"), "Fact"),
            r(&format!("{p}-e-job"), "Entity"),
            has_property_props(&format!("{p}-hp-1"), "stage", "interviewing"),
        )],
        leads_to: vec![(
            r(&format!("{p}-f-1"), "Fact"),
            r(&format!("{p}-ev-1"), "Event"),
            leads_to_props(&format!("{p}-lt-1")),
        )],
    };

    (queue, vec![j_a1, j_a2, j_b1])
}

fn spec(
    run_id: &str,
    queue: Vec<Note>,
    outbox: &std::path::Path,
    log: &std::path::Path,
) -> DreamSpec {
    DreamSpec {
        run_id: run_id.to_string(),
        queue,
        anchor_embedding: one_hot(0),
        drift_windows: Some(drift_windows()),
        outbox: outbox.to_path_buf(),
        dream_log: log.to_path_buf(),
    }
}

// ─── The eval ───────────────────────────────────────────────────────

#[test]
fn full_cycle_delivers_the_job_search_story() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("eval.db"));
    let conn = db.connect().unwrap();
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    let (queue, judgments) = job_search_batch("a");
    let judge = FixedJudge::new(judgments);
    let report = run_dream(&conn, &judge, &spec("run-1", queue, &outbox, &log));

    // ── The dream itself: every count is exact and deterministic ──
    assert_eq!(report.notes_in, 3);
    // 3 notes + 4 entities + 1 fact + 1 event + 3 times = 12 node
    // writes, + 8 edge nodes + 1 hierarchy wiring = 21 write ops.
    assert_eq!(report.nodes_written, 21, "write ops");
    // Observed new *semantic* nodes: 12 content + 4 edge-node entries —
    // same-named edge nodes collapse to one key (all Contains edges
    // share the label "note mentions concept"), which is exactly what
    // semantic keys are supposed to do.
    assert_eq!(report.nodes_added, 16);
    assert_eq!(report.edges_added, 9, "8 triples + 1 hierarchy rel");
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    assert!(report.message_written);
    assert!(report.shelf_ok);

    // A young graph: the seed (6 time/abstract-time nodes + 3 rels) is
    // most of what exists before the dream, so one honest dream takes
    // similarity to ~9/34 ≈ 0.26. This is exactly why the nightmare
    // floor is a caller policy, not a constant.
    assert!(
        report
            .similarity_to_prev
            .is_some_and(|s| (0.2..0.35).contains(&s)),
        "similarity: {:?}",
        report.similarity_to_prev
    );
    assert!(!is_nightmare(&report, 0.1));

    // ── The sermon is grounded ──
    let artifact = outbox.join("run-1-sermon.json");
    shelf_check(&artifact, "run-1").unwrap();
    let msg: Message = serde_json::from_str(&std::fs::read_to_string(&artifact).unwrap()).unwrap();
    assert_eq!(msg.kind, MessageKind::Sermon);

    // Every citation resolves to a real note, and the snippet is that
    // note's actual content — evidence, not vibes.
    let cited: BTreeSet<String> = msg.citations.iter().map(|c| c.note_id.clone()).collect();
    assert_eq!(
        cited,
        ["a-n-a1", "a-n-a2", "a-n-b1"]
            .iter()
            .map(ToString::to_string)
            .collect::<BTreeSet<_>>()
    );
    for c in &msg.citations {
        let n = get_note(&conn, &c.note_id)
            .unwrap()
            .unwrap_or_else(|| panic!("citation {} must resolve to a real note", c.note_id));
        assert_eq!(n.content, c.snippet);
    }

    // The headline insight: anxious → strategic, with drift evidence.
    let drift_insights: Vec<_> = msg.insights.iter().filter(|i| i.drift.is_some()).collect();
    assert_eq!(drift_insights.len(), 1);
    let d = drift_insights[0].drift.unwrap();
    assert!(
        (d.displacement - 1.0).abs() < 1e-6,
        "displacement {}",
        d.displacement
    );
    assert_eq!(d.new_connections, 1);
    assert_eq!(d.dropped_connections, 4);
    assert!(drift_insights[0].summary.contains("job search"));

    // Plus the baseline and events insights.
    assert!(
        msg.insights
            .iter()
            .any(|i| i.summary.contains("3 notes now connect"))
    );
    assert!(
        msg.insights
            .iter()
            .any(|i| i.summary.contains("events recorded"))
    );
    assert!(msg.body.contains("Grounded in your notes"));

    // ── The ledger recorded exactly this run ──
    let ledger = read_dream_reports(&log).unwrap();
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0], report);
}

#[test]
fn the_dream_is_rebuildable() {
    // Same story, different ids: two independent DBs, two dream runs.
    let dir_a = tempfile::tempdir().unwrap();
    let db_a = open_test_db(&dir_a.path().join("a.db"));
    let conn_a = db_a.connect().unwrap();

    let dir_b = tempfile::tempdir().unwrap();
    let db_b = open_test_db(&dir_b.path().join("b.db"));
    let conn_b = db_b.connect().unwrap();

    let (queue_a, judgments_a) = job_search_batch("a");
    let (queue_b, judgments_b) = job_search_batch("b");

    let report_a = run_dream(
        &conn_a,
        &FixedJudge::new(judgments_a),
        &spec(
            "run-a",
            queue_a,
            &dir_a.path().join("outbox"),
            &dir_a.path().join("dreams.jsonl"),
        ),
    );
    let report_b = run_dream(
        &conn_b,
        &FixedJudge::new(judgments_b),
        &spec(
            "run-b",
            queue_b,
            &dir_b.path().join("outbox"),
            &dir_b.path().join("dreams.jsonl"),
        ),
    );
    assert!(report_a.errors.is_empty(), "{:?}", report_a.errors);
    assert!(report_b.errors.is_empty(), "{:?}", report_b.errors);

    // §10: the graph is derived, so the same judgments rebuild it
    // exactly — different UUIDs, identical structure.
    let snap_a = graph_snapshot(&conn_a).unwrap();
    let snap_b = graph_snapshot(&conn_b).unwrap();
    assert!(!snap_a.is_empty());
    let score = structural_similarity(&snap_a, &snap_b);
    assert!(
        (score - 1.0).abs() < 1e-6,
        "rebuild must be structurally identical, got {score}"
    );

    // And the observations agree.
    assert_eq!(report_a.nodes_added, report_b.nodes_added);
    assert_eq!(report_a.nodes_written, report_b.nodes_written);
}

#[test]
fn nightmare_is_detected_then_healed_by_rerun() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("eval.db"));
    let conn = db.connect().unwrap();
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("logs").join("dreams.jsonl");

    // Corrupt the judgment: the judge hallucinates an entity that is
    // never written, and references it in an edge.
    let (queue, mut judgments) = job_search_batch("a");
    judgments[2].contains[1].1 = r("a-ghost", "Entity");
    let report1 = run_dream(
        &conn,
        &FixedJudge::new(judgments),
        &spec("run-1", queue, &outbox, &log),
    );

    // The fault is named and the dream is a nightmare — even though the
    // valid parts landed and the sermon is still grounded.
    assert!(
        report1
            .errors
            .iter()
            .any(|e| e.contains("dangling reference: Entity/a-ghost")),
        "errors: {:?}",
        report1.errors
    );
    assert!(is_nightmare(&report1, 0.1));
    assert!(report1.message_written);
    assert!(report1.shelf_ok);

    // ── Recovery: re-run the dream with corrected judgments ──
    let (queue2, judgments2) = job_search_batch("a");
    let report2 = run_dream(
        &conn,
        &FixedJudge::new(judgments2),
        &spec("run-2", queue2, &outbox, &log),
    );

    // The idempotent replay re-writes everything (nodes_written back to
    // full) but observes only the repair: one new *triple* — and zero
    // new semantic nodes, because the repaired Contains edge shares its
    // name with the four already present. And that is NOT a nightmare.
    assert!(report2.errors.is_empty(), "errors: {:?}", report2.errors);
    assert_eq!(report2.nodes_written, 21, "the replay re-wrote everything");
    assert_eq!(
        report2.nodes_added, 0,
        "no new semantic nodes — shared edge name"
    );
    assert_eq!(report2.edges_added, 1, "only the repaired triple is new");
    assert!(report2.message_written);
    assert!(report2.shelf_ok);
    assert!(
        !is_nightmare(&report2, 0.5),
        "an honest re-run is a good dream"
    );

    // The ledger tells the whole story: nightmare, then healing.
    let ledger = read_dream_reports(&log).unwrap();
    assert_eq!(ledger.len(), 2);
    assert_eq!(ledger[0].run_id, "run-1");
    assert!(!ledger[0].errors.is_empty(), "the nightmare is on record");
    assert_eq!(ledger[1].run_id, "run-2");
    assert!(ledger[1].errors.is_empty(), "the healed run must be clean");

    // Both artifacts exist — the second sermon supersedes the first in
    // the same outbox.
    assert!(outbox.join("run-1-sermon.json").exists());
    assert!(outbox.join("run-2-sermon.json").exists());
}
