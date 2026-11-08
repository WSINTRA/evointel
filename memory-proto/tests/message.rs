use std::fs;

use chrono::DateTime;
use memory_proto::message::{shelf_check, write_message, Citation, Insight, Message, MessageKind};
use memory_proto::mutations::NodeRef;
use memory_proto::queries::Drift;

fn sample_message(run_id: &str) -> Message {
    Message {
        id: format!("msg-{run_id}"),
        run_id: run_id.to_string(),
        kind: MessageKind::Sermon,
        generated_at: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
        insights: vec![Insight {
            summary: "Your job search is shifting from anxious to strategic.".to_string(),
            concept: Some(NodeRef::new("ent-jobs", "Entity")),
            drift: Some(Drift {
                displacement: 1.0,
                new_connections: 2,
                dropped_connections: 2,
            }),
        }],
        citations: vec![Citation {
            note_id: "note-1".to_string(),
            snippet: "applied to five companies today, feeling calm about it".to_string(),
        }],
        body: "Morning. Three things happened around your job search...".to_string(),
    }
}

#[test]
fn write_then_shelf_check_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let outbox = dir.path().join("outbox"); // deliberately does not exist yet

    let msg = sample_message("run-1");
    let path = write_message(&msg, &outbox).unwrap();
    assert!(path.exists(), "artifact must land on disk");
    assert_eq!(
        path.file_name().unwrap(),
        "run-1-sermon.json",
        "deterministic file name"
    );

    // The shelf check passes for the run that wrote it.
    shelf_check(&path, "run-1").unwrap();

    // And the artifact round-trips losslessly.
    let read_back: Message = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(read_back, msg);
}

#[test]
fn rewrite_same_run_overwrites_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let outbox = dir.path().join("outbox");

    let mut msg = sample_message("run-1");
    write_message(&msg, &outbox).unwrap();
    msg.body = "rewritten body".to_string();
    write_message(&msg, &outbox).unwrap();

    let files: Vec<_> = fs::read_dir(&outbox).unwrap().collect();
    assert_eq!(files.len(), 1, "a re-run must not duplicate artifacts");
    let read_back: Message =
        serde_json::from_str(&fs::read_to_string(outbox.join("run-1-sermon.json")).unwrap())
            .unwrap();
    assert_eq!(read_back.body, "rewritten body");
}

#[test]
fn shelf_check_rejects_wrong_run() {
    let dir = tempfile::tempdir().unwrap();
    let outbox = dir.path().join("outbox");
    let path = write_message(&sample_message("run-1"), &outbox).unwrap();

    let err = shelf_check(&path, "run-2").unwrap_err();
    assert!(
        err.contains("run-1"),
        "reason should name the actual run: {err}"
    );
}

#[test]
fn shelf_check_rejects_missing_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nowhere").join("run-1-sermon.json");
    let err = shelf_check(&missing, "run-1").unwrap_err();
    assert!(err.contains("unreadable"), "reason: {err}");
}

#[test]
fn shelf_check_rejects_contentless_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    let outbox = dir.path().join("outbox");

    // No insights.
    let mut msg = sample_message("run-1");
    msg.insights = vec![];
    let path = write_message(&msg, &outbox).unwrap();
    assert!(shelf_check(&path, "run-1")
        .unwrap_err()
        .contains("no insights"));

    // No citations — evidence-free assertions must not ship.
    let mut msg = sample_message("run-2");
    msg.citations = vec![];
    let path = write_message(&msg, &outbox).unwrap();
    assert!(shelf_check(&path, "run-2")
        .unwrap_err()
        .contains("cites no source"));

    // Empty body.
    let mut msg = sample_message("run-3");
    msg.body = "   ".to_string();
    let path = write_message(&msg, &outbox).unwrap();
    assert!(shelf_check(&path, "run-3")
        .unwrap_err()
        .contains("body is empty"));
}

#[test]
fn every_message_kind_gets_its_own_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let outbox = dir.path().join("outbox");

    for (kind, name) in [
        (MessageKind::Sermon, "run-1-sermon.json"),
        (MessageKind::Notification, "run-1-notification.json"),
        (MessageKind::AttentionCard, "run-1-attention_card.json"),
    ] {
        let mut msg = sample_message("run-1");
        msg.kind = kind;
        let path = write_message(&msg, &outbox).unwrap();
        assert_eq!(path.file_name().unwrap(), name);
    }

    let files: Vec<_> = fs::read_dir(&outbox).unwrap().collect();
    assert_eq!(files.len(), 3);
}
