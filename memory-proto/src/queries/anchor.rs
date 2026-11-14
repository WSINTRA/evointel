//! Anchor choice — what the sermon is about tonight.
//!
//! The anchor is a concept the run's own notes touch, ranked by meaning
//! (similarity to the run's center), support (how many of the run's
//! notes point at it), recency (when it last appeared), and novelty (a
//! penalty for concepts used as anchors in recent runs). Repetition is
//! the failure this module exists to prevent: a fixed query word or a
//! nearest-ever lookup made every night the same night.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use lbug::{Connection, Value};

use crate::db::DbResult;
use crate::mutations::NodeRef;
use crate::nodes::{Entity, Note};

use super::fetch::get_entity;
use super::traversal::{connected_notes, neighbors};

/// The reflection's chosen anchor, with the score that chose it.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorChoice {
    pub entity: Entity,
    pub score: f32,
    pub support: usize,
}

/// How many recent runs' anchors are remembered for the novelty penalty.
const RECENT_ANCHOR_SPAN: usize = 10;

/// The cosine-distance ceiling for the fallback search (distance, not
/// similarity: lower is closer). Far concepts are not anchors.
const MAX_FALLBACK_DISTANCE: f32 = 0.65;

fn node_id(node: &lbug::NodeVal) -> Option<String> {
    node.get_properties()
        .iter()
        .find(|(k, _)| k == "id")
        .and_then(|(_, v)| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

/// The run's anchor memory, most recent first.
pub fn recent_anchor_ids(conn: &Connection, limit: usize) -> DbResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "MATCH (r:DreamRun) WHERE r.anchor_id IS NOT NULL \
         RETURN r.anchor_id, r.started_at;",
    )?;
    let result = conn.execute(&mut stmt, vec![])?;
    let mut rows: Vec<(DateTime<Utc>, String)> = vec![];
    for row in result {
        let [Value::String(id), Value::Timestamp(ts)] = row.as_slice() else {
            continue;
        };
        let at = super::fetch::time_to_chrono(ts);
        rows.push((at, id.clone()));
    }
    rows.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    Ok(rows.into_iter().take(limit).map(|(_, id)| id).collect())
}

/// The novelty penalty for an anchor used recently: the immediately
/// previous anchor is expensive, the last few are noticeable, older
/// history is a nudge.
fn novelty_penalty(id: &str, recent: &[String]) -> f32 {
    match recent.iter().position(|r| r == id) {
        Some(0) => 0.75,
        Some(rank) if rank < 3 => 0.35,
        Some(_) => 0.10,
        None => 0.0,
    }
}

/// Chooses the run's anchor from the entities its notes touch.
///
/// `fallback_query` is the run's embedding center (the daemon computes
/// it from the queue); it is used only when the judged edges yielded no
/// entity candidate, so a concept with stale or missing edges can never
/// win over the material the run actually wrote.
pub fn choose_anchor(
    conn: &Connection,
    queue: &[Note],
    fallback_query: &[f32],
    now: DateTime<Utc>,
) -> DbResult<Option<AnchorChoice>> {
    // Candidates: entities this run's notes touch, with support counts.
    let mut support: HashMap<String, usize> = HashMap::new();
    for note in queue {
        let node = NodeRef::new(note.id.clone(), "Note");
        for (neighbor, _) in neighbors(conn, &node, 1)? {
            if neighbor.get_label_name().as_str() != "Entity" {
                continue;
            }
            if let Some(id) = node_id(&neighbor) {
                *support.entry(id).or_insert(0) += 1;
            }
        }
    }
    let recent = recent_anchor_ids(conn, RECENT_ANCHOR_SPAN)?;
    #[allow(clippy::cast_precision_loss)] // support counts stay tiny
    let max_support = support.values().copied().max().unwrap_or(1) as f32;

    let mut best: Option<AnchorChoice> = None;
    for (id, count) in &support {
        let Some(entity) = get_entity(conn, id)? else {
            continue;
        };
        let sim = entity
            .label_embedding
            .as_ref()
            .map_or(0.0, |emb| cosine(fallback_query, emb));
        #[allow(clippy::cast_precision_loss)] // day counts stay tiny
        let age_days = (now - entity.learned_at).num_seconds().max(0) as f32 / 86_400.0;
        let recency = 1.0 / (1.0 + age_days);
        #[allow(clippy::cast_precision_loss)] // support counts stay tiny
        let support_norm = *count as f32 / max_support;
        let score = 0.45 * (sim + 1.0) / 2.0 + 0.30 * recency + 0.35 * support_norm
            - novelty_penalty(id, &recent);
        let candidate = AnchorChoice {
            entity,
            score,
            support: *count,
        };
        if best.as_ref().is_none_or(|b| {
            candidate.score.total_cmp(&b.score).is_gt()
                || (candidate.score.total_cmp(&b.score).is_eq()
                    && candidate.entity.id < b.entity.id)
        }) {
            best = Some(candidate);
        }
    }
    if best.is_some() {
        return Ok(best);
    }

    // Fallback: the nearest entity to the run's center, but only a
    // genuinely close one, only an entity with live citations, and with
    // recent anchors deprioritized.
    if fallback_query.iter().all(|x| *x == 0.0) {
        return Ok(None);
    }
    let hits = crate::vector::semantic_search(conn, fallback_query, 20, "Entity")?;
    let mut ranked: Vec<(f32, f32, Entity)> = vec![];
    for hit in hits {
        if hit.score > MAX_FALLBACK_DISTANCE {
            continue;
        }
        let Some(entity) = get_entity(conn, &hit.id)? else {
            continue;
        };
        let citations = connected_notes(conn, &hit.id)?;
        if citations.is_empty() {
            continue;
        }
        ranked.push((novelty_penalty(&hit.id, &recent), hit.score, entity));
    }
    ranked.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then_with(|| a.1.total_cmp(&b.1))
            .then_with(|| a.2.id.cmp(&b.2.id))
    });
    Ok(ranked
        .into_iter()
        .next()
        .map(|(_, _, entity)| AnchorChoice {
            entity,
            score: 0.0,
            support: 0,
        }))
}
