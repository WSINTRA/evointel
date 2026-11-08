//! The reflection's output artifact.
//!
//! A `Message` is what the system *says* to the user — the sermon, a
//! notification, an attention card. It is written as JSON to an outbox
//! directory: the file is the delivery artifact today (and the thing the
//! shelf check inspects); the email / notification / streaming service
//! layer later slots in behind the same struct.
//!
//! The graph stays 100% derived — messages deliberately live *outside*
//! it. Citations point back at source notes so that "no revision
//! without observation" stays checkable.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::mutations::NodeRef;
use crate::queries::Drift;

/// Which channel this message is destined for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    /// The daily reflective address (the letter channel).
    Sermon,
    /// The in-app pointer, expanded for extra insight.
    Notification,
    /// Mechanical, fault-only; no model writes it.
    AttentionCard,
}

impl MessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sermon => "sermon",
            Self::Notification => "notification",
            Self::AttentionCard => "attention_card",
        }
    }
}

/// A note the message is grounded in — evidence, not vibes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Citation {
    pub note_id: String,
    pub snippet: String,
}

/// One finding the reflection is reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Insight {
    pub summary: String,
    /// The concept the insight is about, if any.
    pub concept: Option<NodeRef>,
    /// The drift evidence behind it, if the insight came from a drift
    /// read.
    pub drift: Option<Drift>,
}

/// A message from the system to the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    /// The dream run that produced this message — the shelf check
    /// verifies the artifact was written by *this* run, not a stale one.
    pub run_id: String,
    pub kind: MessageKind,
    pub generated_at: DateTime<Utc>,
    pub insights: Vec<Insight>,
    pub citations: Vec<Citation>,
    /// The rendered text (what an email adapter would send).
    pub body: String,
}

impl Message {
    /// Deterministic file name inside an outbox: a re-run of the same
    /// run + kind overwrites rather than duplicating.
    pub fn file_name(&self) -> String {
        format!("{}-{}.json", self.run_id, self.kind.as_str())
    }
}

/// Writes the message as pretty JSON into `outbox` (created if missing)
/// and returns the artifact path.
pub fn write_message(msg: &Message, outbox: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(outbox)?;
    let path = outbox.join(msg.file_name());
    let json = serde_json::to_string_pretty(msg).map_err(io::Error::other)?;
    fs::write(&path, json)?;
    Ok(path)
}

/// The durable core's shelf check, applied to a message artifact.
///
/// The artifact must exist, be non-empty, deserialize, carry the
/// expected run's id, and actually say something (insights, citations,
/// body). A green `write_message` return code never stands in for a
/// result — this is the check that proves it.
///
/// Returns `Err(reason)` on failure; reasons are human-readable.
pub fn shelf_check(path: &Path, expected_run_id: &str) -> Result<(), String> {
    let bytes =
        fs::read(path).map_err(|e| format!("unreadable artifact at {}: {e}", path.display()))?;
    if bytes.is_empty() {
        return Err("artifact is empty".to_string());
    }
    let msg: Message = serde_json::from_slice(&bytes)
        .map_err(|e| format!("artifact is not a valid message: {e}"))?;
    if msg.run_id != expected_run_id {
        return Err(format!(
            "artifact from run {} — expected {expected_run_id}",
            msg.run_id
        ));
    }
    if msg.insights.is_empty() {
        return Err("artifact carries no insights".to_string());
    }
    if msg.citations.is_empty() {
        return Err("artifact cites no source notes".to_string());
    }
    if msg.body.trim().is_empty() {
        return Err("artifact body is empty".to_string());
    }
    Ok(())
}
