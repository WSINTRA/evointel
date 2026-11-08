use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Note,
    Entity,
    Time,
    AbstractTime,
    Fact,
    Event,
    Memory,
}

impl NodeType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Note => "Note",
            Self::Entity => "Entity",
            Self::Time => "Time",
            Self::AbstractTime => "AbstractTime",
            Self::Fact => "Fact",
            Self::Event => "Event",
            Self::Memory => "Memory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub content: String,
    pub content_embedding: Option<Vec<f32>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub label: String,
    pub label_resolved: String,
    pub label_embedding: Option<Vec<f32>>,
    pub learned_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Time {
    pub id: String,
    pub granularity: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub label: String,
    pub label_resolved: String,
    pub learned_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbstractTime {
    pub id: String,
    pub semantics: String,
    pub label: String,
    pub label_resolved: String,
    pub learned_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub id: String,
    pub predicate: String,
    pub certainty: f64,
    pub source: String,
    pub label: String,
    pub label_resolved: String,
    pub label_embedding: Option<Vec<f32>>,
    pub learned_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub predicate: String,
    pub certainty: f64,
    pub source: String,
    pub status: String,
    pub is_ongoing: bool,
    pub label: String,
    pub label_resolved: String,
    pub label_embedding: Option<Vec<f32>>,
    pub learned_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub predicate: String,
    pub certainty: f64,
    pub source: String,
    pub status: String,
    pub is_ongoing: bool,
    pub significance: String,
    pub emotions: Vec<String>,
    pub reflection: String,
    /// The dominant lens of the reflection — one of the four functions
    /// (`axes::Function` spellings). The extractor's attribution; W's
    /// position is measured from these choices.
    pub function_tag: Option<String>,
    /// The dream run that chose to keep this memory — the provenance of
    /// the choice. Stamped by the dreamer's write path, not the judge.
    pub run_id: Option<String>,
    pub label: String,
    pub label_resolved: String,
    pub label_embedding: Option<Vec<f32>>,
    pub learned_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

/// One exemplar of one pole, seeded from `prompts/anchors-v1.md`.
/// `function` and `axis` are the schema spellings validated by
/// `axes::Function::parse` / `axes::Axis::parse` at seed time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionAnchor {
    pub id: String,
    pub function: String,
    pub axis: String,
    pub exemplar: String,
    pub embedding: Option<Vec<f32>>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

/// One point in function space for one subject in one window. User
/// states carry only the observed `*_coord` (read, never steered);
/// Wunicorm states carry the full decomposition — `*_core` (damped
/// hysteresis), `*_comp` (rigidity-gated fast component) — plus the
/// rigidity reading the compensation was gated on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfState {
    pub id: String,
    /// `user` or `wunicorm`.
    pub subject: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub judgment_coord: f64,
    pub perception_coord: f64,
    pub judgment_core: f64,
    pub perception_core: f64,
    pub judgment_comp: f64,
    pub perception_comp: f64,
    pub rigidity_judgment: f64,
    pub rigidity_perception: f64,
    pub evidence_kept: i64,
    pub evidence_dropped: i64,
    pub learned_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

/// The dreamer's per-run choice ledger: considered versus kept, per
/// function. The "discarded is data" channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DreamRun {
    pub id: String,
    pub considered: Vec<String>,
    pub dropped: Vec<String>,
    pub kept_t: i64,
    pub kept_f: i64,
    pub kept_s: i64,
    pub kept_n: i64,
    pub dropped_total: i64,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}
