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
