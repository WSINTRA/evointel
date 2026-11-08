use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    Contains,
    Similar,
    HasProperty,
    LeadsTo,
    Causes,
    Prevents,
    BecauseOf,
    Before,
    After,
    During,
    ValidFrom,
    ValidTo,
    Grounds,
}

impl EdgeType {
    /// All 13 edge types, in schema declaration order (Grounds, the
    /// self layer's provenance edge, arrived with migration batch v3).
    pub const ALL: [Self; 13] = [
        Self::Contains,
        Self::Similar,
        Self::HasProperty,
        Self::LeadsTo,
        Self::Causes,
        Self::Prevents,
        Self::BecauseOf,
        Self::Before,
        Self::After,
        Self::During,
        Self::ValidFrom,
        Self::ValidTo,
        Self::Grounds,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Contains => "Contains",
            Self::Similar => "Similar",
            Self::HasProperty => "HasProperty",
            Self::LeadsTo => "LeadsTo",
            Self::Causes => "Causes",
            Self::Prevents => "Prevents",
            Self::BecauseOf => "BecauseOf",
            Self::Before => "Before",
            Self::After => "After",
            Self::During => "During",
            Self::ValidFrom => "ValidFrom",
            Self::ValidTo => "ValidTo",
            Self::Grounds => "Grounds",
        }
    }

    pub fn from_rel(self) -> String {
        format!("FROM_{}", self.label())
    }

    pub fn to_rel(self) -> String {
        format!("TO_{}", self.label())
    }
}

// ─── Structural edges ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contains {
    pub id: String,
    pub containment_types: String,
    pub weight: f64,
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
pub struct Similar {
    pub id: String,
    pub similarity: f64,
    pub sim_context: String,
    pub sim_method: String,
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
pub struct HasProperty {
    pub id: String,
    pub property_name: String,
    pub property_value: String,
    pub prop_context: String,
    pub certainty: f64,
    pub label: String,
    pub label_resolved: String,
    pub learned_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

// ─── Causal edges ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeadsTo {
    pub id: String,
    pub probability: f64,
    pub strength: f64,
    pub mechanism: String,
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
pub struct Causes {
    pub id: String,
    pub probability: f64,
    pub strength: f64,
    pub mechanism: String,
    pub directness: String,
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
pub struct Prevents {
    pub id: String,
    pub probability: f64,
    pub strength: f64,
    pub mechanism: String,
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
pub struct BecauseOf {
    pub id: String,
    pub probability: f64,
    pub strength: f64,
    pub explanation: String,
    pub label: String,
    pub label_resolved: String,
    pub learned_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

// ─── Temporal edges ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Before {
    pub id: String,
    pub gap_duration: String,
    pub confidence: f64,
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
pub struct After {
    pub id: String,
    pub gap_duration: String,
    pub confidence: f64,
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
pub struct During {
    pub id: String,
    pub overlap_type: String,
    pub confidence: f64,
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
pub struct ValidFrom {
    pub id: String,
    pub precision: String,
    pub confidence: f64,
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
pub struct ValidTo {
    pub id: String,
    pub precision: String,
    pub confidence: f64,
    pub termination: String,
    pub label: String,
    pub label_resolved: String,
    pub learned_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}

// ─── Self-layer edges ────────────────────────────────────────────────

/// The provenance edge: a `SelfState` grounded in its evidence — the
/// notes that filled its window (`evidence_kind = "window"`) or the
/// memories its run chose to keep (`evidence_kind = "choice"`). Every
/// positional claim traces to nodes through these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grounds {
    pub id: String,
    /// `window` or `choice`.
    pub evidence_kind: String,
    pub label: String,
    pub label_resolved: String,
    pub learned_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub layer: i16,
    pub context: String,
}
