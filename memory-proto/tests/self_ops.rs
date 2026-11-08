//! Self-layer write tests: anchors, self states, dream runs — roundtrip,
//! idempotency, and the re-seed contract (old rows deactivate, never
//! disappear).

use chrono::{TimeZone, Utc};
use lbug::Value;
use memory_proto::db::MemoryDb;
use memory_proto::edges::Grounds;
use memory_proto::mutations::{
    NodeRef, insert_dream_run, insert_self_state, link_grounds, seed_function_anchors,
    set_note_coordinates, wire_self_trajectory,
};
use memory_proto::nodes::{DreamRun, FunctionAnchor, Note, SelfState};
use memory_proto::queries::graph_snapshot;
use memory_proto::schema::{self, EMBEDDING_DIM};

fn temp_db() -> (tempfile::TempDir, MemoryDb) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let db = MemoryDb::open(&path, EMBEDDING_DIM).unwrap();
    {
        let conn = db.connect().unwrap();
        schema::init(&conn).unwrap();
    }
    (dir, db)
}

fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 9, 0, 0).single().unwrap()
}

fn anchor(id: &str, function: &str, axis: &str, exemplar: &str) -> FunctionAnchor {
    FunctionAnchor {
        id: id.to_string(),
        function: function.to_string(),
        axis: axis.to_string(),
        exemplar: exemplar.to_string(),
        embedding: Some(vec![0.1; EMBEDDING_DIM]),
        active: true,
        created_at: ts(2026, 11, 1),
        updated_at: ts(2026, 11, 1),
        layer: 0,
        context: "test".to_string(),
    }
}

fn self_state(id: &str, subject: &str) -> SelfState {
    SelfState {
        id: id.to_string(),
        subject: subject.to_string(),
        window_start: ts(2026, 11, 1),
        window_end: ts(2026, 11, 30),
        judgment_coord: 0.42,
        perception_coord: -0.18,
        judgment_core: 0.1,
        perception_core: -0.05,
        judgment_comp: -0.08,
        perception_comp: 0.03,
        rigidity_judgment: 0.9,
        rigidity_perception: 0.4,
        evidence_kept: 7,
        evidence_dropped: 2,
        learned_at: ts(2026, 11, 30),
        created_at: ts(2026, 11, 30),
        updated_at: ts(2026, 11, 30),
        layer: 4,
        context: "test".to_string(),
    }
}

fn dream_run(id: &str) -> DreamRun {
    DreamRun {
        id: id.to_string(),
        considered: vec!["n-1".to_string(), "n-2".to_string()],
        dropped: vec!["n-2".to_string()],
        kept_t: 2,
        kept_f: 1,
        kept_s: 0,
        kept_n: 3,
        dropped_total: 1,
        started_at: ts(2026, 11, 30),
        finished_at: ts(2026, 11, 30),
        created_at: ts(2026, 11, 30),
        updated_at: ts(2026, 11, 30),
        layer: 3,
        context: "test".to_string(),
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

#[test]
fn self_state_roundtrip_and_idempotent() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    insert_self_state(&conn, &self_state("ss-1", "user")).unwrap();
    insert_self_state(&conn, &self_state("ss-1", "user")).unwrap();

    assert_eq!(
        count(&conn, "MATCH (s:SelfState) RETURN count(s);"),
        1,
        "MERGE-on-id: re-insert converges"
    );
    let result = conn
        .query("MATCH (s:SelfState {id: 'ss-1'}) RETURN s.subject, s.judgment_coord, s.perception_coord, s.judgment_core, s.judgment_comp, s.rigidity_judgment, s.evidence_kept, s.evidence_dropped;")
        .unwrap();
    for row in result {
        assert_eq!(row[0], Value::String("user".to_string()));
        assert_eq!(row[1], Value::Double(0.42));
        assert_eq!(row[2], Value::Double(-0.18));
        assert_eq!(row[3], Value::Double(0.1));
        assert_eq!(row[4], Value::Double(-0.08));
        assert_eq!(row[5], Value::Double(0.9));
        assert_eq!(row[6], Value::Int64(7));
        assert_eq!(row[7], Value::Int64(2));
    }
}

#[test]
fn dream_run_roundtrip_and_idempotent() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    insert_dream_run(&conn, &dream_run("run-1")).unwrap();
    insert_dream_run(&conn, &dream_run("run-1")).unwrap();

    assert_eq!(count(&conn, "MATCH (r:DreamRun) RETURN count(r);"), 1);
    let result = conn
        .query("MATCH (r:DreamRun {id: 'run-1'}) RETURN r.kept_t, r.kept_f, r.kept_s, r.kept_n, r.dropped_total;")
        .unwrap();
    for row in result {
        assert_eq!(row[0], Value::Int64(2));
        assert_eq!(row[1], Value::Int64(1));
        assert_eq!(row[2], Value::Int64(0));
        assert_eq!(row[3], Value::Int64(3));
        assert_eq!(row[4], Value::Int64(1));
    }
}

#[test]
fn seed_deactivates_old_anchors_and_keeps_history() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    let v1_corpus = [
        anchor("fa-t-1", "thinking", "judgment", "old thinking exemplar"),
        anchor("fa-f-1", "feeling", "judgment", "old feeling exemplar"),
        anchor("fa-s-1", "sensing", "perception", "old sensing exemplar"),
        anchor(
            "fa-n-1",
            "intuiting",
            "perception",
            "old intuiting exemplar",
        ),
    ];
    seed_function_anchors(&conn, &v1_corpus).unwrap();
    assert_eq!(
        count(
            &conn,
            "MATCH (a:FunctionAnchor) WHERE a.active = true RETURN count(a);"
        ),
        4
    );

    // Re-seed: a new corpus version, different ids.
    let v2_corpus = [
        anchor("fa-t-2", "thinking", "judgment", "new thinking exemplar"),
        anchor("fa-f-2", "feeling", "judgment", "new feeling exemplar"),
    ];
    seed_function_anchors(&conn, &v2_corpus).unwrap();

    // The new rows are active; the old rows remain in the graph,
    // deactivated — the auditor is audited.
    assert_eq!(
        count(
            &conn,
            "MATCH (a:FunctionAnchor) WHERE a.active = true RETURN count(a);"
        ),
        2
    );
    assert_eq!(
        count(
            &conn,
            "MATCH (a:FunctionAnchor) WHERE a.active = false RETURN count(a);"
        ),
        4
    );
    assert_eq!(
        count(&conn, "MATCH (a:FunctionAnchor) RETURN count(a);"),
        6,
        "anchor history is preserved, never deleted"
    );
}

#[test]
fn note_coordinates_are_set_without_touching_the_rest() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    let note = Note {
        id: "n-coord".to_string(),
        content: "a note awaiting measurement".to_string(),
        content_embedding: None,
        created_at: ts(2026, 11, 3),
        updated_at: ts(2026, 11, 3),
        layer: 0,
        context: "test".to_string(),
    };
    memory_proto::mutations::insert_note(&conn, &note).unwrap();

    // Before measurement: NULL — no signal, not a fabricated one.
    let result = conn
        .query("MATCH (n:Note {id: 'n-coord'}) RETURN n.judgment_coord;")
        .unwrap();
    for row in result {
        assert!(matches!(row[0], Value::Null(_)));
    }

    set_note_coordinates(&conn, "n-coord", 0.33, -0.66).unwrap();
    let result = conn
        .query("MATCH (n:Note {id: 'n-coord'}) RETURN n.judgment_coord, n.perception_coord, n.content;")
        .unwrap();
    for row in result {
        assert_eq!(row[0], Value::Double(0.33));
        assert_eq!(row[1], Value::Double(-0.66));
        assert_eq!(
            row[2],
            Value::String("a note awaiting measurement".to_string()),
            "the coordinate write touches only the coordinate columns"
        );
    }

    // Re-inserting the note does not wipe its measured position — the
    // self pass owns the columns, insert_note never sets them.
    memory_proto::mutations::insert_note(&conn, &note).unwrap();
    let result = conn
        .query("MATCH (n:Note {id: 'n-coord'}) RETURN n.judgment_coord;")
        .unwrap();
    for row in result {
        assert_eq!(row[0], Value::Double(0.33));
    }
}

#[test]
#[should_panic(expected = "unknown function")]
fn seed_rejects_unknown_function_names() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();
    seed_function_anchors(
        &conn,
        &[anchor("fa-bad", "valorating", "judgment", "not a function")],
    )
    .unwrap();
}

// ─── Grounds + trajectory ────────────────────────────────────────────

fn grounds(id: &str, kind: &str) -> Grounds {
    Grounds {
        id: id.to_string(),
        evidence_kind: kind.to_string(),
        label: format!("grounds {id}"),
        label_resolved: format!("grounds-{id}"),
        learned_at: ts(2026, 11, 30),
        expire_at: None,
        created_at: ts(2026, 11, 30),
        updated_at: ts(2026, 11, 30),
        layer: 0,
        context: "test".to_string(),
    }
}

fn insert_note_with_content(conn: &lbug::Connection, id: &str, content: &str) {
    memory_proto::mutations::insert_note(
        conn,
        &Note {
            id: id.to_string(),
            content: content.to_string(),
            content_embedding: None,
            created_at: ts(2026, 11, 3),
            updated_at: ts(2026, 11, 3),
            layer: 0,
            context: "test".to_string(),
        },
    )
    .unwrap();
}

#[test]
fn grounds_links_self_state_to_note_evidence() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    insert_self_state(&conn, &self_state("ss-user", "user")).unwrap();
    insert_note_with_content(&conn, "n-1", "the note that filled the window");

    let from = NodeRef::new("ss-user", "SelfState");
    let to = NodeRef::new("n-1", "Note");
    link_grounds(&conn, &from, &to, &grounds("g-1", "window")).unwrap();
    link_grounds(&conn, &from, &to, &grounds("g-1", "window")).unwrap();

    // Edge node + both rels, no duplication on re-link.
    assert_eq!(count(&conn, "MATCH (g:Grounds) RETURN count(g);"), 1);
    assert_eq!(
        count(
            &conn,
            "MATCH (:SelfState {id: 'ss-user'})-[r:FROM_Grounds]->(:Grounds {id: 'g-1'}) RETURN count(r);"
        ),
        1
    );
    assert_eq!(
        count(
            &conn,
            "MATCH (:Grounds {id: 'g-1'})-[r:TO_Grounds]->(:Note {id: 'n-1'}) RETURN count(r);"
        ),
        1
    );
    // The evidence kind roundtrips.
    let result = conn
        .query("MATCH (g:Grounds {id: 'g-1'}) RETURN g.evidence_kind;")
        .unwrap();
    for row in result {
        assert_eq!(row[0], Value::String("window".to_string()));
    }
}

#[test]
fn grounds_links_self_state_to_memory_evidence() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    insert_self_state(&conn, &self_state("ss-w", "wunicorm")).unwrap();
    let memory = memory_proto::nodes::Memory {
        id: "m-1".to_string(),
        predicate: "kept".to_string(),
        certainty: 1.0,
        source: "test".to_string(),
        status: "occurred".to_string(),
        is_ongoing: false,
        significance: "why it matters".to_string(),
        emotions: vec![],
        reflection: "what to make of it".to_string(),
        function_tag: Some("feeling".to_string()),
        run_id: None,
        label: "kept memory".to_string(),
        label_resolved: "kept memory".to_string(),
        label_embedding: None,
        learned_at: ts(2026, 11, 30),
        expire_at: None,
        created_at: ts(2026, 11, 30),
        updated_at: ts(2026, 11, 30),
        layer: 4,
        context: "test".to_string(),
    };
    memory_proto::mutations::insert_memory(&conn, &memory).unwrap();

    link_grounds(
        &conn,
        &NodeRef::new("ss-w", "SelfState"),
        &NodeRef::new("m-1", "Memory"),
        &grounds("g-2", "choice"),
    )
    .unwrap();

    assert_eq!(
        count(
            &conn,
            "MATCH (:Grounds {id: 'g-2'})-[r:TO_Grounds]->(:Memory {id: 'm-1'}) RETURN count(r);"
        ),
        1
    );
}

#[test]
fn trajectory_wires_consecutive_self_states() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    insert_self_state(&conn, &self_state("ss-a", "user")).unwrap();
    insert_self_state(&conn, &self_state("ss-b", "user")).unwrap();
    wire_self_trajectory(&conn, "ss-a", "ss-b").unwrap();
    wire_self_trajectory(&conn, "ss-a", "ss-b").unwrap();

    assert_eq!(
        count(
            &conn,
            "MATCH (:SelfState {id: 'ss-a'})-[r:SELF_TRAJECTORY]->(:SelfState {id: 'ss-b'}) RETURN count(r);"
        ),
        1,
        "MERGE: the trajectory converges, never duplicates"
    );
}

#[test]
fn grounds_edges_and_evidence_are_visible_to_the_snapshot() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    insert_self_state(&conn, &self_state("ss-user", "user")).unwrap();
    insert_note_with_content(&conn, "n-1", "the note that filled the window");
    link_grounds(
        &conn,
        &NodeRef::new("ss-user", "SelfState"),
        &NodeRef::new("n-1", "Note"),
        &grounds("g-1", "window"),
    )
    .unwrap();
    insert_self_state(&conn, &self_state("ss-b", "user")).unwrap();
    wire_self_trajectory(&conn, "ss-user", "ss-b").unwrap();

    let snap = graph_snapshot(&conn).unwrap();
    // The Grounds edge node is in the node inventory…
    assert!(
        snap.nodes
            .contains(&("Grounds".to_string(), "grounds-g-1".to_string()))
    );
    // …and the triple traces the positional claim to its evidence. The
    // SelfState keys on its deterministic id — the semantic-key fallback.
    assert!(snap.edges.contains(&(
        "Grounds".to_string(),
        ("SelfState".to_string(), "ss-user".to_string()),
        (
            "Note".to_string(),
            "the note that filled the window".to_string()
        ),
    )));
}
