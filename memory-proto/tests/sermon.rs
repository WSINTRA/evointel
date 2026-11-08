//! Hermetic tests for the sermon's voice: the LLM writer over a
//! [`RecordedClient`], the mechanical floor, and the blend — LLM
//! prose above, mechanical citations below, grounding never
//! delegated to the model.

use chrono::{TimeZone, Utc};
use memory_proto::db::MemoryDb;
use memory_proto::dreamer::{DreamSpec, MechanicalSermon, SermonWriter, run_dream};
use memory_proto::llm::{LlmJudge, LlmSermonWriter, RecordedClient};
use memory_proto::message::{Citation, Insight};
use memory_proto::nodes::Note;
use memory_proto::schema::{self, EMBEDDING_DIM};
use memory_proto::{extensions, vector};

fn one_hot(idx: usize) -> Vec<f32> {
    let mut v = vec![0.0; EMBEDDING_DIM];
    v[idx] = 1.0;
    v
}

fn open_test_db(path: &std::path::Path) -> MemoryDb {
    let db = MemoryDb::open(path, EMBEDDING_DIM).unwrap();
    {
        let conn = db.connect().unwrap();
        extensions::load_extensions(&conn).unwrap();
        schema::init(&conn).unwrap();
        vector::init_vector_indexes(&conn).unwrap();
    }
    db
}

fn test_note() -> Note {
    Note {
        id: "n-1".to_string(),
        content: "I am anxious about my job search.".to_string(),
        content_embedding: None,
        created_at: Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).single().unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).single().unwrap(),
        layer: 0,
        context: "sermon-test".to_string(),
    }
}

fn sample_insights() -> Vec<Insight> {
    vec![Insight {
        summary: "1 notes now connect to 'job search'.".to_string(),
        concept: None,
        drift: None,
    }]
}

fn sample_citations() -> Vec<Citation> {
    vec![Citation {
        note_id: "n-1".to_string(),
        snippet: "I am anxious about my job search.".to_string(),
    }]
}

const PROSE: &str = "You have been carrying the search like a held breath — \
each application sent into silence, each silence read as a verdict. The anxiety \
is not noise; it is the part of you that knows how much this matters, standing \
at the threshold and refusing to pretend otherwise. The coming week asks for \
the other voice too: the one that plans, prepares, and walks through the door. \
Both belong to the crossing.";

#[test]
fn llm_writer_carries_the_material_and_returns_prose() {
    let recorded = RecordedClient::new();
    recorded.push_chat(Ok(PROSE.to_string()));
    let writer = LlmSermonWriter::new(Box::new(recorded.clone()));

    let body = writer
        .compose("job search", &sample_insights(), &sample_citations(), None)
        .unwrap();
    assert_eq!(body, PROSE, "prose passes through, trimmed");

    // The material traveled: system is the portrait prompt, user is
    // anchor + facts + the user's own words, and no grammar constrains
    // prose.
    let calls = recorded.chat_calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].0[0].content.contains("living portrait"));
    assert!(calls[0].0[1].content.contains("job search"));
    assert!(
        calls[0].0[1]
            .content
            .contains("anxious about my job search")
    );
    assert!(calls[0].1.is_none(), "prose is unconstrained");
}

#[test]
fn llm_writer_rejects_empty_prose() {
    let recorded = RecordedClient::new();
    recorded.push_chat(Ok("   ".to_string()));
    let writer = LlmSermonWriter::new(Box::new(recorded));

    let err = writer
        .compose("job search", &sample_insights(), &sample_citations(), None)
        .unwrap_err();
    assert!(err.contains("empty prose"), "{err}");
}

#[test]
fn mechanical_sermon_is_the_floor() {
    let body = MechanicalSermon
        .compose("job search", &sample_insights(), &sample_citations(), None)
        .unwrap();
    assert!(body.starts_with("Morning — a dream pass over your notes."));
    assert!(body.contains("Around 'job search':"));
    assert!(body.contains("- 1 notes now connect to 'job search'."));
    assert!(
        !body.contains("Grounded in"),
        "citations are the dreamer's job"
    );
}

/// The minimal live-style dream: one note, one judgment, one sermon —
/// judge and writer both over recorded clients.
struct DreamMinds {
    judge: LlmJudge,
    writer: LlmSermonWriter,
}

fn dream_minds(judgment: &str, prose: Result<String, memory_proto::llm::LlmError>) -> DreamMinds {
    let judge_rec = RecordedClient::new();
    judge_rec.push_chat(Ok(judgment.to_string()));
    judge_rec.push_embed(Ok(vec![one_hot(0)]));
    let writer_rec = RecordedClient::new();
    writer_rec.push_chat(prose);
    DreamMinds {
        judge: LlmJudge::new(Box::new(judge_rec)),
        writer: LlmSermonWriter::new(Box::new(writer_rec)),
    }
}

fn spec(paths: (&std::path::Path, &std::path::Path)) -> DreamSpec {
    DreamSpec {
        run_id: "run-1".to_string(),
        queue: vec![test_note()],
        anchor_embedding: one_hot(0),
        drift_windows: None,
        outbox: paths.0.to_path_buf(),
        dream_log: paths.1.to_path_buf(),
    }
}

const CANNED: &str = r#"{
  "entities": [{"label": "job search"}],
  "facts": [], "events": [], "memories": [], "times": [],
  "edges": [{"pillar": "contains", "from": "note", "to": "job search"}]
}"#;

#[test]
fn run_dream_blends_llm_prose_with_mechanical_citations() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    let conn = db.connect().unwrap();
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("dream-log.jsonl");
    let minds = dream_minds(CANNED, Ok(PROSE.to_string()));

    let report = run_dream(&conn, &minds.judge, &minds.writer, &spec((&outbox, &log)));

    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    assert!(report.message_written);
    assert!(report.shelf_ok);

    let msg: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(outbox.join("run-1-sermon.json")).unwrap())
            .unwrap();
    let body = msg["body"].as_str().expect("body");
    assert!(
        body.contains("carrying the search"),
        "the portrait leads: {body}"
    );
    assert!(
        body.contains("Grounded in your notes:\n- [n-1]"),
        "the grounding block follows, formatted: {body}"
    );
    assert!(
        body.contains("[n-1]"),
        "the citation is the real note: {body}"
    );
    assert!(
        body.contains("I am anxious about my job search"),
        "the citation quotes the note: {body}"
    );
}

#[test]
fn sermon_failure_falls_back_to_the_floor() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    let conn = db.connect().unwrap();
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("dream-log.jsonl");
    let minds = dream_minds(
        CANNED,
        Err(memory_proto::llm::LlmError::Malformed(
            "writer server down".to_string(),
        )),
    );

    let report = run_dream(&conn, &minds.judge, &minds.writer, &spec((&outbox, &log)));

    // The failure is recorded, the sermon still ships, still grounded.
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("sermon writer failed")),
        "errors: {:?}",
        report.errors
    );
    assert!(report.message_written);
    assert!(report.shelf_ok);
    let msg: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(outbox.join("run-1-sermon.json")).unwrap())
            .unwrap();
    let body = msg["body"].as_str().expect("body");
    assert!(
        body.starts_with("Morning — a dream pass"),
        "the floor speaks when the voice cannot: {body}"
    );
    assert!(body.contains("[n-1]"), "grounding survives the fallback");
}

// ─── The sermon speaks from the self ─────────────────────────────────

fn sample_self_reading() -> memory_proto::dreamer::SelfReading {
    memory_proto::dreamer::SelfReading {
        user_movement: Some("your attention moved toward thinking".to_string()),
        wunicorm_lean: Some("the reading leaned feeling-ward".to_string()),
        tension: None,
    }
}

#[test]
fn the_writer_receives_the_self_block_without_numbers() {
    let recorded = RecordedClient::new();
    recorded.push_chat(Ok(PROSE.to_string()));
    let writer = LlmSermonWriter::new(Box::new(recorded.clone()));
    let reading = sample_self_reading();

    writer
        .compose(
            "job search",
            &sample_insights(),
            &sample_citations(),
            Some(&reading),
        )
        .unwrap();

    let calls = recorded.chat_calls();
    assert_eq!(calls.len(), 1);
    let material = &calls[0].0[1].content;
    // The self block traveled — movement language, tension, lean.
    assert!(material.contains("self"), "the block is named: {material}");
    assert!(
        material.contains("your attention moved toward thinking"),
        "{material}"
    );
    assert!(
        material.contains("the reading leaned feeling-ward"),
        "{material}"
    );
    // The strip is enforced at construction, then tested: the self
    // block carries no digits.
    let self_json = material
        .split("\"self\":")
        .nth(1)
        .expect("a self block exists");
    let self_json = self_json.split('}').next().unwrap_or(self_json);
    assert!(
        !self_json.chars().any(char::is_numeric),
        "numbers belong to the ledger, not the voice: {self_json}"
    );
}

#[test]
fn the_floor_speaks_the_self_when_the_voice_fails() {
    let reading = sample_self_reading();
    let body = MechanicalSermon
        .compose(
            "job search",
            &sample_insights(),
            &sample_citations(),
            Some(&reading),
        )
        .unwrap();
    assert!(
        body.contains("- the self: your attention moved toward thinking"),
        "the floor speaks what the dreamer measured: {body}"
    );
    assert!(body.contains("the reading leaned feeling-ward"), "{body}");

    // And stays silent when there is nothing to say.
    let bare = MechanicalSermon
        .compose("job search", &sample_insights(), &sample_citations(), None)
        .unwrap();
    assert!(!bare.contains("the self:"), "{bare}");
}
