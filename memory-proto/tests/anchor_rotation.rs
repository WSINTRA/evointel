//! Anchor selection and note retirement — the anti-repetition contract.
//!
//! The sermon must be about tonight's material: an anchor derived from
//! the run's own notes, ranked with a novelty penalty for recent
//! anchors, and never grounded in notes that have vanished from the
//! source.

use chrono::{DateTime, Utc};
use lbug::Connection;
use memory_proto::db::MemoryDb;
use memory_proto::edges::Contains;
use memory_proto::mutations::{
    insert_dream_run, insert_entity, insert_note, retire_notes, set_note_coordinates, NodeRef,
};
use memory_proto::nodes::{DreamRun, Entity, Note};
use memory_proto::queries::{choose_anchor, connected_notes, user_position};
use memory_proto::schema::EMBEDDING_DIM;
use memory_proto::{extensions, schema, vector};

fn open_test_db(path: &std::path::Path) -> MemoryDb {
    let db = MemoryDb::open(path, EMBEDDING_DIM).unwrap();
    {
        let conn = db.connect().unwrap();
        extensions::load_extensions(&conn).unwrap();
        schema::init(&conn).unwrap();
        vector::init_vector_indexes(&conn).unwrap();
    }
    db
}

fn ts(day: u32) -> DateTime<Utc> {
    DateTime::from_timestamp(
        chrono::NaiveDate::from_ymd_opt(2026, 11, day)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp(),
        0,
    )
    .unwrap()
}

fn one_hot(idx: usize) -> Vec<f32> {
    let mut v = vec![0.0; EMBEDDING_DIM];
    v[idx] = 1.0;
    v
}

fn note(id: &str, day: u32) -> Note {
    Note {
        id: id.to_string(),
        content: format!("note {id}"),
        content_embedding: Some(one_hot(0)),
        created_at: ts(day),
        updated_at: ts(day),
        layer: 0,
        context: "anchor-test".to_string(),
    }
}

fn entity(id: &str, label: &str, embedding: Vec<f32>, learned_day: u32) -> Entity {
    Entity {
        id: id.to_string(),
        label: label.to_string(),
        label_resolved: label.to_string(),
        label_embedding: Some(embedding),
        learned_at: ts(learned_day),
        expire_at: None,
        created_at: ts(learned_day),
        updated_at: ts(learned_day),
        layer: 1,
        context: "anchor-test".to_string(),
    }
}

fn contains(id: &str) -> Contains {
    Contains {
        id: id.to_string(),
        containment_types: "mention".to_string(),
        weight: 1.0,
        label: "contains".to_string(),
        label_resolved: "note mentions concept".to_string(),
        learned_at: ts(1),
        expire_at: None,
        created_at: ts(1),
        updated_at: ts(1),
        layer: 1,
        context: "anchor-test".to_string(),
    }
}

fn wire(conn: &Connection, note_id: &str, entity_id: &str, edge_id: &str) {
    memory_proto::mutations::link_contains(
        conn,
        &NodeRef::new(note_id, "Note"),
        &NodeRef::new(entity_id, "Entity"),
        &contains(edge_id),
    )
    .unwrap();
}

fn add_concept(conn: &Connection, note_id: &str, entity: &Entity, edge_id: &str) {
    insert_entity(conn, entity).unwrap();
    wire(conn, note_id, &entity.id, edge_id);
}

#[test]
fn the_anchor_comes_from_the_runs_notes() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("anchor.db"));
    let conn = db.connect().unwrap();

    let n1 = note("n1", 3);
    let n2 = note("n2", 4);
    insert_note(&conn, &n1).unwrap();
    insert_note(&conn, &n2).unwrap();
    add_concept(
        &conn,
        "n1",
        &entity("e-job", "job search", one_hot(0), 3),
        "c1",
    );
    add_concept(
        &conn,
        "n2",
        &entity("e-garden", "garden", one_hot(1), 4),
        "c2",
    );

    let now = ts(5);
    let job = choose_anchor(&conn, &[n1], &one_hot(0), now)
        .unwrap()
        .expect("the job note has an anchor");
    assert_eq!(job.entity.id, "e-job");

    // A different night, a different note: the anchor follows the note,
    // not a global "general" query.
    let garden = choose_anchor(&conn, &[n2], &one_hot(1), now)
        .unwrap()
        .expect("the garden note has an anchor");
    assert_eq!(garden.entity.id, "e-garden");
}

#[test]
fn a_recent_anchor_is_penalized_in_favour_of_a_fresh_one() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("anchor.db"));
    let conn = db.connect().unwrap();

    let n1 = note("n1", 3);
    let n2 = note("n2", 4);
    insert_note(&conn, &n1).unwrap();
    insert_note(&conn, &n2).unwrap();
    // Both concepts are supported equally by both notes; e-job is the
    // better semantic match under the run's center, so without history
    // it always wins.
    add_concept(
        &conn,
        "n1",
        &entity("e-job", "job search", one_hot(0), 3),
        "c1",
    );
    add_concept(
        &conn,
        "n2",
        &entity("e-job", "job search", one_hot(0), 3),
        "c2",
    );
    add_concept(
        &conn,
        "n1",
        &entity("e-garden", "garden", one_hot(1), 1),
        "c3",
    );
    add_concept(
        &conn,
        "n2",
        &entity("e-garden", "garden", one_hot(1), 1),
        "c4",
    );

    let queue = vec![n1.clone(), n2.clone()];
    let first = choose_anchor(&conn, &queue, &one_hot(0), ts(5))
        .unwrap()
        .expect("an anchor exists");
    assert_eq!(first.entity.id, "e-job", "best match wins with no history");

    // Yesterday chose e-job. Tonight the same material must not repeat.
    insert_dream_run(
        &conn,
        &DreamRun {
            id: "run-yesterday".to_string(),
            considered: vec!["n1".to_string()],
            dropped: vec![],
            kept_t: 0,
            kept_f: 0,
            kept_s: 0,
            kept_n: 0,
            dropped_total: 0,
            anchor_id: Some("e-job".to_string()),
            started_at: ts(2),
            finished_at: ts(2),
            created_at: ts(2),
            updated_at: ts(2),
            layer: 3,
            context: "anchor-test".to_string(),
        },
    )
    .unwrap();

    let second = choose_anchor(&conn, &queue, &one_hot(0), ts(5))
        .unwrap()
        .expect("an anchor exists");
    assert_eq!(
        second.entity.id, "e-garden",
        "the recent anchor is penalized; novelty wins"
    );
}

#[test]
fn retired_notes_are_history_not_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("anchor.db"));
    let conn = db.connect().unwrap();

    let n1 = note("n1", 3);
    insert_note(&conn, &n1).unwrap();
    add_concept(
        &conn,
        "n1",
        &entity("e-job", "job search", one_hot(0), 3),
        "c1",
    );
    set_note_coordinates(&conn, "n1", 0.5, -0.5).unwrap();

    assert_eq!(connected_notes(&conn, "e-job").unwrap().len(), 1);

    // The source deleted the note: the graph keeps the row, current
    // reads drop it.
    retire_notes(&conn, &["n1".to_string()], ts(6)).unwrap();
    assert!(
        connected_notes(&conn, "e-job").unwrap().is_empty(),
        "a deleted note is never cited again"
    );
    let window = (ts(1), ts(30));
    assert!(
        user_position(&conn, window).unwrap().is_none(),
        "a deleted note does not shape the user's position"
    );

    // Restoring the note (a new ingest of the same id) clears the
    // tombstone.
    insert_note(&conn, &n1).unwrap();
    assert_eq!(connected_notes(&conn, "e-job").unwrap().len(), 1);
}
