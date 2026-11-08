use chrono::{DateTime, Utc};
use memory_proto::MemoryDb;
use memory_proto::edges::{Contains, HasProperty, Similar};
use memory_proto::mutations::{
    NodeRef, insert_entity, insert_fact, insert_note, insert_time, link_contains,
    link_has_property, link_similar,
};
use memory_proto::nodes::{Entity, Fact, Note, Time};
use memory_proto::queries::{concept_activity, concept_drift};
use memory_proto::schema::EMBEDDING_DIM;

/// Window A anchor: Nov 2023.
const T_A: i64 = 1_700_000_000;
/// Midpoint window C: ~45 days later (only the embedding-less Time node).
const T_C: i64 = 1_703_888_000;
/// Window B anchor: ~90 days later.
const T_B: i64 = 1_707_776_000;

fn ts(t: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(t, 0).unwrap()
}

fn day(t: i64) -> (DateTime<Utc>, DateTime<Utc>) {
    (ts(t - 86_400), ts(t + 86_400))
}

fn one_hot(idx: usize) -> Vec<f32> {
    let mut v = vec![0.0; EMBEDDING_DIM];
    v[idx] = 1.0;
    v
}

fn entity(id: &str, name: &str, learned_at: i64, embedding: Vec<f32>) -> Entity {
    Entity {
        id: id.to_string(),
        label: name.to_string(),
        label_resolved: name.to_string(),
        label_embedding: Some(embedding),
        learned_at: ts(learned_at),
        expire_at: None,
        created_at: ts(learned_at),
        updated_at: ts(learned_at),
        layer: 1,
        context: "drift-test".to_string(),
    }
}

/// The concept under study is Entity "coffee". Its neighborhood:
/// - window A: "espresso machine" (Similar, coffee is FROM) and a Note
///   (Contains, coffee is TO) — both embedded near [1,0,...]
/// - window C: a Time node only (Contains) — no embedding
/// - window B: "tea" (Similar) and a Fact (`HasProperty`, coffee is TO) —
///   both embedded near [0,1,0,...]
fn build_fixture(conn: &lbug::Connection) {
    insert_entity(conn, &entity("d-ent-coffee", "coffee", T_A, one_hot(0))).unwrap();

    // Window A neighbors.
    insert_entity(
        conn,
        &entity("d-ent-espresso", "espresso machine", T_A, one_hot(0)),
    )
    .unwrap();
    let similar_a = Similar {
        id: "d-e-similar-a".to_string(),
        similarity: 0.9,
        sim_context: "semantic".to_string(),
        sim_method: "cosine".to_string(),
        label: "similar".to_string(),
        label_resolved: "coffee ~ espresso machine".to_string(),
        learned_at: ts(T_A),
        expire_at: None,
        created_at: ts(T_A),
        updated_at: ts(T_A),
        layer: 1,
        context: "drift-test".to_string(),
    };
    link_similar(
        conn,
        &NodeRef::new("d-ent-coffee", "Entity"),
        &NodeRef::new("d-ent-espresso", "Entity"),
        &similar_a,
    )
    .unwrap();

    let note = Note {
        id: "d-note-1".to_string(),
        content: "shopping for coffee".to_string(),
        content_embedding: Some(one_hot(0)),
        created_at: ts(T_A),
        updated_at: ts(T_A),
        layer: 0,
        context: "drift-test".to_string(),
    };
    insert_note(conn, &note).unwrap();
    let contains_a = Contains {
        id: "d-e-contains-a".to_string(),
        containment_types: "mention".to_string(),
        weight: 1.0,
        label: "contains".to_string(),
        label_resolved: "note mentions coffee".to_string(),
        learned_at: ts(T_A),
        expire_at: None,
        created_at: ts(T_A),
        updated_at: ts(T_A),
        layer: 1,
        context: "drift-test".to_string(),
    };
    link_contains(
        conn,
        &NodeRef::new("d-note-1", "Note"),
        &NodeRef::new("d-ent-coffee", "Entity"),
        &contains_a,
    )
    .unwrap();

    // Window C neighbor: a Time node (no embedding column at all).
    let time = Time {
        id: "d-time-1".to_string(),
        granularity: "day".to_string(),
        starts_at: ts(T_C),
        ends_at: None,
        label: "Nov 30".to_string(),
        label_resolved: "November 30, 2023".to_string(),
        learned_at: ts(T_C),
        expire_at: None,
        created_at: ts(T_C),
        updated_at: ts(T_C),
        layer: 1,
        context: "drift-test".to_string(),
    };
    insert_time(conn, &time).unwrap();
    let contains_c = Contains {
        id: "d-e-contains-c".to_string(),
        containment_types: "anchor".to_string(),
        weight: 1.0,
        label: "contains".to_string(),
        label_resolved: "time anchors coffee".to_string(),
        learned_at: ts(T_C),
        expire_at: None,
        created_at: ts(T_C),
        updated_at: ts(T_C),
        layer: 1,
        context: "drift-test".to_string(),
    };
    link_contains(
        conn,
        &NodeRef::new("d-time-1", "Time"),
        &NodeRef::new("d-ent-coffee", "Entity"),
        &contains_c,
    )
    .unwrap();

    // Window B neighbors.
    insert_entity(conn, &entity("d-ent-tea", "tea", T_B, one_hot(1))).unwrap();
    let similar_b = Similar {
        id: "d-e-similar-b".to_string(),
        similarity: 0.8,
        sim_context: "semantic".to_string(),
        sim_method: "cosine".to_string(),
        label: "similar".to_string(),
        label_resolved: "coffee ~ tea".to_string(),
        learned_at: ts(T_B),
        expire_at: None,
        created_at: ts(T_B),
        updated_at: ts(T_B),
        layer: 1,
        context: "drift-test".to_string(),
    };
    link_similar(
        conn,
        &NodeRef::new("d-ent-coffee", "Entity"),
        &NodeRef::new("d-ent-tea", "Entity"),
        &similar_b,
    )
    .unwrap();

    let fact = Fact {
        id: "d-fact-1".to_string(),
        predicate: "belongs_to".to_string(),
        certainty: 1.0,
        source: "obs".to_string(),
        label: "coffee is a bean".to_string(),
        label_resolved: "coffee is a bean".to_string(),
        label_embedding: Some(one_hot(1)),
        learned_at: ts(T_B),
        expire_at: None,
        created_at: ts(T_B),
        updated_at: ts(T_B),
        layer: 2,
        context: "drift-test".to_string(),
    };
    insert_fact(conn, &fact).unwrap();
    let has_prop = HasProperty {
        id: "d-e-hasprop-1".to_string(),
        property_name: "category".to_string(),
        property_value: "beverage".to_string(),
        prop_context: "drift-test".to_string(),
        certainty: 1.0,
        label: "has property".to_string(),
        label_resolved: "coffee has category beverage".to_string(),
        learned_at: ts(T_B),
        expire_at: None,
        created_at: ts(T_B),
        updated_at: ts(T_B),
        layer: 1,
        context: "drift-test".to_string(),
    };
    link_has_property(
        conn,
        &NodeRef::new("d-fact-1", "Fact"),
        &NodeRef::new("d-ent-coffee", "Entity"),
        &has_prop,
    )
    .unwrap();
}

fn open_test_db(path: &std::path::Path) -> MemoryDb {
    let db = MemoryDb::open(path, EMBEDDING_DIM).unwrap();
    {
        let conn = db.connect().unwrap();
        memory_proto::schema::init(&conn).unwrap();
    }
    db
}

fn coffee() -> NodeRef {
    NodeRef::new("d-ent-coffee", "Entity")
}

#[test]
fn activity_lists_window_neighbors_ordered() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("drift.db"));
    let conn = db.connect().unwrap();
    build_fixture(&conn);

    let (start, end) = day(T_A);
    let activities = concept_activity(&conn, &coffee(), start, end).unwrap();

    // Window A holds the espresso entity and the note — but NOT the
    // window-B/C neighbors (tea, fact, time).
    let ids: Vec<&str> = activities.iter().map(|a| a.node.id.as_str()).collect();
    assert_eq!(ids, vec!["d-ent-espresso", "d-note-1"]);
    assert_eq!(activities[0].edge_label, "Similar");
    assert_eq!(activities[1].edge_label, "Contains");
    assert_eq!(activities[0].observed_at, ts(T_A));

    // Window C holds only the Time node — present in activity even
    // though it carries no embedding.
    let (c_start, c_end) = day(T_C);
    let c = concept_activity(&conn, &coffee(), c_start, c_end).unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].node.id, "d-time-1");
    assert_eq!(c[0].node.label, "Time");
}

#[test]
fn drift_measures_displacement_between_windows() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("drift.db"));
    let conn = db.connect().unwrap();
    build_fixture(&conn);

    let drift = concept_drift(&conn, &coffee(), day(T_A), day(T_B))
        .unwrap()
        .expect("both windows have embedding-bearing neighbors");

    // Centroid A = [1,0,...] (espresso + note), centroid B = [0,1,0,...]
    // (tea + fact). Orthogonal → cosine 0 → displacement exactly 1.
    assert!(
        (drift.displacement - 1.0).abs() < 1e-6,
        "displacement {}",
        drift.displacement
    );
    assert_eq!(drift.new_connections, 2);
    assert_eq!(drift.dropped_connections, 2);
}

#[test]
fn drift_is_none_when_not_measurable() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("drift.db"));
    let conn = db.connect().unwrap();
    build_fixture(&conn);

    // Window C has neighbors, but none with embeddings (Time only).
    let r = concept_drift(&conn, &coffee(), day(T_A), day(T_C)).unwrap();
    assert!(r.is_none(), "no-embedding window must be unmeasurable");

    // A window with no activity at all (year 2030).
    let empty = (ts(1_890_000_000), ts(1_891_000_000));
    let r = concept_drift(&conn, &coffee(), day(T_A), empty).unwrap();
    assert!(r.is_none(), "empty window must be unmeasurable");

    // A concept with no edges at all.
    insert_entity(
        &conn,
        &entity("d-ent-lonely", "lonely concept", T_A, one_hot(0)),
    )
    .unwrap();
    let r = concept_drift(
        &conn,
        &NodeRef::new("d-ent-lonely", "Entity"),
        day(T_A),
        day(T_B),
    )
    .unwrap();
    assert!(r.is_none(), "edge-less concept must be unmeasurable");
}
