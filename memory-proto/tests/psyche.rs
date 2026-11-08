//! Psyche read tests — positions, rigidity, drift, and the W
//! decomposition, from fixtures with known geometry.

use chrono::{TimeZone, Utc};
use memory_proto::axes::Axis;
use memory_proto::db::MemoryDb;
use memory_proto::mutations::{
    insert_dream_run, insert_note, insert_self_state, set_note_coordinates,
};
use memory_proto::nodes::{DreamRun, Note, SelfState};
use memory_proto::queries::{
    evolve_wunicorm, function_drift, rigidity, user_position, wunicorm_observed,
};
use memory_proto::schema::{self, EMBEDDING_DIM};

fn temp_db() -> (tempfile::TempDir, MemoryDb) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let db = MemoryDb::open(&path, EMBEDDING_DIM).unwrap();
    {
        let conn = db.connect().unwrap();
        schema::init(&conn).unwrap();
    }
    (dir, db)
}

fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 9, 0, 0).single().unwrap()
}

/// A date `n` days after Nov 1 2026 — window math without calendar
/// arithmetic panics.
fn day(n: u32) -> chrono::DateTime<Utc> {
    ts(2026, 11, 1) + chrono::Duration::days(i64::from(n))
}

fn note_at(conn: &lbug::Connection, id: &str, day: u32, judgment: f64, perception: f64) {
    insert_note(
        conn,
        &Note {
            id: id.to_string(),
            content: format!("note {id}"),
            content_embedding: None,
            created_at: ts(2026, 11, day),
            updated_at: ts(2026, 11, day),
            layer: 0,
            context: "test".to_string(),
        },
    )
    .unwrap();
    set_note_coordinates(conn, id, judgment, perception).unwrap();
}

fn state_at(
    conn: &lbug::Connection,
    id: &str,
    subject: &str,
    day_offset: u32,
    position: (f64, f64),
) {
    let start = day(day_offset);
    let end = start + chrono::Duration::days(27);
    insert_self_state(
        conn,
        &SelfState {
            id: id.to_string(),
            subject: subject.to_string(),
            window_start: start,
            window_end: end,
            judgment_coord: position.0,
            perception_coord: position.1,
            judgment_core: position.0,
            perception_core: position.1,
            judgment_comp: 0.0,
            perception_comp: 0.0,
            rigidity_judgment: 0.0,
            rigidity_perception: 0.0,
            evidence_kept: 1,
            evidence_dropped: 0,
            learned_at: end,
            created_at: end,
            updated_at: end,
            layer: 4,
            context: "test".to_string(),
        },
    )
    .unwrap();
}

fn run_with(conn: &lbug::Connection, id: &str, counts: (i64, i64, i64, i64, i64)) {
    insert_dream_run(
        conn,
        &DreamRun {
            id: id.to_string(),
            considered: vec![],
            dropped: vec![],
            kept_t: counts.0,
            kept_f: counts.1,
            kept_s: counts.2,
            kept_n: counts.3,
            dropped_total: counts.4,
            started_at: ts(2026, 11, 30),
            finished_at: ts(2026, 11, 30),
            created_at: ts(2026, 11, 30),
            updated_at: ts(2026, 11, 30),
            layer: 3,
            context: "test".to_string(),
        },
    )
    .unwrap();
}

// ─── user_position ───────────────────────────────────────────────────

#[test]
fn user_position_is_the_windowed_centroid() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    note_at(&conn, "n-1", 3, 0.2, -0.4);
    note_at(&conn, "n-2", 5, 0.6, -0.2);
    note_at(&conn, "n-out", 20, 9.9, 9.9); // measured, but outside every window below

    // A window holding only n-1.
    let tight = (ts(2026, 11, 2), ts(2026, 11, 4));
    let pos = user_position(&conn, tight).unwrap().unwrap();
    assert_eq!(pos, (0.2, -0.4));

    // Both in-cluster notes average.
    let wide = (ts(2026, 11, 1), ts(2026, 11, 6));
    let pos = user_position(&conn, wide).unwrap().unwrap();
    assert!((pos.0 - 0.4).abs() < 1e-6, "got {}", pos.0);
    assert!((pos.1 - (-0.3)).abs() < 1e-6, "got {}", pos.1);
}

#[test]
fn user_position_unmeasurable_window_is_none() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    note_at(&conn, "n-1", 3, 0.2, -0.4);
    let empty = (ts(2026, 12, 1), ts(2026, 12, 31));
    assert!(user_position(&conn, empty).unwrap().is_none());
}

// ─── wunicorm_observed ───────────────────────────────────────────────

#[test]
fn wunicorm_observed_reads_the_choice_ledger() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    run_with(&conn, "run-1", (3, 1, 0, 1, 0));
    let (j, p) = wunicorm_observed(&conn, "run-1").unwrap().unwrap();
    assert!((j - 0.5).abs() < 1e-6, "3T/1F → +0.5, got {j}");
    assert!(
        (p - (-1.0)).abs() < 1e-6,
        "0S/1N → the full ratio −1.0, got {p}"
    );

    assert!(wunicorm_observed(&conn, "run-missing").unwrap().is_none());
}

#[test]
fn drops_dilute_but_never_push() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    // One thinking keep, nine drops: heavy dilution — near zero, not at
    // the pole. A drop shrinks confidence; it never pushes position.
    run_with(&conn, "run-dilute", (1, 0, 0, 0, 9));
    let (j, _) = wunicorm_observed(&conn, "run-dilute").unwrap().unwrap();
    let dilution = 0.5 * 9.0;
    let expected = 1.0 / (1.0 + dilution);
    assert!((j - expected).abs() < 1e-6, "got {j}, expected {expected}");
    assert!(j < 0.2, "diluted far toward zero, got {j}");

    // No evidence at all: zero, the honest no-signal.
    run_with(&conn, "run-empty", (0, 0, 0, 0, 0));
    let pos = wunicorm_observed(&conn, "run-empty").unwrap().unwrap();
    assert_eq!(pos, (0.0, 0.0));
}

// ─── rigidity ────────────────────────────────────────────────────────

#[test]
fn rigidity_reads_straightness_not_direction() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    // A straight line, either direction: rigid.
    state_at(&conn, "s-1", "user", 1, (0.0, 0.0));
    state_at(&conn, "s-2", "user", 28, (0.5, 0.0));
    state_at(&conn, "s-3", "user", 56, (1.0, 0.0));
    let r = rigidity(&conn, "user", Axis::Judgment, 3).unwrap();
    assert!(r > 0.99, "a straight line is rigid, got {r}");

    // Direction never matters: a straight line the other way is just as
    // rigid.
    state_at(&conn, "t-1", "wunicorm", 1, (1.0, 0.0));
    state_at(&conn, "t-2", "wunicorm", 28, (0.5, 0.0));
    state_at(&conn, "t-3", "wunicorm", 56, (0.0, 0.0));
    let r = rigidity(&conn, "wunicorm", Axis::Judgment, 3).unwrap();
    assert!(r > 0.99, "rigidity is not direction, got {r}");

    // A zigzag on perception: wandering, not stuck.
    state_at(&conn, "z-1", "user", 84, (0.0, 0.0));
    state_at(&conn, "z-2", "user", 112, (0.0, 0.8));
    state_at(&conn, "z-3", "user", 140, (0.0, 0.0));
    let r = rigidity(&conn, "user", Axis::Perception, 3).unwrap();
    assert!(r < 0.05, "a zigzag is not stuck, got {r}");
    // And the judgment axis of the same zigzag states is flat —
    // unmoved, which reads as 0, not stuck.
    let r = rigidity(&conn, "user", Axis::Judgment, 3).unwrap();
    assert!(r < 0.05, "flat is unmeasured movement, got {r}");
}

#[test]
fn rigidity_needs_a_trajectory() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();
    assert_eq!(rigidity(&conn, "user", Axis::Judgment, 4).unwrap(), 0.0);
    state_at(&conn, "s-1", "user", 1, (0.5, 0.5));
    assert_eq!(rigidity(&conn, "user", Axis::Judgment, 4).unwrap(), 0.0);
}

// ─── function_drift ──────────────────────────────────────────────────

#[test]
fn function_drift_reads_the_two_latest_states() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    assert!(function_drift(&conn, "user").unwrap().is_none());
    state_at(&conn, "s-1", "user", 1, (0.2, -0.1));
    assert!(function_drift(&conn, "user").unwrap().is_none());
    state_at(&conn, "s-2", "user", 28, (0.6, 0.4));
    let d = function_drift(&conn, "user").unwrap().unwrap();
    assert!((d.judgment - 0.4).abs() < 1e-6, "got {}", d.judgment);
    assert!((d.perception - 0.5).abs() < 1e-6, "got {}", d.perception);

    // A third, later state: drift is always the latest step.
    state_at(&conn, "s-3", "user", 56, (0.3, 0.6));
    let d = function_drift(&conn, "user").unwrap().unwrap();
    assert!((d.judgment - (-0.3)).abs() < 1e-6, "got {}", d.judgment);
}

// ─── evolve_wunicorm ─────────────────────────────────────────────────

#[test]
fn compensation_targets_dyad_balance_when_rigid() {
    // A rigid, one-sided user (judgment +0.8, rigidity 0.9): the fast
    // component leans against it — negative, nonzero, bounded.
    let d = evolve_wunicorm((0.0, 0.0), (0.0, 0.0), (0.8, 0.0), (0.9, 0.0));
    assert!(d.comp.0 < 0.0, "leans against the one-sided user");
    let expected = -0.25 * (0.9 - 0.7) * 0.8;
    assert!((d.comp.0 - expected).abs() < 1e-6, "got {}", d.comp.0);
    assert_eq!(d.comp.1, 0.0);
    assert_eq!(d.position.0, d.core.0 + d.comp.0);
}

#[test]
fn compensation_is_zero_below_the_rigidity_threshold() {
    // One-sided but not rigid (0.3 < 0.7): a stance that moves with
    // circumstance is healthy — no compensation at all.
    let d = evolve_wunicorm((0.2, 0.0), (0.1, 0.0), (0.8, -0.8), (0.3, 0.3));
    assert_eq!(d.comp, (0.0, 0.0));
    // The core still absorbs the observation, damped.
    assert!(
        (d.core.0 - (0.9 * 0.2 + 0.1 * 0.1)).abs() < 1e-6,
        "got {}",
        d.core.0
    );
}

#[test]
fn compensation_shrinks_with_a_balanced_user() {
    // Rigid, but the user sits near the origin: nothing to lean
    // against.
    let d = evolve_wunicorm((0.0, 0.0), (0.0, 0.0), (0.01, 0.0), (0.95, 0.95));
    assert!(d.comp.0.abs() < 0.01, "got {}", d.comp.0);
}

#[test]
fn core_damping_is_the_hysteresis() {
    // Months, not windows: 90% of the core survives one run.
    let d = evolve_wunicorm((0.5, -0.5), (1.0, 1.0), (0.0, 0.0), (0.0, 0.0));
    assert!((d.core.0 - 0.55).abs() < 1e-6, "got {}", d.core.0);
    assert!((d.core.1 - (-0.35)).abs() < 1e-6, "got {}", d.core.1);
    assert_eq!(d.position, d.core, "no rigidity → position is core");
}
