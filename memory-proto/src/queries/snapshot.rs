use std::collections::BTreeSet;

use lbug::{Connection, NodeVal, Value};

use crate::db::DbResult;
use crate::edges::EdgeType;

/// Identity of a node by *meaning*, not by UUID: `(table label, name)`.
///
/// Content types key on `label_resolved` (`Note`s on `content`); edge
/// types key on their `label_resolved` as well. IDs are caller-generated
/// and differ across independent dream runs, so they can never be part of
/// a structural fingerprint — two runs that made the same judgment must
/// score as one graph.
pub type NodeKey = (String, String);

/// A linked edge: `(edge table, from node, to node)`.
///
/// Edge *nodes* live in [`GraphSnapshot::nodes`] on their own; an
/// unlinked or half-linked edge node (a `MERGE` that wrote step 1 but
/// died before step 2 or 3) therefore shows up as a node with no matching
/// triple — exactly the kind of torn write the nightmare detector needs
/// to be able to see.
pub type EdgeTriple = (String, NodeKey, NodeKey);

/// A `TIME_HIERARCHY` rel: `(parent, child)`.
pub type TimeRel = (NodeKey, NodeKey);

/// Structural fingerprint of a graph: every node, every linked edge
/// triple, every time-hierarchy rel. Embeddings are deliberately absent —
/// drift is measured by `concept_drift`, not by rebuild similarity.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphSnapshot {
    /// All nodes across the 7 content tables and the 12 edge tables.
    pub nodes: BTreeSet<NodeKey>,
    /// Linked edges only; see [`EdgeTriple`].
    pub edges: BTreeSet<EdgeTriple>,
    /// `TIME_HIERARCHY` rels between `Time` nodes.
    pub time_rels: BTreeSet<TimeRel>,
}

impl GraphSnapshot {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.edges.is_empty() && self.time_rels.is_empty()
    }
}

/// Intersection and union sizes of two element sets.
fn jaccard_counts<T: Ord>(x: &BTreeSet<T>, y: &BTreeSet<T>) -> (usize, usize) {
    (x.intersection(y).count(), x.union(y).count())
}

/// Structural similarity in `[0, 1]`: summed Jaccard overlap across
/// nodes, edge triples, and time rels.
///
/// Both snapshots empty → `1.0`: two empty graphs are structurally
/// identical, and `0/0` has no sensible other answer.
pub fn structural_similarity(a: &GraphSnapshot, b: &GraphSnapshot) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let mut shared = 0usize;
    let mut total = 0usize;
    for (s, t) in [
        jaccard_counts(&a.nodes, &b.nodes),
        jaccard_counts(&a.edges, &b.edges),
        jaccard_counts(&a.time_rels, &b.time_rels),
    ] {
        shared += s;
        total += t;
    }
    #[allow(clippy::cast_precision_loss)] // counts stay far below 2^24
    {
        shared as f32 / total as f32
    }
}

// ─── Snapshotting a live graph ───────────────────────────────────────
//
// One query per node table, one per edge type, one for the time
// hierarchy. It is a read-path primitive: no mutations, no state.

/// Content tables and the column that names them semantically.
/// Notes are keyed by `content` (they have no `label_resolved`).
const CONTENT_TABLES: &[(&str, &str)] = &[
    ("Note", "content"),
    ("Entity", "label_resolved"),
    ("Time", "label_resolved"),
    ("AbstractTime", "label_resolved"),
    ("Fact", "label_resolved"),
    ("Event", "label_resolved"),
    ("Memory", "label_resolved"),
];

/// Builds the structural fingerprint of the whole graph.
///
/// Unlinked or half-linked edge nodes land in `nodes` but contribute no
/// edge triple — torn writes stay visible to the similarity score.
pub fn graph_snapshot(conn: &Connection) -> DbResult<GraphSnapshot> {
    let mut snap = GraphSnapshot::default();

    for (table, name_col) in CONTENT_TABLES {
        collect_table_names(conn, table, name_col, &mut snap.nodes)?;
    }
    for etype in EdgeType::ALL {
        collect_table_names(conn, etype.label(), "label_resolved", &mut snap.nodes)?;
        collect_edge_triples(conn, etype, &mut snap.edges)?;
    }
    collect_time_rels(conn, &mut snap.time_rels)?;

    Ok(snap)
}

/// Semantic key of a returned node: `(label name, label_resolved|content)`.
fn semantic_key(node: &NodeVal) -> Option<NodeKey> {
    let props = node.get_properties();
    let name = string_prop(props, "label_resolved").or_else(|| string_prop(props, "content"))?;
    Some((node.get_label_name().clone(), name))
}

fn string_prop(props: &[(String, Value)], key: &str) -> Option<String> {
    props
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
}

fn collect_table_names(
    conn: &Connection,
    table: &str,
    name_col: &str,
    out: &mut BTreeSet<NodeKey>,
) -> DbResult<()> {
    let cypher = format!("MATCH (n:{table}) RETURN n.{name_col};");
    for row in conn.query(&cypher)? {
        let Value::String(name) = &row[0] else {
            continue;
        };
        out.insert((table.to_string(), name.clone()));
    }
    Ok(())
}

fn collect_edge_triples(
    conn: &Connection,
    etype: EdgeType,
    out: &mut BTreeSet<EdgeTriple>,
) -> DbResult<()> {
    let label = etype.label();
    let cypher = format!("MATCH (a)-[:FROM_{label}]->(e:{label})-[:TO_{label}]->(b) RETURN a, b;");
    for row in conn.query(&cypher)? {
        let (Value::Node(from), Value::Node(to)) = (&row[0], &row[1]) else {
            continue;
        };
        let (Some(f), Some(t)) = (semantic_key(from), semantic_key(to)) else {
            continue;
        };
        out.insert((label.to_string(), f, t));
    }
    Ok(())
}

fn collect_time_rels(conn: &Connection, out: &mut BTreeSet<TimeRel>) -> DbResult<()> {
    for row in conn.query("MATCH (p:Time)-[:TIME_HIERARCHY]->(c:Time) RETURN p, c;")? {
        let (Value::Node(parent), Value::Node(child)) = (&row[0], &row[1]) else {
            continue;
        };
        let (Some(p), Some(c)) = (semantic_key(parent), semantic_key(child)) else {
            continue;
        };
        out.insert((p, c));
    }
    Ok(())
}
