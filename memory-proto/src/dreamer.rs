//! The Dreamer — Wunicorm's sleep.
//!
//! One run of the overnight pass, end to end: consume the queue, judge
//! each note (the LLM seam — mocked with [`FixedJudge`] until the
//! `OpenAPI` layer exists), write nodes and four-pillar edges idempotently
//! with endpoint validation, observe the delta against a pre-dream
//! snapshot, reflect (search → citations → drift → events), and speak
//! the morning's message into the outbox. Every step is observed; the
//! [`DreamReport`] is the run's truth.
//!
//! The dreamer is the graph's *only writer*. It never delivers anything
//! — the outbox artifact is the handoff to the (future) email /
//! notification service layer.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use lbug::{Connection, Value};

use crate::db::DbResult;
use crate::edges::{Contains, HasProperty, LeadsTo, Similar};
use crate::message::{Citation, Insight, Message, MessageKind, shelf_check, write_message};
use crate::mutations::{
    NodeRef, insert_entity, insert_event, insert_fact, insert_memory, insert_note, insert_time,
    link_contains, link_has_property, link_leads_to, link_similar, wire_time_hierarchy,
};
use crate::nodes::{Entity, Event, Fact, Memory, Note, Time};
use crate::observability::{DreamReport, append_dream_report};
use crate::queries::{
    concept_drift, connected_notes, events_in_range, get_entity, graph_snapshot,
    structural_similarity,
};
use crate::vector::semantic_search;

/// An edge the judgment asserts: endpoints plus the edge's own
/// properties.
pub type Triple<P> = (NodeRef, NodeRef, P);

/// What the judge concluded about one note: the nodes to write, the
/// time anchoring, and the four-pillar edges between them.
///
/// This struct is the LLM layer's output contract — the live judge
/// fills it in; the write path applies it unchanged.
#[derive(Debug, Clone)]
pub struct DreamJudgment {
    /// The note this judgment came from (provenance).
    pub note_id: String,
    pub entities: Vec<Entity>,
    pub facts: Vec<Fact>,
    pub events: Vec<Event>,
    pub memories: Vec<Memory>,
    pub times: Vec<Time>,
    /// `(parent time id, child time id)` pairs to wire into the
    /// hierarchy.
    pub hierarchy: Vec<(String, String)>,
    pub contains: Vec<Triple<Contains>>,
    pub similar: Vec<Triple<Similar>>,
    pub has_property: Vec<Triple<HasProperty>>,
    pub leads_to: Vec<Triple<LeadsTo>>,
}

/// The dreamer's brain: turns a note into structured judgments.
///
/// The connection is handed in because a real judge wants graph context
/// (which entities already exist — create vs. link is a judgment call).
pub trait Judge {
    fn judge(&self, conn: &Connection, note: &Note) -> Result<DreamJudgment, String>;
}

/// Deterministic judge for tests and demos: pre-baked judgments keyed
/// by note id. Errors on notes it has no judgment for — the dreamer
/// records that as a run error.
#[derive(Debug)]
pub struct FixedJudge {
    by_note: HashMap<String, DreamJudgment>,
}

impl FixedJudge {
    pub fn new(judgments: Vec<DreamJudgment>) -> Self {
        Self {
            by_note: judgments
                .into_iter()
                .map(|j| (j.note_id.clone(), j))
                .collect(),
        }
    }
}

impl Judge for FixedJudge {
    fn judge(&self, _conn: &Connection, note: &Note) -> Result<DreamJudgment, String> {
        self.by_note
            .get(&note.id)
            .cloned()
            .ok_or_else(|| format!("no judgment baked for note {}", note.id))
    }
}

/// Two time windows the reflection compares for drift: `(from, to)`,
/// each window a `(start, end)` pair.
pub type TimeWindows = (
    (DateTime<Utc>, DateTime<Utc>),
    (DateTime<Utc>, DateTime<Utc>),
);

/// The inputs to one dream run.
#[derive(Debug, Clone)]
pub struct DreamSpec {
    pub run_id: String,
    /// The queue: new/changed notes since the last run. The sorter's
    /// output; a `Vec` stands in until the daemon exists.
    pub queue: Vec<Note>,
    /// The reflection searches for the concept nearest this embedding
    /// and grounds the message in it.
    pub anchor_embedding: Vec<f32>,
    /// The two windows the reflection compares for drift. `None` skips
    /// the drift (and events) insight.
    pub drift_windows: Option<TimeWindows>,
    /// Where the message artifact lands.
    pub outbox: PathBuf,
    /// The append-only dream ledger.
    pub dream_log: PathBuf,
}

/// Runs one full dream cycle and returns its observation.
///
/// Failures are *recorded*, not thrown: a dream that hits problems
/// still produces a report, because the report is the observation the
/// nightmare detector and the ledger act on.
pub fn run_dream(conn: &Connection, judge: &dyn Judge, spec: &DreamSpec) -> DreamReport {
    let started_at = Utc::now();
    let mut errors: Vec<String> = vec![];
    let mut nodes_written = 0usize;

    // ── Baseline: the world before the dream ──
    let before = match graph_snapshot(conn) {
        Ok(s) => Some(s),
        Err(e) => {
            errors.push(format!("pre-dream snapshot failed: {e}"));
            None
        }
    };

    // ── Judge & write: every note through the LLM seam, then MERGE ──
    for note in &spec.queue {
        let judgment = match judge.judge(conn, note) {
            Ok(j) => j,
            Err(e) => {
                errors.push(format!("judge failed for note {}: {e}", note.id));
                continue;
            }
        };
        if let Err(e) = insert_note(conn, note) {
            errors.push(format!("note {} write failed: {e}", note.id));
            continue;
        }
        nodes_written += 1;
        apply_judgment(conn, &judgment, &mut nodes_written, &mut errors);
    }

    // ── Observe: the delta, honestly counted ──
    let after = match graph_snapshot(conn) {
        Ok(s) => Some(s),
        Err(e) => {
            errors.push(format!("post-dream snapshot failed: {e}"));
            None
        }
    };
    let (nodes_added, edges_added, similarity_to_prev) = match (&before, &after) {
        (Some(b), Some(a)) => (
            a.nodes.len().saturating_sub(b.nodes.len()),
            (a.edges.len() + a.time_rels.len()).saturating_sub(b.edges.len() + b.time_rels.len()),
            Some(structural_similarity(b, a)),
        ),
        _ => (0, 0, None),
    };

    // ── Reflect & speak: read the model, write the sermon material ──
    // Quiet days say nothing on purpose — silence is the correct output
    // when there is nothing new.
    let (message_written, shelf_ok) = if spec.queue.is_empty() {
        (false, true)
    } else {
        reflect_and_speak(conn, spec, started_at, &mut errors)
    };

    let mut report = DreamReport {
        run_id: spec.run_id.clone(),
        started_at,
        finished_at: Some(Utc::now()),
        notes_in: spec.queue.len(),
        nodes_written,
        nodes_added,
        edges_added,
        errors,
        message_written,
        shelf_ok,
        similarity_to_prev,
    };

    // ── The ledger: this run joins the permanent record ──
    if let Err(e) = append_dream_report(&spec.dream_log, &report) {
        report
            .errors
            .push(format!("dream ledger append failed: {e}"));
    }

    report
}

/// Applies one judgment's writes. Every write is idempotent; every
/// failure is recorded and the dream continues.
fn apply_judgment(
    conn: &Connection,
    judgment: &DreamJudgment,
    nodes_written: &mut usize,
    errors: &mut Vec<String>,
) {
    let mut write_node =
        |res: DbResult<()>, what: &str, id: &str, errors: &mut Vec<String>| match res {
            Ok(()) => *nodes_written += 1,
            Err(e) => errors.push(format!("{what} {id} write failed: {e}")),
        };

    for e in &judgment.entities {
        write_node(insert_entity(conn, e), "entity", &e.id, errors);
    }
    for f in &judgment.facts {
        write_node(insert_fact(conn, f), "fact", &f.id, errors);
    }
    for ev in &judgment.events {
        write_node(insert_event(conn, ev), "event", &ev.id, errors);
    }
    for m in &judgment.memories {
        write_node(insert_memory(conn, m), "memory", &m.id, errors);
    }
    for t in &judgment.times {
        write_node(insert_time(conn, t), "time", &t.id, errors);
    }

    for (parent, child) in &judgment.hierarchy {
        let endpoints = (
            NodeRef::new(parent.clone(), "Time"),
            NodeRef::new(child.clone(), "Time"),
        );
        link_edge(
            conn,
            "TIME_HIERARCHY",
            &endpoints.0,
            &endpoints.1,
            || wire_time_hierarchy(conn, parent, child).map(|()| String::new()),
            nodes_written,
            errors,
        );
    }

    for (from, to, props) in &judgment.contains {
        link_edge(
            conn,
            "Contains",
            from,
            to,
            || link_contains(conn, from, to, props),
            nodes_written,
            errors,
        );
    }
    for (from, to, props) in &judgment.similar {
        link_edge(
            conn,
            "Similar",
            from,
            to,
            || link_similar(conn, from, to, props),
            nodes_written,
            errors,
        );
    }
    for (from, to, props) in &judgment.has_property {
        link_edge(
            conn,
            "HasProperty",
            from,
            to,
            || link_has_property(conn, from, to, props),
            nodes_written,
            errors,
        );
    }
    for (from, to, props) in &judgment.leads_to {
        link_edge(
            conn,
            "LeadsTo",
            from,
            to,
            || link_leads_to(conn, from, to, props),
            nodes_written,
            errors,
        );
    }
}

/// Links one edge, validating both endpoints first — a dangling
/// reference is a recorded fault, never a silent no-op.
fn link_edge(
    conn: &Connection,
    edge_name: &str,
    from: &NodeRef,
    to: &NodeRef,
    link: impl FnOnce() -> DbResult<String>,
    nodes_written: &mut usize,
    errors: &mut Vec<String>,
) {
    for node in [from, to] {
        match node_exists(conn, node) {
            Ok(true) => {}
            Ok(false) => {
                errors.push(format!(
                    "{edge_name} edge skipped — dangling reference: {}/{}",
                    node.label, node.id
                ));
                return;
            }
            Err(e) => {
                errors.push(format!(
                    "{edge_name} edge skipped — endpoint check failed: {e}"
                ));
                return;
            }
        }
    }
    match link() {
        Ok(_) => *nodes_written += 1,
        Err(e) => errors.push(format!("{edge_name} link failed: {e}")),
    }
}

fn node_exists(conn: &Connection, node: &NodeRef) -> Result<bool, String> {
    let cypher = format!("MATCH (n:{} {{id: $id}}) RETURN count(n);", node.label);
    let mut stmt = conn.prepare(&cypher).map_err(|e| e.to_string())?;
    let result = conn
        .execute(&mut stmt, vec![("id", Value::String(node.id.clone()))])
        .map_err(|e| e.to_string())?;
    for row in result {
        if let Value::Int64(n) = row[0] {
            return Ok(n > 0);
        }
    }
    Ok(false)
}

/// The reflection stage: read the model the dream just updated, and
/// write the sermon material into the outbox.
///
/// Returns `(message_written, shelf_ok)`.
fn reflect_and_speak(
    conn: &Connection,
    spec: &DreamSpec,
    now: DateTime<Utc>,
    errors: &mut Vec<String>,
) -> (bool, bool) {
    // Find the anchor concept by meaning.
    let anchor = match semantic_search(conn, &spec.anchor_embedding, 1, "Entity") {
        Ok(hits) => hits.into_iter().next(),
        Err(e) => {
            errors.push(format!("reflection: anchor search failed: {e}"));
            None
        }
    };
    let Some(anchor) = anchor else {
        errors.push("reflection: no anchor concept found for the sermon".to_string());
        return (false, false);
    };
    let anchor_ref = NodeRef::new(anchor.id.clone(), "Entity");
    let anchor_name = get_entity(conn, &anchor.id)
        .ok()
        .flatten()
        .map_or_else(|| anchor.id.clone(), |e| e.label_resolved);

    // Citations: the user's actual notes the anchor connects to.
    let citations: Vec<Citation> = match connected_notes(conn, &anchor.id) {
        Ok(notes) => notes
            .iter()
            .map(|n| Citation {
                note_id: n.id.clone(),
                snippet: snippet(&n.content),
            })
            .collect(),
        Err(e) => {
            errors.push(format!("reflection: citation read failed: {e}"));
            vec![]
        }
    };

    // Insights: what the anchor is connected to, drift across the
    // windows, and the events of the period.
    let mut insights: Vec<Insight> = vec![];

    // The baseline insight — a dream with nothing to compare still has
    // a "what's here now" to report.
    if !citations.is_empty() {
        insights.push(Insight {
            summary: format!("{} notes now connect to '{anchor_name}'.", citations.len()),
            concept: Some(anchor_ref.clone()),
            drift: None,
        });
    }

    if let Some((from, to)) = &spec.drift_windows {
        match concept_drift(conn, &anchor_ref, *from, *to) {
            Ok(Some(d)) => insights.push(Insight {
                summary: format!(
                    "'{anchor_name}' is shifting: displacement {:.2}, +{} / -{} connections.",
                    d.displacement, d.new_connections, d.dropped_connections
                ),
                concept: Some(anchor_ref.clone()),
                drift: Some(d),
            }),
            // Unmeasurable windows are "can't tell", not an error.
            Ok(None) => {}
            Err(e) => errors.push(format!("reflection: drift read failed: {e}")),
        }
        match events_in_range(conn, from.0, to.1) {
            Ok(events) if !events.is_empty() => insights.push(Insight {
                summary: format!(
                    "{} events recorded around '{anchor_name}' in this period.",
                    events.len()
                ),
                concept: None,
                drift: None,
            }),
            Ok(_) => {}
            Err(e) => errors.push(format!("reflection: events read failed: {e}")),
        }
    }

    // Assemble, write, prove.
    let body = render_body(&anchor_name, &insights, &citations);
    let msg = Message {
        id: format!("msg-{}", spec.run_id),
        run_id: spec.run_id.clone(),
        kind: MessageKind::Sermon,
        generated_at: now,
        insights,
        citations,
        body,
    };
    match write_message(&msg, &spec.outbox) {
        Ok(path) => match shelf_check(&path, &spec.run_id) {
            Ok(()) => (true, true),
            Err(reason) => {
                errors.push(format!("shelf check failed: {reason}"));
                (true, false)
            }
        },
        Err(e) => {
            errors.push(format!("message write failed: {e}"));
            (false, false)
        }
    }
}

/// Renders the sermon material: insights first, then the notes that
/// ground them. Mechanical on purpose — prose craft belongs to a future
/// renderer, not to the dreamer.
fn render_body(anchor: &str, insights: &[Insight], citations: &[Citation]) -> String {
    let mut body = format!("Morning — a dream pass over your notes.\n\nAround '{anchor}':\n");
    for i in insights {
        let _ = writeln!(body, "- {}", i.summary);
    }
    if !citations.is_empty() {
        let _ = writeln!(body, "\nGrounded in your notes:");
        for c in citations {
            let _ = writeln!(body, "- [{}] {}", c.note_id, c.snippet);
        }
    }
    body
}

/// First ~80 characters of a note — enough to recognize it, not enough
/// to bloat the artifact.
fn snippet(content: &str) -> String {
    content.chars().take(80).collect()
}
