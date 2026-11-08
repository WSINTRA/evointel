use chrono::{DateTime, Utc};
use memory_proto::observability::{
    DreamReport, append_dream_report, is_nightmare, read_dream_reports,
};

fn ts(t: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(t, 0).unwrap()
}

fn good_report() -> DreamReport {
    DreamReport {
        run_id: "run-good".to_string(),
        started_at: ts(1_700_000_000),
        finished_at: Some(ts(1_700_000_900)),
        notes_in: 3,
        nodes_written: 14,
        nodes_added: 12,
        edges_added: 5,
        errors: vec![],
        message_written: true,
        shelf_ok: true,
        similarity_to_prev: Some(0.93),
        self_pass: None,
    }
}

/// Each field that can be corrupted independently gets its own test, so
/// a failure names the invariant that broke.

#[test]
fn good_dream_is_not_a_nightmare() {
    assert!(!is_nightmare(&good_report(), 0.5));
}

#[test]
fn unfinished_dream_is_a_nightmare() {
    let mut r = good_report();
    r.finished_at = None;
    assert!(is_nightmare(&r, 0.5));
}

#[test]
fn errors_make_a_nightmare() {
    let mut r = good_report();
    r.errors = vec!["write path failed: buffer manager exception".to_string()];
    assert!(is_nightmare(&r, 0.5));
}

#[test]
fn consuming_notes_without_writing_is_a_nightmare() {
    let mut r = good_report();
    r.notes_in = 3;
    r.nodes_written = 0;
    assert!(is_nightmare(&r, 0.5), "evidence in, no model out");
}

#[test]
fn an_honest_rerun_is_not_a_nightmare() {
    // Idempotent replay: every MERGE re-executes (nodes_written > 0) but
    // observes nothing new (nodes_added == 0). This is the recovery
    // path working, not a failure.
    let mut r = good_report();
    r.notes_in = 3;
    r.nodes_written = 14;
    r.nodes_added = 0;
    r.edges_added = 0;
    assert!(!is_nightmare(&r, 0.5));
}

#[test]
fn a_quiet_day_is_not_a_nightmare() {
    // Empty queue: no notes, nothing written, no sermon (silence is the
    // correct output), shelf vacuously true.
    let mut r = good_report();
    r.notes_in = 0;
    r.nodes_written = 0;
    r.nodes_added = 0;
    r.edges_added = 0;
    r.message_written = false;
    r.similarity_to_prev = Some(1.0);
    assert!(!is_nightmare(&r, 0.5));
}

#[test]
fn failed_shelf_check_is_a_nightmare() {
    let mut r = good_report();
    r.shelf_ok = false;
    assert!(is_nightmare(&r, 0.5));
}

#[test]
fn structural_shock_is_a_nightmare() {
    let mut r = good_report();
    r.similarity_to_prev = Some(0.1);
    assert!(is_nightmare(&r, 0.5));
}

#[test]
fn shock_threshold_is_the_callers_policy() {
    // Same report, different floors: the daemon owns the policy.
    let mut r = good_report();
    r.similarity_to_prev = Some(0.4);
    assert!(is_nightmare(&r, 0.5));
    assert!(!is_nightmare(&r, 0.3));
}

#[test]
fn unmeasured_similarity_is_not_a_nightmare() {
    // "Can't tell" must not be treated as "went wrong".
    let mut r = good_report();
    r.similarity_to_prev = None;
    assert!(!is_nightmare(&r, 0.5));
}

#[test]
fn dream_ledger_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("logs").join("dreams.jsonl"); // parent dir missing on purpose

    let mut nightmare = good_report();
    nightmare.run_id = "run-bad".to_string();
    nightmare.errors = vec!["boom".to_string()];

    append_dream_report(&log, &good_report()).unwrap();
    append_dream_report(&log, &nightmare).unwrap();

    let read_back = read_dream_reports(&log).unwrap();
    assert_eq!(read_back.len(), 2);
    assert_eq!(read_back[0], good_report());
    assert_eq!(read_back[1], nightmare);

    // The ledger is append-only: a third append grows the file by a line.
    append_dream_report(&log, &good_report()).unwrap();
    assert_eq!(read_dream_reports(&log).unwrap().len(), 3);
}
