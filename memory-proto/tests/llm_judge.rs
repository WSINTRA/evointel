//! Hermetic tests for the live judge: [`LlmJudge`] over a
//! [`RecordedClient`], so every behavior — parse, retry, mapping,
//! create-vs-link, phantom endpoints, embed failures — is proven
//! without a network. The last test closes the circle: a full
//! `run_dream` with the LLM judge, where the model's hallucinated
//! edge is named by the dangling-ref fence while the note's good
//! content still lands.

use chrono::{TimeZone, Utc};
use memory_proto::db::MemoryDb;
use memory_proto::dreamer::{run_dream, DreamSpec, Judge, MechanicalSermon, NoDelivery};
use memory_proto::llm::{LlmError, LlmJudge, RecordedClient};
use memory_proto::mutations::insert_entity;
use memory_proto::nodes::{Entity, Note};
use memory_proto::observability::read_dream_reports;
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
        context: "llm-judge".to_string(),
    }
}

/// The rich canned judgment: every node type, every pillar, a dated
/// calendar, and one hallucinated edge endpoint (`perceived void`)
/// that must become a phantom the fence can name.
const CANNED: &str = r#"{
  "entities": [{"label": "Job Search"}, {"label": "anxiety"}],
  "facts": [{"predicate": "requires", "label": "job search requires patience"}],
  "events": [{"predicate": "applied_to", "status": "occurred", "label": "applications submitted"}],
  "memories": [{
    "predicate": "felt",
    "significance": "recurring emotional state",
    "emotions": ["anxiety"],
    "reflection": "the void is silence talking back",
    "function": "feeling",
    "label": "job search anxiety"
  }],
  "times": [{"iso": "2026-11-03"}, {"iso": "2026-11"}],
  "edges": [
    {"pillar": "contains", "from": "note", "to": "job search"},
    {"pillar": "contains", "from": "note", "to": "anxiety"},
    {"pillar": "similar", "from": "job search", "to": "anxiety"},
    {"pillar": "has_property", "from": "job search", "to": "anxiety",
     "property_name": "emotional state", "property_value": "anxious"},
    {"pillar": "leads_to", "from": "job search", "to": "perceived void"}
  ]
}"#;

fn recorded_with_canned() -> RecordedClient {
    let c = RecordedClient::new();
    c.push_chat(Ok(CANNED.to_string()));
    // Entities iterate in sorted-resolved order (anxiety, then job
    // search), then the fact, event, and memory in input order.
    c.push_embed(Ok(vec![
        one_hot(0),
        one_hot(1),
        one_hot(2),
        one_hot(3),
        one_hot(4),
    ]));
    c
}

fn judge_it(
    db: &MemoryDb,
    recorded: &RecordedClient,
    note: &Note,
) -> Result<memory_proto::dreamer::DreamJudgment, String> {
    let conn = db.connect().unwrap();
    let judge = LlmJudge::new(Box::new(recorded.clone()));
    judge.judge(&conn, note)
}

#[test]
fn maps_a_canned_judgment_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    let recorded = recorded_with_canned();
    let judgment = judge_it(&db, &recorded, &test_note()).unwrap();

    // The seam: the note went out as the user message, schema attached.
    let calls = recorded.chat_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0[1].content, "I am anxious about my job search.");
    assert!(calls[0]
        .1
        .as_ref()
        .is_some_and(|s| s["properties"].is_object()));

    // One batch of meaning, in extraction order.
    assert_eq!(
        recorded.embed_calls()[0],
        vec![
            "anxiety",
            "Job Search",
            "job search requires patience",
            "applications submitted",
            "job search anxiety",
        ]
    );

    // Entities: deterministic ids, resolved labels, embeddings by order.
    let job = judgment
        .entities
        .iter()
        .find(|e| e.label_resolved == "job search")
        .expect("job search entity");
    assert_eq!(job.id, "e-job-search");
    assert_eq!(job.label, "Job Search");
    assert_eq!(job.label_embedding, Some(one_hot(1)));
    let anxiety = judgment
        .entities
        .iter()
        .find(|e| e.label_resolved == "anxiety")
        .expect("anxiety entity");
    assert_eq!(anxiety.id, "e-anxiety");
    assert_eq!(anxiety.label_embedding, Some(one_hot(0)));

    // Content nodes: mechanics filled, provenance in source and context.
    assert_eq!(judgment.facts[0].id, "f-job-search-requires-patience");
    assert_eq!(judgment.facts[0].source, "n-1");
    assert_eq!(judgment.facts[0].label_embedding, Some(one_hot(2)));
    assert_eq!(judgment.events[0].id, "ev-applications-submitted");
    assert_eq!(judgment.events[0].status, "occurred");
    assert_eq!(judgment.events[0].label_embedding, Some(one_hot(3)));
    assert_eq!(judgment.memories[0].id, "m-job-search-anxiety");
    assert_eq!(judgment.memories[0].emotions, vec!["anxiety".to_string()]);
    assert_eq!(
        judgment.memories[0].reflection,
        "the void is silence talking back"
    );
    assert_eq!(
        judgment.memories[0].function_tag,
        Some("feeling".to_string()),
        "the extractor's attribution lands on the memory"
    );
    assert_eq!(judgment.memories[0].run_id, None, "the dreamer stamps it");
    assert_eq!(judgment.memories[0].label_embedding, Some(one_hot(4)));

    // The calendar: both dates plus the parent chain, wired.
    assert_eq!(judgment.times.len(), 3);
    let day = judgment
        .times
        .iter()
        .find(|t| t.id == "t-2026-11-03")
        .unwrap();
    assert_eq!(day.granularity, "day");
    assert_eq!(day.label, "November 3, 2026");
    assert_eq!(day.starts_at, day.ends_at.unwrap());
    let month = judgment.times.iter().find(|t| t.id == "t-2026-11").unwrap();
    assert_eq!(month.granularity, "month");
    assert_eq!(month.label, "November 2026");
    let year = judgment.times.iter().find(|t| t.id == "t-2026").unwrap();
    assert_eq!(year.granularity, "year");
    assert_eq!(year.label, "2026");
    assert!(judgment
        .hierarchy
        .contains(&("t-2026".to_string(), "t-2026-11".to_string())));
    assert!(judgment
        .hierarchy
        .contains(&("t-2026-11".to_string(), "t-2026-11-03".to_string())));

    // Pillar edges: note endpoints, property fields, deterministic ids.
    assert_eq!(judgment.contains.len(), 2);
    let (from, to, props) = &judgment.contains[0];
    assert_eq!((from.id.as_str(), from.label.as_str()), ("n-1", "Note"));
    assert_eq!(
        (to.id.as_str(), to.label.as_str()),
        ("e-job-search", "Entity")
    );
    assert_eq!(props.id, "c-n-1-job-search");
    assert_eq!(props.label_resolved, "note mentions concept");
    assert_eq!(judgment.similar.len(), 1);
    assert_eq!(judgment.similar[0].2.id, "sim-job-search-anxiety");
    assert_eq!(judgment.has_property.len(), 1);
    assert_eq!(judgment.has_property[0].2.property_name, "emotional state");
    assert_eq!(judgment.has_property[0].2.property_value, "anxious");
    assert_eq!(
        judgment.has_property[0].2.label_resolved,
        "emotional state = anxious"
    );

    // The hallucination becomes a named phantom, not a silent drop.
    assert_eq!(judgment.leads_to.len(), 1);
    let (_, phantom, props) = &judgment.leads_to[0];
    assert_eq!(phantom.id, "e-perceived-void");
    assert_eq!(phantom.label, "Entity");
    assert_eq!(props.label_resolved, "job search leads to perceived void");
}

#[test]
fn retries_once_on_garbage_then_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    let recorded = RecordedClient::new();
    recorded.push_chat(Ok("definitely not json".to_string()));
    recorded.push_chat(Ok(CANNED.to_string()));
    recorded.push_embed(Ok(vec![
        one_hot(0),
        one_hot(1),
        one_hot(2),
        one_hot(3),
        one_hot(4),
    ]));

    let judgment = judge_it(&db, &recorded, &test_note()).unwrap();
    assert!(
        !judgment.entities.is_empty(),
        "the retry recovered a full judgment"
    );

    // Two chats; the second carries the bad reply and the correction.
    let calls = recorded.chat_calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].0[2].content, "definitely not json");
    assert!(calls[1].0[3].content.contains("not valid"));
}

#[test]
fn fails_cleanly_after_two_bad_parses() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    let recorded = RecordedClient::new();
    recorded.push_chat(Ok("garbage one".to_string()));
    recorded.push_chat(Ok("garbage two".to_string()));

    let err = judge_it(&db, &recorded, &test_note()).unwrap_err();
    assert!(err.contains("unparseable judgment after retry"), "{err}");
    assert_eq!(recorded.chat_call_count(), 2, "exactly one retry");
}

#[test]
fn reuses_existing_entities_by_label() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    {
        // A pre-existing "job search" under a foreign id.
        let conn = db.connect().unwrap();
        insert_entity(
            &conn,
            &Entity {
                id: "e-existing".to_string(),
                label: "job search".to_string(),
                label_resolved: "job search".to_string(),
                label_embedding: Some(one_hot(9)),
                learned_at: Utc::now(),
                expire_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                layer: 1,
                context: "seed".to_string(),
            },
        )
        .unwrap();
    }
    let recorded = recorded_with_canned();
    let judgment = judge_it(&db, &recorded, &test_note()).unwrap();

    let job = judgment
        .entities
        .iter()
        .find(|e| e.label_resolved == "job search")
        .unwrap();
    assert_eq!(job.id, "e-existing", "create-vs-link keeps the old id");
    let anxiety = judgment
        .entities
        .iter()
        .find(|e| e.label_resolved == "anxiety")
        .unwrap();
    assert_eq!(anxiety.id, "e-anxiety", "fresh labels get slug ids");
    // And the contains edge points at the existing node.
    assert!(judgment
        .contains
        .iter()
        .any(|(from, _, _)| from.id == "n-1" || from.id == "e-existing"));
}

#[test]
fn propagates_embed_failures() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    let recorded = RecordedClient::new();
    recorded.push_chat(Ok(CANNED.to_string()));
    recorded.push_embed(Err(LlmError::Malformed("embedder down".to_string())));

    let err = judge_it(&db, &recorded, &test_note()).unwrap_err();
    assert!(err.contains("embedder down"), "{err}");
}

#[test]
fn an_unknown_function_refuses_the_whole_judgment() {
    // The function attribution is W's evidence — a broken instrument
    // refuses the judgment (recorded as a run fault), never silently
    // defaults to a pole.
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    let recorded = RecordedClient::new();
    recorded.push_chat(Ok(
        CANNED.replace("\"function\": \"feeling\"", "\"function\": \"valorating\"")
    ));
    recorded.push_embed(Ok(vec![one_hot(0); 6]));

    let err = judge_it(&db, &recorded, &test_note()).unwrap_err();
    assert!(err.contains("unknown function"), "{err}");
    assert!(err.contains("valorating"), "{err}");
}

#[test]
fn empty_extraction_embeds_nothing_and_says_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    let recorded = RecordedClient::new();
    recorded.push_chat(Ok(
        r#"{"entities":[],"facts":[],"events":[],"memories":[],"times":[],"edges":[]}"#.to_string(),
    ));

    let judgment = judge_it(&db, &recorded, &test_note()).unwrap();
    assert!(judgment.entities.is_empty(), "nothing was extracted");
    assert!(judgment.times.is_empty(), "no calendar was implied");
    assert_eq!(recorded.embed_call_count(), 0, "nothing to embed");
}

#[test]
fn tokenization_drift_converges_on_one_concept() {
    // The model writes "job_search" some nights and "job search" on
    // others — one concept, one node, one edge id. And a time
    // referenced by its exact ISO string resolves to the Time node.
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    let recorded = RecordedClient::new();
    recorded.push_chat(Ok(r#"{
          "entities": [{"label": "job_search"}],
          "facts": [], "events": [], "memories": [],
          "times": [{"iso": "2026-11-03"}],
          "edges": [
            {"pillar": "contains", "from": "note", "to": "job search"},
            {"pillar": "contains", "from": "note", "to": "job_search"},
            {"pillar": "contains", "from": "note", "to": "2026-11-03"}
          ]
        }"#
    .to_string()));
    recorded.push_embed(Ok(vec![one_hot(0)]));

    let judgment = judge_it(&db, &recorded, &test_note()).unwrap();

    // One concept under the canonical key, original label preserved.
    assert_eq!(judgment.entities.len(), 1);
    assert_eq!(judgment.entities[0].id, "e-job-search");
    assert_eq!(judgment.entities[0].label, "job_search");
    assert_eq!(judgment.entities[0].label_resolved, "job search");

    // Both spellings resolve to the same node and the same edge id.
    assert_eq!(judgment.contains.len(), 3);
    for (from, to, props) in &judgment.contains[..2] {
        assert_eq!(to.id, "e-job-search");
        assert_eq!(props.id, "c-n-1-job-search");
        assert_eq!(from.id, "n-1");
    }

    // The exact ISO form is not prose — it lands on the Time node.
    let (_, time_ref, _) = &judgment.contains[2];
    assert_eq!(time_ref.id, "t-2026-11-03");
    assert_eq!(time_ref.label, "Time");
}

#[test]
fn run_dream_with_llm_judge_names_phantoms_and_lands_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_test_db(&dir.path().join("t.db"));
    let outbox = dir.path().join("outbox");
    let log = dir.path().join("dream-log.jsonl");

    let recorded = RecordedClient::new();
    recorded.push_chat(Ok(CANNED.to_string()));
    // Sorted entity order: anxiety at one_hot(0), job search at one_hot(1).
    recorded.push_embed(Ok(vec![
        one_hot(0),
        one_hot(1),
        one_hot(2),
        one_hot(3),
        one_hot(4),
    ]));

    let conn = db.connect().unwrap();
    let judge = LlmJudge::new(Box::new(recorded.clone()));
    let spec = DreamSpec {
        run_id: "run-1".to_string(),
        queue: vec![test_note()],
        anchor_embedding: one_hot(1),
        drift_windows: None,
        outbox: outbox.clone(),
        dream_log: log.clone(),
    };
    let report = run_dream(&conn, &judge, &MechanicalSermon, &NoDelivery, &spec);

    // The wrong-shaped claim is named before the database sees it; the
    // good content lands anyway. (The phantom endpoint itself is named
    // at the judgment level in maps_a_canned_judgment_end_to_end.)
    assert_eq!(report.errors.len(), 1, "errors: {:?}", report.errors);
    assert!(
        report.errors[0].contains("illegal label 'Entity' for FROM_LeadsTo: e-job-search"),
        "errors: {:?}",
        report.errors
    );
    assert_eq!(report.notes_in, 1);
    // note + 2 entities + fact + event + memory + 3 times = 9 node
    // writes, + 2 contains + similar + has_property = 4 edge writes,
    // + 2 hierarchy wires = 15 write ops.
    assert_eq!(
        report.nodes_written, 15,
        "nodes_written: {}",
        report.nodes_written
    );
    assert!(report.message_written);
    assert!(report.shelf_ok);

    // The sermon is grounded in the real note via the anchor.
    let msg: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(outbox.join("run-1-sermon.json")).unwrap())
            .unwrap();
    assert_eq!(msg["citations"][0]["note_id"], "n-1");

    // The ledger tells the story.
    let ledger = read_dream_reports(&log).unwrap();
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].run_id, "run-1");
}
