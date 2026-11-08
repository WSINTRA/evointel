use lbug::{Connection, NodeVal, Value};

use crate::db::DbResult;
use crate::mutations::NodeRef;
use crate::nodes::Note;

use super::fetch::get_note;

/// Which node labels each edge type connects from and to.
/// Mirrors the poly-rel DDL — needed because the lbug binder rejects
/// `MATCH` patterns whose node labels don't fit the rel table.
pub(crate) struct EdgeDef {
    pub(crate) edge: &'static str,
    pub(crate) from: &'static [&'static str],
    pub(crate) to: &'static [&'static str],
}

pub(crate) const EDGES: &[EdgeDef] = &[
    EdgeDef {
        edge: "Contains",
        from: &["Memory", "Event", "Fact", "Note", "Time"],
        to: &["Memory", "Event", "Fact", "Entity", "Time"],
    },
    EdgeDef {
        edge: "Similar",
        from: &["Note", "Entity", "Fact", "Event", "Memory"],
        to: &["Note", "Entity", "Fact", "Event", "Memory"],
    },
    EdgeDef {
        edge: "HasProperty",
        from: &["Note", "Entity", "Fact", "Event", "Memory"],
        to: &["Entity"],
    },
    EdgeDef {
        edge: "LeadsTo",
        from: &["Memory", "Event", "Fact"],
        to: &["Memory", "Event", "Fact", "AbstractTime"],
    },
    EdgeDef {
        edge: "Causes",
        from: &["Memory", "Event", "Fact"],
        to: &["Event", "Fact"],
    },
    EdgeDef {
        edge: "Prevents",
        from: &["Event", "Fact"],
        to: &["Event", "Fact", "Memory"],
    },
    EdgeDef {
        edge: "BecauseOf",
        from: &["Memory", "Event"],
        to: &["Event", "Fact"],
    },
    EdgeDef {
        edge: "Before",
        from: &["Note", "Time", "Event", "AbstractTime"],
        to: &["Time", "Event", "AbstractTime"],
    },
    EdgeDef {
        edge: "After",
        from: &["Note", "Time", "Event", "AbstractTime"],
        to: &["Time", "Event", "AbstractTime"],
    },
    EdgeDef {
        edge: "During",
        from: &["Note", "Time", "Event"],
        to: &["Time", "Event"],
    },
    EdgeDef {
        edge: "ValidFrom",
        from: &["Time", "AbstractTime"],
        to: &["Note", "Fact", "Event", "Memory"],
    },
    EdgeDef {
        edge: "ValidTo",
        from: &["Time", "AbstractTime"],
        to: &["Note", "Fact", "Event", "Memory"],
    },
];

fn node_id(node: &NodeVal) -> String {
    node.get_properties()
        .iter()
        .find(|(k, _)| k == "id")
        .and_then(|(_, v)| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Finds neighbors of a node via edge-node hops (BFS).
///
/// Each hop follows `FROM_X` rels from the node to an edge node, then
/// `TO_X` rels from that edge node to the neighbor — and also the reverse
/// direction, where the node is the edge's target.
///
/// Returns (neighbor node, path of edge labels) pairs.
pub fn neighbors(
    conn: &Connection,
    node: &NodeRef,
    depth: usize,
) -> DbResult<Vec<(NodeVal, Vec<String>)>> {
    if depth == 0 {
        return Ok(vec![]);
    }

    let mut visited: Vec<String> = vec![node.id.clone()];
    let mut results: Vec<(NodeVal, Vec<String>)> = vec![];

    // Frontier entries: (id, label, path_so_far)
    let mut frontier: Vec<(String, String, Vec<String>)> =
        vec![(node.id.clone(), node.label.clone(), vec![])];

    for _hop in 0..depth {
        let mut next: Vec<(String, String, Vec<String>)> = vec![];

        for (id, label, path) in &frontier {
            for def in EDGES {
                // Node is a source: (a)-[:FROM_X]->(e:X)-[:TO_X]->(b)
                if def.from.contains(&label.as_str()) {
                    let q = format!(
                        "MATCH (a:{label} {{id: $id}})-[:FROM_{e}]->(e:{e})-[:TO_{e}]->(b) RETURN b;",
                        e = def.edge
                    );
                    let mut stmt = conn.prepare(&q)?;
                    let result =
                        conn.execute(&mut stmt, vec![("id", Value::String(id.clone()))])?;
                    for row in result {
                        if let Value::Node(n) = &row[0] {
                            let nid = node_id(n);
                            if nid == *id || visited.contains(&nid) {
                                continue;
                            }
                            visited.push(nid.clone());
                            let mut new_path = path.clone();
                            new_path.push(def.edge.to_string());
                            next.push((nid, n.get_label_name().clone(), new_path.clone()));
                            results.push((n.clone(), new_path));
                        }
                    }
                }

                // Node is a target: (c)-[:FROM_X]->(e:X)-[:TO_X]->(a)
                if def.to.contains(&label.as_str()) {
                    let q = format!(
                        "MATCH (c)-[:FROM_{e}]->(e:{e})-[:TO_{e}]->(a:{label} {{id: $id}}) RETURN c;",
                        e = def.edge
                    );
                    let mut stmt = conn.prepare(&q)?;
                    let result =
                        conn.execute(&mut stmt, vec![("id", Value::String(id.clone()))])?;
                    for row in result {
                        if let Value::Node(n) = &row[0] {
                            let nid = node_id(n);
                            if nid == *id || visited.contains(&nid) {
                                continue;
                            }
                            visited.push(nid.clone());
                            let mut new_path = path.clone();
                            new_path.push(def.edge.to_string());
                            next.push((nid, n.get_label_name().clone(), new_path.clone()));
                            results.push((n.clone(), new_path));
                        }
                    }
                }
            }
        }

        frontier = next;
    }

    Ok(results)
}

/// Returns all Notes connected to a given entity via edge-nodes.
pub fn connected_notes(conn: &Connection, entity_id: &str) -> DbResult<Vec<Note>> {
    let mut notes = Vec::new();
    let mut seen: Vec<String> = vec![];

    // Entity is the target: Note → edge → Entity
    let q1 = "MATCH (n:Note)-[]->(e)-[]->(ent:Entity {id: $id}) RETURN n;";
    let mut stmt = conn.prepare(q1)?;
    let result = conn.execute(
        &mut stmt,
        vec![("id", Value::String(entity_id.to_string()))],
    )?;
    for row in result {
        if let Value::Node(n) = &row[0] {
            let nid = node_id(n);
            if !seen.contains(&nid) {
                seen.push(nid.clone());
                if let Some(note) = get_note(conn, &nid)? {
                    notes.push(note);
                }
            }
        }
    }

    // Entity is the source: Entity → edge → Note
    let q2 = "MATCH (ent:Entity {id: $id})-[]->(e)-[]->(n:Note) RETURN n;";
    let mut stmt = conn.prepare(q2)?;
    let result = conn.execute(
        &mut stmt,
        vec![("id", Value::String(entity_id.to_string()))],
    )?;
    for row in result {
        if let Value::Node(n) = &row[0] {
            let nid = node_id(n);
            if !seen.contains(&nid) {
                seen.push(nid.clone());
                if let Some(note) = get_note(conn, &nid)? {
                    notes.push(note);
                }
            }
        }
    }

    Ok(notes)
}
