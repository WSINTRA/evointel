use chrono::DateTime;
use memory_proto::edges::{Contains, Similar};
use memory_proto::mutations::{
    insert_entity, insert_fact, insert_note, insert_time, link_contains, link_similar,
    wire_time_hierarchy, NodeRef,
};
use memory_proto::nodes::{Entity, Fact, Note, Time};
use memory_proto::queries::{graph_snapshot, structural_similarity, GraphSnapshot};
use memory_proto::schema::EMBEDDING_DIM;
use memory_proto::MemoryDb;

type NodeSpec<'a> = (&'a str, &'a str);
type EdgeSpec<'a> = (&'a str, NodeSpec<'a>, NodeSpec<'a>);
type TimeSpec<'a> = (NodeSpec<'a>, NodeSpec<'a>);

fn node(spec: NodeSpec) -> (String, String) {
    (spec.0.to_string(), spec.1.to_string())
}

fn snapshot(nodes: &[NodeSpec], edges: &[EdgeSpec], time: &[TimeSpec]) -> GraphSnapshot {
    GraphSnapshot {
        nodes: nodes.iter().map(|&n| node(n)).collect(),
        edges: edges
            .iter()
            .map(|&(e, f, t)| (e.to_string(), node(f), node(t)))
            .collect(),
        time_rels: time.iter().map(|&(p, c)| (node(p), node(c))).collect(),
    }
}

fn assert_score(a: &GraphSnapshot, b: &GraphSnapshot, expected: f32) {
    let got = structural_similarity(a, b);
    assert!(
        (got - expected).abs() < 1e-6,
        "got {got}, expected {expected}"
    );
}

#[test]
fn identical_graphs_score_one() {
    let a = snapshot(&[("Note", "hello")], &[], &[]);
    let b = snapshot(&[("Note", "hello")], &[], &[]);
    assert_score(&a, &b, 1.0);
}

#[test]
fn disjoint_graphs_score_zero() {
    let a = snapshot(&[("Note", "hello")], &[], &[]);
    let b = snapshot(&[("Entity", "coffee")], &[], &[]);
    assert_score(&a, &b, 0.0);
}

#[test]
fn both_empty_graphs_score_one() {
    assert_score(&GraphSnapshot::default(), &GraphSnapshot::default(), 1.0);
}

#[test]
fn partial_overlap_is_jaccard() {
    // nodes {a,b} vs {b,c}: shared 1 of union 3.
    let a = snapshot(&[("Note", "a"), ("Note", "b")], &[], &[]);
    let b = snapshot(&[("Note", "b"), ("Note", "c")], &[], &[]);
    assert_score(&a, &b, 1.0 / 3.0);
}

#[test]
fn scores_are_symmetric() {
    let a = snapshot(&[("Note", "a"), ("Note", "b")], &[], &[]);
    let b = snapshot(
        &[("Note", "b"), ("Note", "c")],
        &[("Similar", ("Note", "b"), ("Note", "c"))],
        &[],
    );
    let (ab, ba) = (structural_similarity(&a, &b), structural_similarity(&b, &a));
    assert!((ab - ba).abs() < 1e-9);
}

#[test]
fn torn_edge_write_drops_the_score() {
    let full = snapshot(
        &[("Note", "n"), ("Entity", "e"), ("Contains", "ce")],
        &[("Contains", ("Note", "n"), ("Entity", "e"))],
        &[],
    );
    // Same three nodes written, but the edge never got linked.
    let torn = snapshot(
        &[("Note", "n"), ("Entity", "e"), ("Contains", "ce")],
        &[],
        &[],
    );
    // shared: 3 nodes + 0 edges; union: 3 nodes + 1 edge.
    assert_score(&full, &torn, 3.0 / 4.0);
}

#[test]
fn same_structure_different_node_names_differ() {
    // Guards the semantic-key choice: identity lives in the name, so two
    // graphs with the same shape but different concepts are *not* similar
    // just because labels line up.
    let a = snapshot(
        &[("Entity", "coffee"), ("Entity", "sleep")],
        &[("Similar", ("Entity", "coffee"), ("Entity", "sleep"))],
        &[],
    );
    let b = snapshot(
        &[("Entity", "tea"), ("Entity", "rest")],
        &[("Similar", ("Entity", "tea"), ("Entity", "rest"))],
        &[],
    );
    // Shared elements: none. The `(label, name)` pair IS the identity, so
    // matching shapes with different concepts must score 0, not "looks
    // structurally similar, partial credit".
    assert_score(&a, &b, 0.0);
}

// ─── Live-graph snapshot tests ──────────────────────────────────────

fn ts() -> DateTime<chrono::Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}

fn open_test_db(path: &std::path::Path) -> MemoryDb {
    let db = MemoryDb::open(path, EMBEDDING_DIM).unwrap();
    {
        let conn = db.connect().unwrap();
        memory_proto::schema::init(&conn).unwrap();
    }
    db
}

/// Builds a small but structurally varied graph: notes, entities, a
/// fact, a time hierarchy, a Contains edge, and a Similar edge.
/// Ids are prefixed; *names* are identical across prefixes — that's the
/// property the semantic-key design has to hold up.
fn build_graph(conn: &lbug::Connection, p: &str) {
    let note = Note {
        id: format!("{p}-note-1"),
        content: "shopping for coffee".to_string(),
        content_embedding: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 0,
        context: "test".to_string(),
    };
    insert_note(conn, &note).unwrap();

    let ent = Entity {
        id: format!("{p}-ent-1"),
        label: "coffee".to_string(),
        label_resolved: "coffee".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    insert_entity(conn, &ent).unwrap();

    let ent2 = Entity {
        id: format!("{p}-ent-2"),
        label: "espresso".to_string(),
        label_resolved: "espresso machine".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    insert_entity(conn, &ent2).unwrap();

    let fact = Fact {
        id: format!("{p}-fact-1"),
        predicate: "belongs_to".to_string(),
        certainty: 1.0,
        source: "obs".to_string(),
        label: "coffee is a bean".to_string(),
        label_resolved: "coffee is a bean".to_string(),
        label_embedding: None,
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 2,
        context: "test".to_string(),
    };
    insert_fact(conn, &fact).unwrap();

    let parent = Time {
        id: format!("{p}-t-parent"),
        granularity: "month".to_string(),
        starts_at: DateTime::from_timestamp(1_762_214_400, 0).unwrap(),
        ends_at: None,
        label: "November 2026".to_string(),
        label_resolved: "November 2026".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    insert_time(conn, &parent).unwrap();

    let child = Time {
        id: format!("{p}-t-child"),
        granularity: "day".to_string(),
        starts_at: DateTime::from_timestamp(1_762_214_400, 0).unwrap(),
        ends_at: None,
        label: "Nov 3".to_string(),
        label_resolved: "November 3, 2026".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    insert_time(conn, &child).unwrap();
    wire_time_hierarchy(conn, &format!("{p}-t-parent"), &format!("{p}-t-child")).unwrap();

    let contains = Contains {
        id: format!("{p}-e-contains"),
        containment_types: "mention".to_string(),
        weight: 1.0,
        label: "contains".to_string(),
        label_resolved: "note mentions coffee".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    link_contains(
        conn,
        &NodeRef::new(format!("{p}-note-1"), "Note"),
        &NodeRef::new(format!("{p}-ent-1"), "Entity"),
        &contains,
    )
    .unwrap();

    let similar = Similar {
        id: format!("{p}-e-similar"),
        similarity: 0.9,
        sim_context: "semantic".to_string(),
        sim_method: "cosine".to_string(),
        label: "similar".to_string(),
        label_resolved: "coffee ~ espresso machine".to_string(),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    };
    link_similar(
        conn,
        &NodeRef::new(format!("{p}-ent-1"), "Entity"),
        &NodeRef::new(format!("{p}-ent-2"), "Entity"),
        &similar,
    )
    .unwrap();
}

#[test]
fn rebuild_with_different_ids_scores_one() {
    let d1 = tempfile::tempdir().unwrap();
    let db1 = open_test_db(&d1.path().join("one.db"));
    let c1 = db1.connect().unwrap();
    build_graph(&c1, "a");

    let d2 = tempfile::tempdir().unwrap();
    let db2 = open_test_db(&d2.path().join("two.db"));
    let c2 = db2.connect().unwrap();
    build_graph(&c2, "b");

    let s1 = graph_snapshot(&c1).unwrap();
    let s2 = graph_snapshot(&c2).unwrap();

    // Guard against vacuous success: the snapshots must be non-empty and
    // contain what we wrote — including the edge triple with full
    // semantic keys extracted from returned nodes.
    assert!(!s1.is_empty(), "snapshot should not be empty");
    assert!(s1
        .nodes
        .contains(&("Entity".to_string(), "coffee".to_string())));
    assert!(s1
        .nodes
        .contains(&("Note".to_string(), "shopping for coffee".to_string())));
    assert!(s1
        .nodes
        .contains(&("Time".to_string(), "November 3, 2026".to_string())));
    assert!(s1
        .nodes
        .contains(&("Contains".to_string(), "note mentions coffee".to_string())));
    let expected_edges: std::collections::BTreeSet<(_, _, _)> = [
        (
            "Contains",
            ("Note", "shopping for coffee"),
            ("Entity", "coffee"),
        ),
        (
            "Similar",
            ("Entity", "coffee"),
            ("Entity", "espresso machine"),
        ),
    ]
    .into_iter()
    .map(|(e, f, t)| (e.to_string(), node(f), node(t)))
    .collect();
    assert_eq!(s1.edges, expected_edges);

    // Superset, not equality: schema::init seeds a time tree
    // (2026 → 2026-11 → 2026-11-03) into every fresh DB. Our hierarchy
    // rel must be present; the seeded rels are identical in both DBs, so
    // they don't disturb the similarity comparison.
    let our_rel = (
        node(("Time", "November 2026")),
        node(("Time", "November 3, 2026")),
    );
    assert!(s1.time_rels.contains(&our_rel));
    assert!(s1.time_rels.len() >= 3, "expected the seeded time tree too");

    // The headline property: identical judgments, completely different
    // ids ("a-*" vs "b-*") → structurally the same graph.
    assert_score(&s1, &s2, 1.0);
}

#[test]
fn torn_edge_write_visible_in_snapshot() {
    let d1 = tempfile::tempdir().unwrap();
    let db1 = open_test_db(&d1.path().join("one.db"));
    let c1 = db1.connect().unwrap();
    build_graph(&c1, "a");

    // Second graph: same content nodes and times, but the Contains edge
    // node got written (step 1 of the link) and never linked (steps 2-3
    // "crashed") — the classic torn write.
    let d2 = tempfile::tempdir().unwrap();
    let db2 = open_test_db(&d2.path().join("two.db"));
    let c2 = db2.connect().unwrap();
    build_graph(&c2, "b");
    c2.query("MATCH (e:Contains) DETACH DELETE e;").unwrap();
    c2.query(
        "MERGE (e:Contains {id: 'b-e-contains'}) \
         SET e.label_resolved = 'note mentions coffee';",
    )
    .unwrap();

    let s1 = graph_snapshot(&c1).unwrap();
    let s2 = graph_snapshot(&c2).unwrap();

    // The edge node exists as a node...
    assert!(s2
        .nodes
        .contains(&("Contains".to_string(), "note mentions coffee".to_string())));
    // ...but contributes no Contains triple — the torn write is visible.
    // (The fixture's Similar edge survives; only Contains was torn.)
    assert!(
        !s2.edges.iter().any(|(label, _, _)| label == "Contains"),
        "no Contains triple should exist after the torn write"
    );
    let score = structural_similarity(&s1, &s2);
    assert!(
        (0.0..1.0).contains(&score),
        "torn write must score strictly below 1.0, got {score}"
    );
}
