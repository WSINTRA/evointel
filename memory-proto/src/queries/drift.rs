//! Drift reads — how a concept's semantic neighborhood changes over
//! time. This is the reflection's habit-inference primitive: "your job
//! search is shifting from anxious to strategic" is a concept whose
//! surroundings moved in embedding space between two windows.
//!
//! A concept's *position in a window* is the centroid of its neighbors'
//! embeddings, restricted to neighbors whose activity time falls in that
//! window. Notes are bucketed by `created_at` (they have no
//! `learned_at`); every other neighbor type uses `learned_at`.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use lbug::{Connection, NodeVal, Value};

use crate::db::DbResult;
use crate::mutations::NodeRef;

use super::fetch::{get_embedding, get_prop, time_to_chrono};
use super::traversal::EDGES;

/// One neighbor observation inside a time window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    pub node: NodeRef,
    pub edge_label: String,
    /// The time this neighbor was bucketed by: its `learned_at`, or
    /// `created_at` for notes.
    pub observed_at: DateTime<Utc>,
}

/// The change in a concept's neighborhood between two windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drift {
    /// `1 - cosine(centroid_from, centroid_to)` in `[0, 2]`.
    /// Zero means the semantic atmosphere did not move.
    pub displacement: f32,
    /// Neighbors active in the `to` window that were not active in the
    /// `from` window.
    pub new_connections: usize,
    /// Neighbors active in the `from` window that were not active in
    /// the `to` window.
    pub dropped_connections: usize,
}

/// A 1-hop neighbor of a concept via a specific edge type.
struct Neighbor {
    id: String,
    label: String,
    edge_label: String,
    embedding: Option<Vec<f32>>,
    /// `learned_at`, or `created_at` for notes; `None` if neither.
    at: Option<DateTime<Utc>>,
}

/// Like `get_prop`, but treats `Value::Null` as absent.
///
/// lbug gotcha: nodes returned through *unlabeled* endpoints in poly-rel
/// patterns are materialized with the union of all table columns, with
/// missing ones as `Null`. So a `Note` read this way carries
/// `learn_at = Null(Timestamp)` even though `Note` has no such column —
/// a plain `or_else` chain on `get_prop` would see `Some(Null)` and
/// never fall through to the fallback column.
fn prop_value<'a>(props: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    match get_prop(props, key) {
        Some(Value::Null(_)) | None => None,
        Some(v) => Some(v),
    }
}

fn to_neighbor(node: &NodeVal, edge_label: &str) -> Option<Neighbor> {
    let props = node.get_properties();
    let id = match prop_value(props, "id") {
        Some(Value::String(s)) => s.clone(),
        _ => return None,
    };
    let embedding = get_embedding(props, "label_embedding")
        .or_else(|| get_embedding(props, "content_embedding"));
    let at = match prop_value(props, "learned_at").or_else(|| prop_value(props, "created_at")) {
        Some(Value::Timestamp(t)) => Some(time_to_chrono(t)),
        _ => None,
    };
    Some(Neighbor {
        id,
        label: node.get_label_name().clone(),
        edge_label: edge_label.to_string(),
        embedding,
        at,
    })
}

/// Collects all 1-hop neighbors of `node` via edge-nodes, both
/// directions, deduped on `(neighbor id, edge label)`.
fn neighborhood(conn: &Connection, node: &NodeRef) -> DbResult<Vec<Neighbor>> {
    let mut out: Vec<Neighbor> = vec![];
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();

    for def in EDGES {
        // Node is the source: (a)-[:FROM_X]->(e:X)-[:TO_X]->(b)
        if def.from.contains(&node.label.as_str()) {
            let q = format!(
                "MATCH (a:{} {{id: $id}})-[:FROM_{}]->(e:{})-[:TO_{}]->(b) RETURN b;",
                node.label, def.edge, def.edge, def.edge
            );
            collect_side(conn, &q, node, def.edge, &mut seen, &mut out)?;
        }
        // Node is the target: (c)-[:FROM_X]->(e:X)-[:TO_X]->(a)
        if def.to.contains(&node.label.as_str()) {
            let q = format!(
                "MATCH (c)-[:FROM_{}]->(e:{})-[:TO_{}]->(a:{} {{id: $id}}) RETURN c;",
                def.edge, def.edge, def.edge, node.label
            );
            collect_side(conn, &q, node, def.edge, &mut seen, &mut out)?;
        }
    }

    Ok(out)
}

fn collect_side(
    conn: &Connection,
    cypher: &str,
    node: &NodeRef,
    edge_label: &str,
    seen: &mut BTreeSet<(String, String)>,
    out: &mut Vec<Neighbor>,
) -> DbResult<()> {
    let mut stmt = conn.prepare(cypher)?;
    let result = conn.execute(&mut stmt, vec![("id", Value::String(node.id.clone()))])?;
    for row in result {
        let Value::Node(n) = &row[0] else { continue };
        let Some(nb) = to_neighbor(n, edge_label) else {
            continue;
        };
        if seen.insert((nb.id.clone(), nb.edge_label.clone())) {
            out.push(nb);
        }
    }
    Ok(())
}

/// Unique neighbors (by node id) active in a window.
fn window_neighbors<'a>(
    nbs: &'a [Neighbor],
    window: &(DateTime<Utc>, DateTime<Utc>),
) -> Vec<&'a Neighbor> {
    let mut seen = BTreeSet::new();
    let mut out = vec![];
    for n in nbs {
        let Some(at) = n.at else { continue };
        if at >= window.0 && at <= window.1 && seen.insert(n.id.as_str()) {
            out.push(n);
        }
    }
    out
}

/// Centroid (element-wise mean) of the embeddings of unique neighbors.
/// `None` when no neighbor carries an embedding — no position, no drift.
fn centroid(nbs: &[&Neighbor]) -> Option<Vec<f32>> {
    let embs: Vec<&Vec<f32>> = nbs.iter().filter_map(|n| n.embedding.as_ref()).collect();
    if embs.is_empty() {
        return None;
    }
    let mut acc = vec![0f32; embs[0].len()];
    for e in &embs {
        for (i, x) in e.iter().enumerate() {
            acc[i] += x;
        }
    }
    #[allow(clippy::cast_precision_loss)] // neighbor counts stay tiny
    for a in &mut acc {
        *a /= embs.len() as f32;
    }
    Some(acc)
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return f32::NAN;
    }
    dot / (na * nb)
}

/// Lists a concept's neighborhood activity inside a window, ordered by
/// observation time (ties broken by node id). No limit — callers slice.
pub fn concept_activity(
    conn: &Connection,
    node: &NodeRef,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> DbResult<Vec<Activity>> {
    let nbs = neighborhood(conn, node)?;
    let mut activities: Vec<Activity> = nbs
        .into_iter()
        .filter_map(|n| {
            let at = n.at?;
            (at >= start && at <= end).then(|| Activity {
                node: NodeRef::new(n.id, n.label),
                edge_label: n.edge_label,
                observed_at: at,
            })
        })
        .collect();
    activities.sort_by(|a, b| {
        a.observed_at
            .cmp(&b.observed_at)
            .then_with(|| a.node.id.cmp(&b.node.id))
    });
    Ok(activities)
}

/// Measures how a concept's semantic neighborhood moved between two
/// windows.
///
/// Returns `Ok(None)` when drift is not measurable: a window with no
/// active neighbors, or none carrying embeddings (e.g. only `Time`
/// nodes — structural, not semantic atmosphere). "Didn't move" and
/// "can't tell" are different claims.
pub fn concept_drift(
    conn: &Connection,
    node: &NodeRef,
    from: (DateTime<Utc>, DateTime<Utc>),
    to: (DateTime<Utc>, DateTime<Utc>),
) -> DbResult<Option<Drift>> {
    let nbs = neighborhood(conn, node)?;
    let n1 = window_neighbors(&nbs, &from);
    let n2 = window_neighbors(&nbs, &to);

    let ids1: BTreeSet<&str> = n1.iter().map(|n| n.id.as_str()).collect();
    let ids2: BTreeSet<&str> = n2.iter().map(|n| n.id.as_str()).collect();
    let new_connections = ids2.difference(&ids1).count();
    let dropped_connections = ids1.difference(&ids2).count();

    let (Some(c1), Some(c2)) = (centroid(&n1), centroid(&n2)) else {
        return Ok(None);
    };
    let cos = cosine(&c1, &c2);
    if !cos.is_finite() {
        return Ok(None);
    }
    Ok(Some(Drift {
        displacement: 1.0 - cos,
        new_connections,
        dropped_connections,
    }))
}
