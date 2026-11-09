//! The mirror guard: `EdgeType::allowed_from_labels` /
//! `allowed_to_labels` must match the poly-rel DDL that actually
//! builds the schema. If a migration batch and the Rust allowlists
//! ever drift, the dreamer's link fence would validate against the
//! wrong law — this test fails first.

use memory_proto::edges::EdgeType;
use memory_proto::schema::poly_relations::ALL_POLY_RELATIONS;
use memory_proto::schema::self_edges::GROUNDS_RELS;

/// The `Grounds` rels live in the self layer's batch, not the pillar
/// DDL — fold them in.
const GROUNDS_DDL: &str = GROUNDS_RELS;

/// Collect every `FROM <a> TO <b>` pair from a DDL chunk, table by
/// table. Returns (table name, source label, target label) triples.
fn parse_pairs(ddl: &str) -> Vec<(String, String, String)> {
    let mut pairs = vec![];
    for stmt in ddl.split("CREATE REL TABLE IF NOT EXISTS ") {
        let Some(rest) = stmt.split_once('(') else {
            continue;
        };
        let table = rest.0.trim().to_string();
        if let Some(body) = rest.1.split(')').next() {
            for line in body.split(',') {
                let line = line.trim();
                if let Some(after_from) = line.strip_prefix("FROM ")
                    && let Some((a, b)) = after_from.split_once(" TO ")
                {
                    pairs.push((table.clone(), a.trim().to_string(), b.trim().to_string()));
                }
            }
        }
    }
    pairs
}

#[test]
fn allowlists_mirror_the_poly_rel_ddl() {
    let mut all_ddl = ALL_POLY_RELATIONS.join("\n");
    all_ddl.push('\n');
    all_ddl.push_str(GROUNDS_DDL);
    let pairs = parse_pairs(&all_ddl);
    assert!(!pairs.is_empty(), "parsed no pairs from the DDL");

    for edge in EdgeType::ALL {
        let from_rel = edge.from_rel();
        let to_rel = edge.to_rel();

        let mut from_labels: Vec<&str> = pairs
            .iter()
            .filter(|(t, _, b)| *t == from_rel && b == edge.label())
            .map(|(_, a, _)| a.as_str())
            .collect();
        let mut to_labels: Vec<&str> = pairs
            .iter()
            .filter(|(t, a, _)| *t == to_rel && a == edge.label())
            .map(|(_, _, b)| b.as_str())
            .collect();
        assert!(
            !from_labels.is_empty() && !to_labels.is_empty(),
            "{}: no rel tables found in the DDL at all",
            edge.label()
        );

        from_labels.sort_unstable();
        to_labels.sort_unstable();
        let mut allowed_from: Vec<&str> = edge.allowed_from_labels().to_vec();
        let mut allowed_to: Vec<&str> = edge.allowed_to_labels().to_vec();
        allowed_from.sort_unstable();
        allowed_to.sort_unstable();

        assert_eq!(
            allowed_from,
            from_labels,
            "FROM_{} allowlist drifted from the DDL",
            edge.label()
        );
        assert_eq!(
            allowed_to,
            to_labels,
            "TO_{} allowlist drifted from the DDL",
            edge.label()
        );
    }
}

#[test]
fn every_edge_label_round_trips_through_from_label() {
    for edge in EdgeType::ALL {
        assert_eq!(EdgeType::from_label(edge.label()), Some(edge));
    }
    assert_eq!(EdgeType::from_label("SELF_TRAJECTORY"), None);
    assert_eq!(EdgeType::from_label("TIME_HIERARCHY"), None);
}
