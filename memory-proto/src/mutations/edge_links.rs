use lbug::{Connection, Value};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;
use crate::edges::{
    After, BecauseOf, Before, Causes, Contains, During, EdgeType, HasProperty, LeadsTo, Prevents,
    Similar, ValidFrom, ValidTo,
};

use super::{chrono_to_time, chrono_to_time_opt};

/// A typed reference to a graph node. The `label` is the Cypher table name
/// (e.g. "Note", "Entity") used to build `MATCH (a:Label {id: $id})` clauses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRef {
    pub id: String,
    pub label: String,
}

impl NodeRef {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// Executes the 3-step edge-node linking pattern:
/// 1. MERGE the edge node with its properties
/// 2. MERGE the FROM rel (source → edge)
/// 3. MERGE the TO rel (edge → target)
///
/// Returns the edge node id.
fn create_edge_link(
    conn: &Connection,
    edge_type: EdgeType,
    from: &NodeRef,
    to: &NodeRef,
    edge_id: &str,
    set_clauses: &str,
    params: Vec<(&str, Value)>,
) -> DbResult<String> {
    let label = edge_type.label();
    let from_rel = edge_type.from_rel();
    let to_rel = edge_type.to_rel();

    // Step 1: MERGE the edge node
    let step1 = format!("MERGE (e:{label} {{id: $id}}) SET {set_clauses};");
    let mut stmt1 = conn.prepare(&step1)?;
    let mut all_params = vec![("id", Value::String(edge_id.to_string()))];
    all_params.extend(params);
    conn.execute(&mut stmt1, all_params)?;

    // Step 2: Wire source → edge
    let step2 = format!(
        "MATCH (a:{from_label} {{id: $from_id}}), (e:{label} {{id: $id}}) \
         MERGE (a)-[:{from_rel} {{role: 'source'}}]->(e);",
        from_label = from.label,
        label = label,
        from_rel = from_rel,
    );
    let mut stmt2 = conn.prepare(&step2)?;
    conn.execute(
        &mut stmt2,
        vec![
            ("from_id", Value::String(from.id.clone())),
            ("id", Value::String(edge_id.to_string())),
        ],
    )?;

    // Step 3: Wire edge → target
    let step3 = format!(
        "MATCH (e:{label} {{id: $id}}), (b:{to_label} {{id: $to_id}}) \
         MERGE (e)-[:{to_rel} {{role: 'target'}}]->(b);",
        label = label,
        to_label = to.label,
        to_rel = to_rel,
    );
    let mut stmt3 = conn.prepare(&step3)?;
    conn.execute(
        &mut stmt3,
        vec![
            ("id", Value::String(edge_id.to_string())),
            ("to_id", Value::String(to.id.clone())),
        ],
    )?;

    Ok(edge_id.to_string())
}

// ─── Structural edges ───────────────────────────────────────────────

pub fn link_contains(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &Contains,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::Contains,
        from,
        to,
        &props.id,
        "e.containment_types = $ct, \
         e.weight = $w, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("ct", Value::String(props.containment_types.clone())),
            ("w", Value::Double(props.weight)),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

pub fn link_similar(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &Similar,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::Similar,
        from,
        to,
        &props.id,
        "e.similarity = $sim, \
         e.sim_context = $sim_context, \
         e.sim_method = $sim_method, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("sim", Value::Double(props.similarity)),
            ("sim_context", Value::String(props.sim_context.clone())),
            ("sim_method", Value::String(props.sim_method.clone())),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

pub fn link_has_property(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &HasProperty,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::HasProperty,
        from,
        to,
        &props.id,
        "e.property_name = $pname, \
         e.property_value = $pval, \
         e.prop_context = $pctx, \
         e.certainty = $cert, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("pname", Value::String(props.property_name.clone())),
            ("pval", Value::String(props.property_value.clone())),
            ("pctx", Value::String(props.prop_context.clone())),
            ("cert", Value::Double(props.certainty)),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

// ─── Causal edges ───────────────────────────────────────────────────

pub fn link_leads_to(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &LeadsTo,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::LeadsTo,
        from,
        to,
        &props.id,
        "e.probability = $prob, \
         e.strength = $str, \
         e.mechanism = $mech, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("prob", Value::Double(props.probability)),
            ("str", Value::Double(props.strength)),
            ("mech", Value::String(props.mechanism.clone())),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

pub fn link_causes(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &Causes,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::Causes,
        from,
        to,
        &props.id,
        "e.probability = $prob, \
         e.strength = $str, \
         e.mechanism = $mech, \
         e.directness = $direct, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("prob", Value::Double(props.probability)),
            ("str", Value::Double(props.strength)),
            ("mech", Value::String(props.mechanism.clone())),
            ("direct", Value::String(props.directness.clone())),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

pub fn link_prevents(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &Prevents,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::Prevents,
        from,
        to,
        &props.id,
        "e.probability = $prob, \
         e.strength = $str, \
         e.mechanism = $mech, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("prob", Value::Double(props.probability)),
            ("str", Value::Double(props.strength)),
            ("mech", Value::String(props.mechanism.clone())),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

pub fn link_because_of(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &BecauseOf,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::BecauseOf,
        from,
        to,
        &props.id,
        "e.probability = $prob, \
         e.strength = $str, \
         e.explanation = $expl, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("prob", Value::Double(props.probability)),
            ("str", Value::Double(props.strength)),
            ("expl", Value::String(props.explanation.clone())),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

// ─── Temporal edges ─────────────────────────────────────────────────

pub fn link_before(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &Before,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::Before,
        from,
        to,
        &props.id,
        "e.gap_duration = $gap, \
         e.confidence = $conf, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("gap", Value::String(props.gap_duration.clone())),
            ("conf", Value::Double(props.confidence)),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

pub fn link_after(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &After,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::After,
        from,
        to,
        &props.id,
        "e.gap_duration = $gap, \
         e.confidence = $conf, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("gap", Value::String(props.gap_duration.clone())),
            ("conf", Value::Double(props.confidence)),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

pub fn link_during(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &During,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::During,
        from,
        to,
        &props.id,
        "e.overlap_type = $overlap, \
         e.confidence = $conf, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("overlap", Value::String(props.overlap_type.clone())),
            ("conf", Value::Double(props.confidence)),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

pub fn link_valid_from(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &ValidFrom,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::ValidFrom,
        from,
        to,
        &props.id,
        "e.precision = $prec, \
         e.confidence = $conf, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("prec", Value::String(props.precision.clone())),
            ("conf", Value::Double(props.confidence)),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}

pub fn link_valid_to(
    conn: &Connection,
    from: &NodeRef,
    to: &NodeRef,
    props: &ValidTo,
) -> DbResult<String> {
    create_edge_link(
        conn,
        EdgeType::ValidTo,
        from,
        to,
        &props.id,
        "e.precision = $prec, \
         e.confidence = $conf, \
         e.termination = $term, \
         e.label = $label, \
         e.label_resolved = $label_resolved, \
         e.learned_at = $learned_at, \
         e.expire_at = $expire_at, \
         e.created_at = $created_at, \
         e.updated_at = $updated_at, \
         e.layer = $layer, \
         e.context = $context",
        vec![
            ("prec", Value::String(props.precision.clone())),
            ("conf", Value::Double(props.confidence)),
            ("term", Value::String(props.termination.clone())),
            ("label", Value::String(props.label.clone())),
            (
                "label_resolved",
                Value::String(props.label_resolved.clone()),
            ),
            (
                "learned_at",
                Value::Timestamp(chrono_to_time(&props.learned_at)),
            ),
            ("expire_at", chrono_to_time_opt(props.expire_at.as_ref())),
            (
                "created_at",
                Value::Timestamp(chrono_to_time(&props.created_at)),
            ),
            (
                "updated_at",
                Value::Timestamp(chrono_to_time(&props.updated_at)),
            ),
            ("layer", Value::Int16(props.layer)),
            ("context", Value::String(props.context.clone())),
        ],
    )
}
