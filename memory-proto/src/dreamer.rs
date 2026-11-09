//! The Dreamer — Wunicorm's sleep.
//!
//! One run of the overnight pass, end to end: consume the queue, judge
//! each note (the extraction seam — [`Judge`], implemented live by the
//! `llm` layer), write nodes and four-pillar edges idempotently with
//! endpoint validation, observe the delta against a pre-dream
//! snapshot, reflect (search → citations → drift → events), and speak
//! the morning's message into the outbox through the voice seam
//! ([`SermonWriter`] — mechanical floor, LLM craft above it). Every
//! step is observed; the [`DreamReport`] is the run's truth.
//!
//! The dreamer is the graph's *only writer*. Delivery to the user is a
//! seam ([`MessageDelivery`]): the outbox artifact stays the source of
//! truth, and the daemon's delivery impl carries it to Mac Notes —
//! read-back verified, faults recorded in the run's ledger.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use lbug::{Connection, Value};

use crate::axes::{Axis, Function, RIGIDITY_SPAN};
use crate::db::DbResult;
use crate::edges::{Contains, EdgeType, Grounds, HasProperty, LeadsTo, Similar};
use crate::message::{Citation, Insight, Message, MessageKind, shelf_check, write_message};
use crate::mutations::{
    NodeRef, insert_dream_run, insert_entity, insert_event, insert_fact, insert_memory,
    insert_note, insert_self_state, insert_time, link_contains, link_grounds, link_has_property,
    link_leads_to, link_similar, set_note_coordinates, wire_self_trajectory, wire_time_hierarchy,
};
use crate::nodes::{DreamRun, Entity, Event, Fact, Memory, Note, SelfState, Time};
use crate::observability::{DreamReport, SelfPassReport, append_dream_report};
use crate::queries::{
    active_axis_frame, concept_drift, connected_notes, events_in_range, evolve_wunicorm,
    function_drift, get_entity, graph_snapshot, rigidity, structural_similarity, user_position,
    wunicorm_observed,
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

/// The sermon's voice: turns the reflection's grounded material —
/// anchor, insights, citations, the self reading — into the body the
/// reader meets.
///
/// The facts are always the dreamer's own; the writer only speaks
/// them. [`MechanicalSermon`] is the floor that cannot fail; the living
/// voice is the LLM writer in `llm::LlmSermonWriter`. A writer that
/// errors falls back to the floor, recorded, and the dream continues.
pub trait SermonWriter {
    fn compose(
        &self,
        anchor: &str,
        insights: &[Insight],
        citations: &[Citation],
        self_reading: Option<&SelfReading>,
    ) -> Result<String, String>;
}

/// The self pass rendered for the voice: movement language only.
/// Numbers belong to the ledger — and the strip happens here, at
/// construction, not by prompt rule. No field ever attributes the
/// user's movement to the system: observed, never caused.
#[derive(Debug, Clone, PartialEq)]
pub struct SelfReading {
    /// "your attention moved toward thinking" — from the drift between
    /// the two latest user states. `None` until a second window exists.
    pub user_movement: Option<String>,
    /// "the reading leaned thinking-ward" — the compensation, when it
    /// engaged. `None` when it stayed silent.
    pub wunicorm_lean: Option<String>,
    /// "the feeling stance is holding very still" — rigidity above the
    /// threshold, named by the stance's direction. `None` otherwise.
    pub tension: Option<String>,
}

impl SelfReading {
    /// The floor's one line — everything the reading dares to say.
    pub fn summary(&self) -> String {
        let parts: Vec<&str> = [
            self.user_movement.as_deref(),
            self.wunicorm_lean.as_deref(),
            self.tension.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();
        parts.join("; ")
    }
}

/// Renders the self pass for the voice — movement language, no numbers,
/// no claimed effects.
fn render_self_reading(pass: &SelfPassReport) -> SelfReading {
    // Movement: the drift between the user's two latest states. Zero
    // drift is honest stillness; a first window has nothing to compare
    // and stays silent.
    let user_movement = pass.drift.map(|(j, p)| {
        let mut phrases: Vec<&str> = vec![];
        if j.abs() > f32::EPSILON {
            phrases.push(if j > 0.0 {
                "toward thinking"
            } else {
                "toward feeling"
            });
        }
        if p.abs() > f32::EPSILON {
            phrases.push(if p > 0.0 {
                "toward sensing"
            } else {
                "toward intuiting"
            });
        }
        if phrases.is_empty() {
            "your attention held still on both axes".to_string()
        } else {
            format!("your attention moved {}", phrases.join(" and "))
        }
    });

    // The lean: compensation that engaged, named by its direction. The
    // words only name the lean — never the cause, never the credit.
    let wunicorm_lean = match pass.wunicorm_comp {
        (j, p) if j.abs() > f32::EPSILON || p.abs() > f32::EPSILON => {
            let mut phrases: Vec<&str> = vec![];
            if j.abs() > f32::EPSILON {
                phrases.push(if j > 0.0 {
                    "thinking-ward"
                } else {
                    "feeling-ward"
                });
            }
            if p.abs() > f32::EPSILON {
                phrases.push(if p > 0.0 {
                    "sensing-ward"
                } else {
                    "intuiting-ward"
                });
            }
            Some(format!("the reading leaned {}", phrases.join(" and ")))
        }
        _ => None,
    };

    // Tension: a rigid stance, named by its direction.
    let tension = pass
        .user_position
        .filter(|_| {
            pass.rigidity.0 > crate::axes::RIGIDITY_THRESHOLD
                || pass.rigidity.1 > crate::axes::RIGIDITY_THRESHOLD
        })
        .map(|(j, p)| {
            if pass.rigidity.0 > crate::axes::RIGIDITY_THRESHOLD {
                let stance = if j <= 0.0 { "feeling" } else { "thinking" };
                format!("the {stance} stance is holding very still")
            } else {
                let stance = if p <= 0.0 { "intuiting" } else { "sensing" };
                format!("the {stance} stance is holding very still")
            }
        });

    SelfReading {
        user_movement,
        wunicorm_lean,
        tension,
    }
}

/// The guaranteed voice: mechanical, grounded, a log by design. It
/// exists so the sermon always ships — craft is a layer above, not a
/// dependency.
pub struct MechanicalSermon;

impl SermonWriter for MechanicalSermon {
    fn compose(
        &self,
        anchor: &str,
        insights: &[Insight],
        _citations: &[Citation],
        self_reading: Option<&SelfReading>,
    ) -> Result<String, String> {
        let mut body = format!("Morning — a dream pass over your notes.\n\nAround '{anchor}':\n");
        for i in insights {
            let _ = writeln!(body, "- {}", i.summary);
        }
        // The floor speaks what the dreamer measured, even when the
        // living voice fails.
        if let Some(reading) = self_reading
            && !reading.summary().is_empty()
        {
            let _ = writeln!(body, "- the self: {}", reading.summary());
        }
        Ok(body)
    }
}

/// The outcome of one delivery attempt: the note's truth, the
/// banner's truth, and the faults either produced. Nothing silent —
/// every failure names itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeliveryOutcome {
    pub note_delivered: bool,
    pub notification_sent: bool,
    pub errors: Vec<String>,
}

/// The delivery seam: carries a finished message to the user. The
/// daemon's implementation writes Mac Notes and the notification;
/// tests and the demo use `NoDelivery`. Delivery, like every other
/// write, is verified by read-back — a green return code never stands
/// in for a result.
pub trait MessageDelivery {
    fn deliver(&self, msg: &Message) -> DeliveryOutcome;
}

/// Delivery not wired (tests, demo): reports success without touching
/// the machine.
pub struct NoDelivery;

impl MessageDelivery for NoDelivery {
    fn deliver(&self, _msg: &Message) -> DeliveryOutcome {
        DeliveryOutcome {
            note_delivered: true,
            notification_sent: true,
            errors: vec![],
        }
    }
}

/// Deterministic judge for tests and demos: pre-baked judgments keyed
/// by note id. Errors on notes it has no judgment for — the dreamer
/// records that as a run error.#[derive(Debug)]
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
pub fn run_dream(
    conn: &Connection,
    judge: &dyn Judge,
    sermon: &dyn SermonWriter,
    delivery: &dyn MessageDelivery,
    spec: &DreamSpec,
) -> DreamReport {
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
    // The choice ledger tallies as the loop runs: what was considered,
    // what was kept (per function), what was judged and dropped whole.
    let mut considered: Vec<String> = vec![];
    let mut dropped: Vec<String> = vec![];
    let mut kept = [0i64; 4]; // [thinking, feeling, sensing, intuiting]
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
        considered.push(note.id.clone());
        if judgment.memories.is_empty() {
            dropped.push(note.id.clone());
        }
        for m in &judgment.memories {
            if let Some(tag) = m.function_tag.as_deref().and_then(Function::parse) {
                kept[kept_index(tag)] += 1;
            }
        }
        apply_judgment(
            conn,
            &judgment,
            &spec.run_id,
            &mut nodes_written,
            &mut errors,
        );
    }

    // ── The choice ledger: what this run kept and what it let go ──
    // One idempotent row per non-quiet run. The discarded is data —
    // Wunicorm's position is measured from these choices, and drops
    // dilute its confidence rather than pushing its position.
    if !considered.is_empty() {
        let now = Utc::now();
        let run = DreamRun {
            id: spec.run_id.clone(),
            considered: considered.clone(),
            dropped: dropped.clone(),
            kept_t: kept[0],
            kept_f: kept[1],
            kept_s: kept[2],
            kept_n: kept[3],
            // Per-run note counts stay far below 2^63.
            #[allow(clippy::cast_possible_wrap)]
            dropped_total: dropped.len() as i64,
            started_at,
            finished_at: now,
            created_at: now,
            updated_at: now,
            layer: 3,
            context: "dream".to_string(),
        };
        if let Err(e) = insert_dream_run(conn, &run) {
            errors.push(format!("dream run ledger write failed: {e}"));
        }
    }

    // ── Work the self: positions on the axes, the W update, and the
    // provenance that ties every claim to nodes. Runs before the
    // observe snapshot so the dream's writes (states, grounds) are
    // honestly counted in the delta. Quiet days don't work the self —
    // there is nothing to measure and no one to compensate.
    let self_pass = if spec.queue.is_empty() {
        None
    } else {
        work_the_self(conn, spec, &mut nodes_written, &mut errors)
    };

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
    let (message_written, shelf_ok, note_delivered) = if spec.queue.is_empty() {
        (false, true, false)
    } else {
        reflect_and_speak(conn, sermon, delivery, spec, started_at, self_pass, &mut errors)
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
        note_delivered,
        similarity_to_prev,
        self_pass,
    };

    // ── The ledger: this run joins the permanent record ──
    if let Err(e) = append_dream_report(&spec.dream_log, &report) {
        report
            .errors
            .push(format!("dream ledger append failed: {e}"));
    }

    report
}

/// The choice-ledger column index for a function:
/// `[thinking, feeling, sensing, intuiting]`.
fn kept_index(f: Function) -> usize {
    match f {
        Function::Thinking => 0,
        Function::Feeling => 1,
        Function::Sensing => 2,
        Function::Intuiting => 3,
    }
}

// ─── The self pass ───────────────────────────────────────────────────

/// Works the self: measures the run's notes onto the axes, writes U's
/// window state, evolves W (damped core + rigidity-gated compensation),
/// and grounds every positional claim in the nodes it came from.
///
/// The dream never dies for the self's sake — every failure is recorded
/// and the pass degrades: unseeded anchors, an unmeasurable window, a
/// missing ledger each skip their part, honestly named.
fn work_the_self(
    conn: &Connection,
    spec: &DreamSpec,
    nodes_written: &mut usize,
    errors: &mut Vec<String>,
) -> Option<SelfPassReport> {
    // The axes themselves. Unseeded anchors are a quiet skip, not a
    // fault: the self layer is opt-in — until anchors are seeded, a DB
    // dreams exactly as it did before. Failures *after* opting in are
    // errors.
    let frame = match active_axis_frame(conn) {
        Ok(Some(f)) => f,
        Ok(None) => return None,
        Err(e) => {
            errors.push(format!("self pass: anchor load failed: {e}"));
            return None;
        }
    };

    // The window being worked: the drift comparison's "to" window when
    // present, else the queue's own span.
    let queue_span = || {
        let first = spec.queue.iter().map(|n| n.created_at).min()?;
        let last = spec.queue.iter().map(|n| n.created_at).max()?;
        Some((first, last))
    };
    let window = match spec.drift_windows {
        Some((_, to)) => to,
        None => queue_span()?,
    };
    let now = Utc::now();

    // 1. Measure the run's notes onto the axes. A note without an
    // embedding stays unmeasured — no signal is not a center.
    let mut measured: Vec<String> = vec![];
    for note in &spec.queue {
        let Some(embedding) = note.content_embedding.as_ref() else {
            continue;
        };
        let (judgment, perception) = frame.coordinates(embedding);
        match set_note_coordinates(conn, &note.id, f64::from(judgment), f64::from(perception)) {
            Ok(()) => measured.push(note.id.clone()),
            Err(e) => errors.push(format!(
                "self pass: note {} coordinate write failed: {e}",
                note.id
            )),
        }
    }

    // 2. U's position for the window, and its state on the trajectory.
    // U is read, never steered — the decomposition fields stay zero on
    // user states; they are W's record.
    let user = match user_position(conn, window) {
        Ok(p) => p,
        Err(e) => {
            errors.push(format!("self pass: user position read failed: {e}"));
            None
        }
    };
    let user_state_id = format!("self-user-{}", window.0.timestamp());
    if let Some(position) = user {
        let state = SelfState {
            id: user_state_id.clone(),
            subject: "user".to_string(),
            window_start: window.0,
            window_end: window.1,
            judgment_coord: f64::from(position.0),
            perception_coord: f64::from(position.1),
            judgment_core: 0.0,
            perception_core: 0.0,
            judgment_comp: 0.0,
            perception_comp: 0.0,
            rigidity_judgment: 0.0,
            rigidity_perception: 0.0,
            #[allow(clippy::cast_possible_wrap)] // note counts stay tiny
            evidence_kept: measured.len() as i64,
            evidence_dropped: 0,
            learned_at: now,
            created_at: now,
            updated_at: now,
            layer: 4,
            context: spec.run_id.clone(),
        };
        match insert_self_state(conn, &state) {
            Ok(()) => *nodes_written += 1,
            Err(e) => errors.push(format!("self pass: user state write failed: {e}")),
        }
        wire_trajectory(
            conn,
            "user",
            window.0,
            &user_state_id,
            nodes_written,
            errors,
        );
        for note_id in &measured {
            ground_evidence(
                conn,
                &user_state_id,
                note_id,
                "Note",
                "window",
                &spec.run_id,
                now,
                nodes_written,
                errors,
            );
        }
    }

    // 3. W: observed from this run's ledger, evolved against the damped
    // core, compensated only against a rigid user. No user position →
    // no compensation (nothing to lean against), never an invented one.
    let observed = match wunicorm_observed(conn, &spec.run_id) {
        Ok(o) => o,
        Err(e) => {
            errors.push(format!("self pass: choice ledger read failed: {e}"));
            return None;
        }
    };
    let Some(observed) = observed else {
        errors.push("self pass skipped — no choice ledger this run".to_string());
        return None;
    };
    let prev_core = match wunicorm_prev_core(conn) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("self pass: W core read failed: {e}"));
            (0.0, 0.0)
        }
    };
    // Rigidity reads the trajectory including the state just written —
    // the user's stuckness now, not last month's.
    let rigidities = (
        rigidity(conn, "user", Axis::Judgment, RIGIDITY_SPAN).unwrap_or(0.0),
        rigidity(conn, "user", Axis::Perception, RIGIDITY_SPAN).unwrap_or(0.0),
    );
    let decomposition =
        evolve_wunicorm(prev_core, observed, user.unwrap_or((0.0, 0.0)), rigidities);

    let (kept_total, dropped_total) = match dream_run_evidence(conn, &spec.run_id) {
        Ok((k, d)) => (k, d),
        Err(e) => {
            errors.push(format!("self pass: ledger evidence read failed: {e}"));
            (0, 0)
        }
    };
    let w_state_id = format!("self-wunicorm-{}", window.0.timestamp());
    let w_state = SelfState {
        id: w_state_id.clone(),
        subject: "wunicorm".to_string(),
        window_start: window.0,
        window_end: window.1,
        judgment_coord: f64::from(decomposition.position.0),
        perception_coord: f64::from(decomposition.position.1),
        judgment_core: f64::from(decomposition.core.0),
        perception_core: f64::from(decomposition.core.1),
        judgment_comp: f64::from(decomposition.comp.0),
        perception_comp: f64::from(decomposition.comp.1),
        rigidity_judgment: f64::from(rigidities.0),
        rigidity_perception: f64::from(rigidities.1),
        evidence_kept: kept_total,
        evidence_dropped: dropped_total,
        learned_at: now,
        created_at: now,
        updated_at: now,
        layer: 4,
        context: spec.run_id.clone(),
    };
    match insert_self_state(conn, &w_state) {
        Ok(()) => *nodes_written += 1,
        Err(e) => errors.push(format!("self pass: wunicorm state write failed: {e}")),
    }
    wire_trajectory(
        conn,
        "wunicorm",
        window.0,
        &w_state_id,
        nodes_written,
        errors,
    );
    // Provenance: the memories this run chose to keep — W's evidence is
    // its choices.
    match kept_memory_ids(conn, &spec.run_id) {
        Ok(ids) => {
            for memory_id in ids {
                ground_evidence(
                    conn,
                    &w_state_id,
                    &memory_id,
                    "Memory",
                    "choice",
                    &spec.run_id,
                    now,
                    nodes_written,
                    errors,
                );
            }
        }
        Err(e) => errors.push(format!("self pass: kept memory read failed: {e}")),
    }

    // 4. The drift reading — U's latest step on the axes.
    let drift = match function_drift(conn, "user") {
        Ok(d) => d.map(|d| (d.judgment, d.perception)),
        Err(e) => {
            errors.push(format!("self pass: function drift read failed: {e}"));
            None
        }
    };

    Some(SelfPassReport {
        user_position: user,
        wunicorm_core: decomposition.core,
        wunicorm_comp: decomposition.comp,
        wunicorm_position: decomposition.position,
        rigidity: rigidities,
        drift,
        evidence_kept: kept_total,
        evidence_dropped: dropped_total,
    })
}

/// A timestamp column value → chrono, the boundary swap.
fn chrono_timestamp(v: &Value) -> Option<DateTime<Utc>> {
    match v {
        Value::Timestamp(t) => {
            Some(DateTime::from_timestamp(t.unix_timestamp(), 0).expect("valid unix timestamp"))
        }
        _ => None,
    }
}

/// The most recent state of one subject from a strictly earlier window.
fn latest_state_before(
    conn: &Connection,
    subject: &str,
    window_start: DateTime<Utc>,
) -> DbResult<Option<String>> {
    let mut stmt =
        conn.prepare("MATCH (s:SelfState {subject: $subject}) RETURN s.id, s.window_start;")?;
    let result = conn.execute(
        &mut stmt,
        vec![("subject", Value::String(subject.to_string()))],
    )?;
    let mut best: Option<(DateTime<Utc>, String)> = None;
    for row in result {
        let [Value::String(id), ts] = row.as_slice() else {
            continue;
        };
        let Some(at) = chrono_timestamp(ts) else {
            continue;
        };
        if at < window_start && best.as_ref().is_none_or(|(t, _)| at > *t) {
            best = Some((at, id.clone()));
        }
    }
    Ok(best.map(|(_, id)| id))
}

/// W's previous core — the damped position this run evolves from.
/// `(0.0, 0.0)` when W has never stood before.
fn wunicorm_prev_core(conn: &Connection) -> DbResult<(f32, f32)> {
    let mut stmt = conn.prepare(
        "MATCH (s:SelfState {subject: 'wunicorm'}) RETURN s.window_start, s.judgment_core, s.perception_core;",
    )?;
    let result = conn.execute(&mut stmt, vec![])?;
    let mut best: Option<(DateTime<Utc>, f32, f32)> = None;
    for row in result {
        let [ts, Value::Double(j), Value::Double(p)] = row.as_slice() else {
            continue;
        };
        let Some(at) = chrono_timestamp(ts) else {
            continue;
        };
        // f64 columns were written from f32 — same precision back.
        #[allow(clippy::cast_possible_truncation)]
        let candidate = (at, *j as f32, *p as f32);
        if best.as_ref().is_none_or(|(t, _, _)| candidate.0 > *t) {
            best = Some(candidate);
        }
    }
    Ok(best.map_or((0.0, 0.0), |(_, j, p)| (j, p)))
}

/// The ledger's evidence counts: `(total kept, dropped)`.
fn dream_run_evidence(conn: &Connection, run_id: &str) -> DbResult<(i64, i64)> {
    let mut stmt = conn.prepare(
        "MATCH (r:DreamRun {id: $id}) RETURN r.kept_t + r.kept_f + r.kept_s + r.kept_n, r.dropped_total;",
    )?;
    let result = conn.execute(&mut stmt, vec![("id", Value::String(run_id.to_string()))])?;
    for row in result {
        if let [Value::Int64(kept), Value::Int64(dropped)] = row.as_slice() {
            return Ok((*kept, *dropped));
        }
    }
    Ok((0, 0))
}

/// The ids of the memories this run chose to keep.
fn kept_memory_ids(conn: &Connection, run_id: &str) -> DbResult<Vec<String>> {
    let mut stmt = conn.prepare("MATCH (m:Memory {run_id: $id}) RETURN m.id;")?;
    let result = conn.execute(&mut stmt, vec![("id", Value::String(run_id.to_string()))])?;
    Ok(result
        .into_iter()
        .filter_map(|row| match row.first() {
            Some(Value::String(id)) => Some(id.clone()),
            _ => None,
        })
        .collect())
}

/// Wires the trajectory edge from the subject's previous state, fenced
/// like every other link — a dangling reference is a recorded fault.
fn wire_trajectory(
    conn: &Connection,
    subject: &str,
    window_start: DateTime<Utc>,
    next_id: &str,
    nodes_written: &mut usize,
    errors: &mut Vec<String>,
) {
    let prev = match latest_state_before(conn, subject, window_start) {
        Ok(Some(p)) => p,
        Ok(None) => return, // first state — the trajectory starts here
        Err(e) => {
            errors.push(format!("self pass: trajectory read failed: {e}"));
            return;
        }
    };
    let from = NodeRef::new(prev.clone(), "SelfState");
    let to = NodeRef::new(next_id.to_string(), "SelfState");
    link_edge(
        conn,
        "SELF_TRAJECTORY",
        &from,
        &to,
        || wire_self_trajectory(conn, &prev, next_id).map(|()| String::new()),
        nodes_written,
        errors,
    );
}

/// Grounds one state in one piece of evidence — the provenance edge,
/// fenced like every other link.
#[allow(clippy::too_many_arguments)]
fn ground_evidence(
    conn: &Connection,
    state_id: &str,
    evidence_id: &str,
    evidence_label: &str,
    evidence_kind: &str,
    run_id: &str,
    now: DateTime<Utc>,
    nodes_written: &mut usize,
    errors: &mut Vec<String>,
) {
    let edge_id = format!("g-{state_id}-{evidence_id}");
    let from = NodeRef::new(state_id.to_string(), "SelfState");
    let to = NodeRef::new(evidence_id.to_string(), evidence_label);
    let props = Grounds {
        id: edge_id.clone(),
        evidence_kind: evidence_kind.to_string(),
        label: "grounds".to_string(),
        label_resolved: "self grounded in evidence".to_string(),
        learned_at: now,
        expire_at: None,
        created_at: now,
        updated_at: now,
        layer: 0,
        context: run_id.to_string(),
    };
    link_edge(
        conn,
        "Grounds",
        &from,
        &to,
        || link_grounds(conn, &from, &to, &props),
        nodes_written,
        errors,
    );
}

/// Applies one judgment's writes. Every write is idempotent; every
/// failure is recorded and the dream continues. Kept memories are
/// stamped with the run id — the provenance of the choice, W's ledger.
fn apply_judgment(
    conn: &Connection,
    judgment: &DreamJudgment,
    run_id: &str,
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
        let mut m = m.clone();
        m.run_id = Some(run_id.to_string());
        write_node(insert_memory(conn, &m), "memory", &m.id, errors);
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

/// Links one edge, validating the endpoints first: label legality
/// against the schema's poly-rel allowlists, then existence. An
/// illegal label is a wrong-shaped claim, a dangling reference a
/// hallucinated one — either is a recorded fault, never a silent
/// no-op and never a raw binder error from the database.
fn link_edge(
    conn: &Connection,
    edge_name: &str,
    from: &NodeRef,
    to: &NodeRef,
    link: impl FnOnce() -> DbResult<String>,
    nodes_written: &mut usize,
    errors: &mut Vec<String>,
) {
    if let Some(kind) = EdgeType::from_label(edge_name) {
        for (node, side, legal) in [
            (from, "FROM", kind.allowed_from_labels()),
            (to, "TO", kind.allowed_to_labels()),
        ] {
            if !legal.contains(&node.label.as_str()) {
                errors.push(format!(
                    "{edge_name} edge skipped — illegal label '{}' for {side}_{edge_name}: {} (expected one of: {})",
                    node.label,
                    node.id,
                    legal.join(", ")
                ));
                return;
            }
        }
    }
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

/// The reflection stage: read the model the dream just updated, write
/// the sermon into the outbox, and hand it to the delivery seam.
///
/// Returns `(message_written, shelf_ok, note_delivered)`.
fn reflect_and_speak(
    conn: &Connection,
    sermon: &dyn SermonWriter,
    delivery: &dyn MessageDelivery,
    spec: &DreamSpec,
    now: DateTime<Utc>,
    self_pass: Option<SelfPassReport>,
    errors: &mut Vec<String>,
) -> (bool, bool, bool) {
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
        return (false, false, false);
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

    // Assemble, write, prove. The prose is the writer's; the
    // citations are always mechanical — grounding is never delegated.
    // The self reading reaches the voice as movement language only —
    // rendered here, numbers never handed over.
    let self_reading = self_pass.as_ref().map(render_self_reading);
    let prose = match sermon.compose(&anchor_name, &insights, &citations, self_reading.as_ref()) {
        Ok(p) => p,
        Err(e) => {
            errors.push(format!(
                "sermon writer failed, fell back to mechanical prose: {e}"
            ));
            MechanicalSermon
                .compose(&anchor_name, &insights, &citations, self_reading.as_ref())
                .unwrap_or_default()
        }
    };
    let mut body = prose.trim_end().to_string();
    if !citations.is_empty() {
        let _ = write!(body, "\n\n{}", citations_block(&citations));
    }
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
            Ok(()) => {
                // The artifact is green; delivery is a separate truth,
                // verified by the impl's own read-back.
                let outcome = delivery.deliver(&msg);
                errors.extend(outcome.errors);
                (true, true, outcome.note_delivered)
            }
            Err(reason) => {
                errors.push(format!("shelf check failed: {reason}"));
                (true, false, false)
            }
        },
        Err(e) => {
            errors.push(format!("message write failed: {e}"));
            (false, false, false)
        }
    }
}

/// The grounding block, rendered mechanically: every citation, always
/// in the same shape, never the writer's job.
fn citations_block(citations: &[Citation]) -> String {
    let mut block = String::from("Grounded in your notes:\n");
    for c in citations {
        let _ = writeln!(block, "- [{}] {}", c.note_id, c.snippet);
    }
    block.trim_end().to_string()
}

/// First ~80 characters of a note — enough to recognize it, not enough
/// to bloat the artifact.
fn snippet(content: &str) -> String {
    content.chars().take(80).collect()
}
