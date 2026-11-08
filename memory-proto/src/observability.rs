//! Dream observability — the dreamer watching itself.
//!
//! A `DreamReport` is the observation of one dream run; the JSONL log is
//! the append-only record of them. Both deliberately live *outside* the
//! memory graph: the graph stays 100% derived (rebuildable from notes),
//! while the system's self-observations are their own source of truth.
//!
//! This is the durable core re-homed as the nightmare detector: two
//! kinds of truth (declaration: "the scheduler fired at 3am" vs
//! observation: this report), the shelf check (`shelf_ok`), and the
//! fault ledger (the log — a nightmare is closed by a fix, not prose).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The observation of one dream run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DreamReport {
    pub run_id: String,
    pub started_at: DateTime<Utc>,
    /// `None` — the dream crashed and never finished.
    pub finished_at: Option<DateTime<Utc>>,
    /// Notes pulled from the queue for this run.
    pub notes_in: usize,
    /// Node writes the dreamer *executed* (idempotent `MERGE` targets),
    /// whether or not they created anything new. A legitimate re-run
    /// writes everything again and observes `nodes_added == 0` — that
    /// must never be read as "extracted nothing".
    pub nodes_written: usize,
    /// New node keys observed vs the pre-dream snapshot (analytics).
    pub nodes_added: usize,
    /// New edge triples + time rels observed vs the pre-dream snapshot.
    pub edges_added: usize,
    /// Errors encountered during the run (any one is a nightmare).
    pub errors: Vec<String>,
    /// Whether this run produced a message artifact at all. Quiet days
    /// (empty queue) write no sermon on purpose — silence is the correct
    /// output when there is nothing new to say.
    pub message_written: bool,
    /// The message artifact passed its shelf check. Vacuously true on
    /// quiet days (nothing was supposed to be produced).
    pub shelf_ok: bool,
    /// Structural similarity to the pre-dream graph, when a pre-dream
    /// snapshot was taken. `None` means "not measured" — which is not
    /// the same as "stable".
    pub similarity_to_prev: Option<f32>,
}

/// Decides whether a dream was a nightmare.
///
/// The invariants, in the durable core's terms:
/// - **didn't finish** — declaration (scheduled) never became
///   observation (ran);
/// - **errors** — the write path hit something it couldn't handle;
/// - **processed notes but extracted nothing** — `notes_in > 0` with
///   `nodes_written == 0` means the dreamer consumed evidence and wrote
///   no model at all (deliberately counted as *executed writes*, not new
///   nodes, so an idempotent re-run is never mistaken for failure);
/// - **failed shelf check** — no proven artifact;
/// - **structural shock** — similarity to the previous graph cratered
///   below `similarity_floor`.
///
/// `similarity_floor` is a *parameter*, not a constant: on a young graph
/// a single legitimate dream can take Jaccard similarity low (the delta
/// is large relative to the whole), while on a mature graph the same
/// score means the dream demolished the model. The caller owns that
/// policy; the predicate stays pure.
///
/// Recovery is a re-run (every write is an idempotent `MERGE`), never a
/// silent retry — and repeated nightmares escalate to the attention
/// card, where a human decides.
pub fn is_nightmare(report: &DreamReport, similarity_floor: f32) -> bool {
    report.finished_at.is_none()
        || !report.errors.is_empty()
        || (report.notes_in > 0 && report.nodes_written == 0)
        || !report.shelf_ok
        || report
            .similarity_to_prev
            .is_some_and(|s| s < similarity_floor)
}

/// Appends one report as a single JSON line — the dream ledger.
///
/// One line per run; never rewritten. The analytics layer reads this
/// file; nothing here ever writes to the memory graph.
pub fn append_dream_report(log_path: &Path, report: &DreamReport) -> io::Result<()> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(report).map_err(io::Error::other)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "{line}")
}

/// Reads the dream ledger back, oldest line first.
///
/// This is the "analytics" today: dead simple, and the only consumer of
/// the log until a service layer wants charts.
pub fn read_dream_reports(log_path: &Path) -> io::Result<Vec<DreamReport>> {
    let content = fs::read_to_string(log_path)?;
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).map_err(io::Error::other))
        .collect()
}
