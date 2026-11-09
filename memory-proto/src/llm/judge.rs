//! The live judge: [`Judge`] implemented over the client layer.
//!
//! The LLM decides *meaning* — labels, predicates, pillars, dates.
//! This module decides *mechanics* — ids, timestamps, layers,
//! context, embeddings. The thin [`RawJudgment`] the model emits
//! under grammar constraint is mapped onto the full [`DreamJudgment`]
//! the write path applies unchanged.
//!
//! Idempotency is constructed, not hoped for: node ids are
//! deterministic slugs of the semantic key (`e-job-search`,
//! `t-2026-11-03`), so a concept re-judged next night MERGEs onto the
//! same node instead of duplicating. Entities additionally go through
//! create-vs-link: a label that already resolves in the graph keeps
//! its existing id.
//!
//! The model's mistakes are named, never silent: an edge endpoint
//! that was never extracted becomes a phantom reference that the
//! dreamer's dangling-ref fence records as a fault — the note's good
//! content still lands. A malformed reply gets exactly one
//! schema-feedback retry; a second failure fails the note, recorded,
//! and the dream continues.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, TimeZone, Utc};
use lbug::{Connection, Value};
use serde::Deserialize;

use crate::dreamer::{DreamJudgment, Judge, Triple};
use crate::edges::{Contains, HasProperty, LeadsTo, Similar};
use crate::llm::{extract_prompt, extract_schema, redact, ChatMessage, LlmClient};
use crate::mutations::NodeRef;
use crate::nodes::{Entity, Event, Fact, Memory, Note, Time};

/// The judge over any [`LlmClient`] — `UreqClient` for the real
/// servers, `RecordedClient` for hermetic tests.
pub struct LlmJudge {
    client: Box<dyn LlmClient>,
}

impl LlmJudge {
    pub fn new(client: Box<dyn LlmClient>) -> Self {
        Self { client }
    }

    /// One chat call, parsed, with a single schema-feedback retry.
    fn ask(&self, note: &Note) -> Result<RawJudgment, String> {
        let system = extract_prompt().map_err(|e| e.to_string())?;
        let schema = extract_schema().map_err(|e| e.to_string())?;
        let content = redact(&note.content);
        let mut messages = vec![ChatMessage::system(&system), ChatMessage::user(&content)];

        for attempt in 0..2 {
            let raw = self
                .client
                .chat(&messages, Some(&schema))
                .map_err(|e| e.to_string())?;
            match serde_json::from_str::<RawJudgment>(&raw) {
                Ok(judgment) => return Ok(judgment),
                Err(e) if attempt == 0 => {
                    messages.push(ChatMessage::assistant(&raw));
                    messages.push(ChatMessage::user(&format!(
                        "That reply was not valid for the required JSON schema: {e}. \
                         Re-emit the complete judgment, following the schema exactly."
                    )));
                }
                Err(e) => {
                    return Err(format!(
                        "unparseable judgment after retry: {e}; raw: {}",
                        truncate(&raw, 400)
                    ));
                }
            }
        }
        unreachable!("the loop returns on both paths")
    }

    /// Meaning in, mechanics out: the thin judgment becomes a full
    /// [`DreamJudgment`] with deterministic ids, one batch of
    /// embeddings, a wired calendar, and pillar edges whose endpoints
    /// are resolved against the extraction.
    fn map(
        &self,
        conn: &Connection,
        note: &Note,
        raw: &RawJudgment,
    ) -> Result<DreamJudgment, String> {
        let now = Utc::now();
        let ctx = Ctx {
            note_id: note.id.clone(),
            now,
        };

        // Pre-filter: empty labels are noise, dropped before the
        // embedding batch so consumption stays in lockstep. Entities
        // dedupe by resolved label (models repeat themselves).
        let entities_raw: BTreeMap<String, String> = raw
            .entities
            .iter()
            .filter_map(|e| {
                let resolved = norm(&e.label);
                (!resolved.is_empty()).then_some((resolved, e.label.trim().to_string()))
            })
            .collect();
        let facts_raw: Vec<(String, String, String)> = raw
            .facts
            .iter()
            .filter_map(|f| {
                let resolved = norm(&f.label);
                (!resolved.is_empty()).then(|| {
                    (
                        resolved,
                        f.predicate.trim().to_string(),
                        f.label.trim().to_string(),
                    )
                })
            })
            .collect();
        let events_raw: Vec<(String, String, String, String)> = raw
            .events
            .iter()
            .filter_map(|ev| {
                let resolved = norm(&ev.label);
                (!resolved.is_empty()).then(|| {
                    let status = if ev.status.trim().is_empty() {
                        "occurred".to_string()
                    } else {
                        ev.status.trim().to_string()
                    };
                    (
                        resolved,
                        ev.predicate.trim().to_string(),
                        status,
                        ev.label.trim().to_string(),
                    )
                })
            })
            .collect();
        let memories_raw: Vec<(String, RawMemory)> = raw
            .memories
            .iter()
            .filter_map(|m| {
                let resolved = norm(&m.label);
                (!resolved.is_empty()).then_some((resolved, m.clone()))
            })
            .collect();

        // One batch of meaning for everything extracted.
        let mut embed_texts: Vec<String> = vec![];
        embed_texts.extend(entities_raw.values().cloned());
        embed_texts.extend(facts_raw.iter().map(|(_, _, label)| label.clone()));
        embed_texts.extend(events_raw.iter().map(|(_, _, _, label)| label.clone()));
        embed_texts.extend(memories_raw.iter().map(|(_, m)| m.label.trim().to_string()));
        let embeddings = if embed_texts.is_empty() {
            vec![]
        } else {
            let refs: Vec<&str> = embed_texts.iter().map(String::as_str).collect();
            self.client.embed(&refs).map_err(|e| e.to_string())?
        };

        // The calendar: every extracted time plus the parent chain it
        // implies, wired into the hierarchy. Invalid ISO strings are
        // skipped — a bad date never blocks good content.
        let mut times: BTreeMap<String, Time> = BTreeMap::new();
        let mut hierarchy: BTreeSet<(String, String)> = BTreeSet::new();
        for t in &raw.times {
            add_time(&t.iso, &ctx, &mut times, &mut hierarchy);
        }

        // Nodes, with create-vs-link on entities and deterministic
        // ids everywhere else.
        let mut labels: BTreeMap<String, NodeRef> = BTreeMap::new();
        let mut entities: Vec<Entity> = vec![];
        for (i, (resolved, label)) in entities_raw.iter().enumerate() {
            let id = existing_entity_id(conn, resolved)
                .unwrap_or_else(|| format!("e-{}", slug(resolved)));
            labels.insert(resolved.clone(), NodeRef::new(id.clone(), "Entity"));
            entities.push(Entity {
                id,
                label: label.clone(),
                label_resolved: resolved.clone(),
                label_embedding: Some(next_embedding_n(&embeddings, i)),
                learned_at: now,
                expire_at: None,
                created_at: now,
                updated_at: now,
                layer: 1,
                context: ctx.note_id.clone(),
            });
        }
        let mut facts: Vec<Fact> = vec![];
        for (i, (resolved, predicate, label)) in facts_raw.iter().enumerate() {
            let id = format!("f-{}", slug(resolved));
            labels.insert(resolved.clone(), NodeRef::new(id.clone(), "Fact"));
            facts.push(Fact {
                id,
                predicate: predicate.clone(),
                certainty: 1.0,
                source: ctx.note_id.clone(),
                label: label.clone(),
                label_resolved: resolved.clone(),
                label_embedding: Some(next_embedding_n(&embeddings, entities.len() + i)),
                learned_at: now,
                expire_at: None,
                created_at: now,
                updated_at: now,
                layer: 2,
                context: ctx.note_id.clone(),
            });
        }
        let mut events: Vec<Event> = vec![];
        for (i, (resolved, predicate, status, label)) in events_raw.iter().enumerate() {
            let id = format!("ev-{}", slug(resolved));
            labels.insert(resolved.clone(), NodeRef::new(id.clone(), "Event"));
            events.push(Event {
                id,
                predicate: predicate.clone(),
                certainty: 1.0,
                source: ctx.note_id.clone(),
                status: status.clone(),
                is_ongoing: false,
                label: label.clone(),
                label_resolved: resolved.clone(),
                label_embedding: Some(next_embedding_n(
                    &embeddings,
                    entities.len() + facts.len() + i,
                )),
                learned_at: now,
                expire_at: None,
                created_at: now,
                updated_at: now,
                layer: 3,
                context: ctx.note_id.clone(),
            });
        }
        let mut memories: Vec<Memory> = vec![];
        for (i, (resolved, m)) in memories_raw.iter().enumerate() {
            // The function attribution is W's evidence — an unknown tag
            // is a broken instrument, and the whole judgment is refused
            // (recorded as a run fault, house pattern), never silently
            // defaulted.
            if crate::axes::Function::parse(m.function.trim()).is_none() {
                return Err(format!(
                    "memory '{}' carries an unknown function '{}'",
                    m.label.trim(),
                    m.function.trim()
                ));
            }
            let id = format!("m-{}", slug(resolved));
            labels.insert(resolved.clone(), NodeRef::new(id.clone(), "Memory"));
            memories.push(Memory {
                id,
                predicate: m.predicate.trim().to_string(),
                certainty: 1.0,
                source: ctx.note_id.clone(),
                status: "occurred".to_string(),
                is_ongoing: false,
                significance: m.significance.trim().to_string(),
                emotions: m.emotions.clone(),
                reflection: m.reflection.trim().to_string(),
                function_tag: Some(m.function.trim().to_string()),
                run_id: None, // stamped by the dreamer's write path
                label: m.label.trim().to_string(),
                label_resolved: resolved.clone(),
                label_embedding: Some(next_embedding_n(
                    &embeddings,
                    entities.len() + facts.len() + events.len() + i,
                )),
                learned_at: now,
                expire_at: None,
                created_at: now,
                updated_at: now,
                layer: 4,
                context: ctx.note_id.clone(),
            });
        }
        for iso in times.keys() {
            labels.insert(iso.clone(), NodeRef::new(format!("t-{iso}"), "Time"));
        }

        // Pillar edges. Endpoints resolve against the extraction; an
        // unknown label becomes a phantom Entity. The write path's
        // fences name it — as an illegal-label fault when the pillar
        // forbids that endpoint's type outright, as a dangling
        // reference when the type is legal but the node was never
        // written.
        let mut contains: Vec<Triple<Contains>> = vec![];
        let mut similar: Vec<Triple<Similar>> = vec![];
        let mut has_property: Vec<Triple<HasProperty>> = vec![];
        let mut leads_to: Vec<Triple<LeadsTo>> = vec![];
        for edge in &raw.edges {
            let from = resolve_endpoint(&labels, note, &edge.from);
            let to = resolve_endpoint(&labels, note, &edge.to);
            let id = format!(
                "{}-{}-{}",
                edge.pillar.slug_prefix(),
                endpoint_slug(note, &edge.from),
                endpoint_slug(note, &edge.to),
            );
            match edge.pillar {
                Pillar::Contains => contains.push((from, to, contains_props(&ctx, &id))),
                Pillar::Similar => similar.push((from, to, similar_props(&ctx, &id))),
                Pillar::HasProperty => {
                    let value = edge
                        .property_value
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&edge.to);
                    let name = edge
                        .property_name
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .unwrap_or("property");
                    has_property.push((from, to, has_property_props(&ctx, &id, name, value)));
                }
                Pillar::LeadsTo => {
                    let from_name = &edge.from;
                    let to_name = &edge.to;
                    leads_to.push((
                        from,
                        to,
                        leads_to_props(&ctx, &id, &norm(from_name), &norm(to_name)),
                    ));
                }
            }
        }

        Ok(DreamJudgment {
            note_id: note.id.clone(),
            entities,
            facts,
            events,
            memories,
            times: times.values().cloned().collect(),
            hierarchy: hierarchy.into_iter().collect(),
            contains,
            similar,
            has_property,
            leads_to,
        })
    }
}

impl Judge for LlmJudge {
    fn judge(&self, conn: &Connection, note: &Note) -> Result<DreamJudgment, String> {
        let raw = self.ask(note)?;
        self.map(conn, note, &raw)
    }
}

// ─── The thin judgment (what the model emits) ───────────────────────

/// The model's raw output — labels, predicates, pillars, dates.
/// Mirrors `prompts/schema-v1.json` exactly; the grammar makes
/// structurally invalid JSON impossible, serde verifies the shape
/// anyway.
#[derive(Debug, Clone, Deserialize)]
pub struct RawJudgment {
    #[serde(default)]
    pub entities: Vec<RawEntity>,
    #[serde(default)]
    pub facts: Vec<RawFact>,
    #[serde(default)]
    pub events: Vec<RawEvent>,
    #[serde(default)]
    pub memories: Vec<RawMemory>,
    #[serde(default)]
    pub times: Vec<RawTime>,
    #[serde(default)]
    pub edges: Vec<RawEdge>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawEntity {
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawFact {
    pub predicate: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawEvent {
    pub predicate: String,
    pub status: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawMemory {
    pub predicate: String,
    pub significance: String,
    #[serde(default)]
    pub emotions: Vec<String>,
    pub reflection: String,
    /// The dominant lens of the reflection — one of the four functions.
    /// The grammar constrains the enum; serde plus the mapping's own
    /// parse are the fence for anything that slips through.
    pub function: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawTime {
    pub iso: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawEdge {
    pub pillar: Pillar,
    pub from: String,
    pub to: String,
    pub property_name: Option<String>,
    pub property_value: Option<String>,
}

/// The four pillars, as the model names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Pillar {
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "similar")]
    Similar,
    #[serde(rename = "has_property")]
    HasProperty,
    #[serde(rename = "leads_to")]
    LeadsTo,
}

impl Pillar {
    fn slug_prefix(self) -> &'static str {
        match self {
            Self::Contains => "c",
            Self::Similar => "sim",
            Self::HasProperty => "hp",
            Self::LeadsTo => "lt",
        }
    }
}

// ─── Mechanics ──────────────────────────────────────────────────────

struct Ctx {
    note_id: String,
    now: DateTime<Utc>,
}

fn contains_props(ctx: &Ctx, id: &str) -> Contains {
    Contains {
        id: id.to_string(),
        containment_types: "mention".to_string(),
        weight: 1.0,
        label: "contains".to_string(),
        label_resolved: "note mentions concept".to_string(),
        learned_at: ctx.now,
        expire_at: None,
        created_at: ctx.now,
        updated_at: ctx.now,
        layer: 1,
        context: ctx.note_id.clone(),
    }
}

fn similar_props(ctx: &Ctx, id: &str) -> Similar {
    Similar {
        id: id.to_string(),
        similarity: 0.8,
        sim_context: "llm-judged".to_string(),
        sim_method: "model judgment".to_string(),
        label: "similar".to_string(),
        label_resolved: "concepts sit close".to_string(),
        learned_at: ctx.now,
        expire_at: None,
        created_at: ctx.now,
        updated_at: ctx.now,
        layer: 1,
        context: ctx.note_id.clone(),
    }
}

fn has_property_props(ctx: &Ctx, id: &str, name: &str, value: &str) -> HasProperty {
    HasProperty {
        id: id.to_string(),
        property_name: name.to_string(),
        property_value: value.to_string(),
        prop_context: ctx.note_id.clone(),
        certainty: 1.0,
        label: "has property".to_string(),
        label_resolved: format!("{name} = {value}"),
        learned_at: ctx.now,
        expire_at: None,
        created_at: ctx.now,
        updated_at: ctx.now,
        layer: 1,
        context: ctx.note_id.clone(),
    }
}

fn leads_to_props(ctx: &Ctx, id: &str, from: &str, to: &str) -> LeadsTo {
    LeadsTo {
        id: id.to_string(),
        probability: 0.9,
        strength: 0.7,
        mechanism: "judged from note".to_string(),
        label: "leads to".to_string(),
        label_resolved: format!("{from} leads to {to}"),
        learned_at: ctx.now,
        expire_at: None,
        created_at: ctx.now,
        updated_at: ctx.now,
        layer: 2,
        context: ctx.note_id.clone(),
    }
}

/// `Job_Search!` → `job search` — the canonical semantic key. Runs of
/// underscores and whitespace collapse to one space, because the
/// model writes `job_search` some nights and `job search` on others,
/// and those are one concept. Dashes are preserved — ISO dates pass
/// through untouched.
fn norm(s: &str) -> String {
    let lowered = s.trim().to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut pending_space = false;
    for ch in lowered.chars() {
        if ch == '_' || ch.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(ch);
        }
    }
    out
}

/// `job search` → `job-search` — the id slug.
fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            dash = true;
        } else if dash && !out.ends_with('-') {
            out.push('-');
            dash = false;
        }
    }
    out.trim_end_matches('-').to_string()
}

fn resolve_endpoint(labels: &BTreeMap<String, NodeRef>, note: &Note, endpoint: &str) -> NodeRef {
    let key = norm(endpoint);
    if key == "note" {
        return NodeRef::new(note.id.clone(), "Note");
    }
    if let Some(r) = labels.get(&key) {
        return r.clone();
    }
    // Exact-form fallback: keys that are not prose (a time's ISO
    // string) must survive normalization untouched.
    if let Some(r) = labels.get(endpoint.trim()) {
        return r.clone();
    }
    NodeRef::new(format!("e-{}", slug(&key)), "Entity")
}

/// The slug for an edge endpoint: `note` slugs to the note id, every
/// other endpoint slugs to its label — edge identity keys on meaning,
/// so a create-vs-link'd entity (foreign id, same label) still
/// produces the same edge id.
fn endpoint_slug(note: &Note, endpoint: &str) -> String {
    let key = norm(endpoint);
    if key == "note" {
        return slug(&note.id);
    }
    slug(&key)
}

fn next_embedding_n(embeddings: &[Vec<f32>], i: usize) -> Vec<f32> {
    embeddings.get(i).cloned().unwrap_or_default()
}

/// Create-vs-link: an entity that already resolves in the graph keeps
/// its id. A lookup error counts as not-found — the deterministic
/// slug id still MERGEs onto the same node either way.
fn existing_entity_id(conn: &Connection, label_resolved: &str) -> Option<String> {
    let mut stmt = conn
        .prepare("MATCH (e:Entity {label_resolved: $key}) RETURN e.id;")
        .ok()?;
    let result = conn
        .execute(
            &mut stmt,
            vec![("key", Value::String(label_resolved.to_string()))],
        )
        .ok()?;
    for row in result {
        if let Value::String(id) = &row[0] {
            return Some(id.clone());
        }
    }
    None
}

// ─── The calendar ───────────────────────────────────────────────────

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn dt(y: i32, m: u32, d: u32) -> Option<DateTime<Utc>> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).single()
}

/// One ISO period (`2026`, `2026-11`, `2026-11-03`) as calendar
/// facts. `ends_at` follows the seed convention: the last day of the
/// period at midnight.
fn iso_time(iso: &str) -> Option<(String, String, DateTime<Utc>, DateTime<Utc>)> {
    let parts: Vec<&str> = iso.split('-').collect();
    match parts.as_slice() {
        [y] => {
            let y: i32 = y.parse().ok()?;
            Some((
                "year".to_string(),
                y.to_string(),
                dt(y, 1, 1)?,
                dt(y, 12, 31)?,
            ))
        }
        [y, m] => {
            let y: i32 = y.parse().ok()?;
            let m: u32 = m.parse().ok()?;
            if !(1..=12).contains(&m) {
                return None;
            }
            let last = last_day(y, m)?;
            Some((
                "month".to_string(),
                format!("{} {y}", MONTHS[(m - 1) as usize]),
                dt(y, m, 1)?,
                dt(y, m, last)?,
            ))
        }
        [y, m, d] => {
            let y: i32 = y.parse().ok()?;
            let m: u32 = m.parse().ok()?;
            let d: u32 = d.parse().ok()?;
            if !(1..=12).contains(&m) {
                return None;
            }
            Some((
                "day".to_string(),
                format!("{} {d}, {y}", MONTHS[(m - 1) as usize]),
                dt(y, m, d)?,
                dt(y, m, d)?,
            ))
        }
        _ => None,
    }
}

fn last_day(y: i32, m: u32) -> Option<u32> {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    Some(dt(ny, nm, 1)?.date_naive().pred_opt()?.day())
}

/// The parent chain a period implies: a day carries its month and
/// year; a month carries its year.
fn parent_isos(iso: &str) -> Vec<String> {
    match iso.split('-').count() {
        3 => vec![iso[..iso.rfind('-').expect("day has a month part")].to_string()],
        2 => vec![iso[..iso.find('-').expect("month has a year part")].to_string()],
        _ => vec![],
    }
}

fn add_time(
    iso: &str,
    ctx: &Ctx,
    times: &mut BTreeMap<String, Time>,
    hierarchy: &mut BTreeSet<(String, String)>,
) {
    if times.contains_key(iso) {
        return;
    }
    let Some((granularity, label, starts_at, ends_at)) = iso_time(iso) else {
        return;
    };
    times.insert(
        iso.to_string(),
        Time {
            id: format!("t-{iso}"),
            granularity,
            starts_at,
            ends_at: Some(ends_at),
            label,
            label_resolved: iso.to_string(),
            learned_at: ctx.now,
            expire_at: None,
            created_at: ctx.now,
            updated_at: ctx.now,
            layer: 1,
            context: ctx.note_id.clone(),
        },
    );
    for parent in parent_isos(iso) {
        add_time(&parent, ctx, times, hierarchy);
        hierarchy.insert((format!("t-{parent}"), format!("t-{iso}")));
    }
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}
