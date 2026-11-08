//! Opt-in live probes against the real local servers. Skipped unless
//! `WUNICORM_LLM_LIVE=1` is set, so hermetic and CI runs never touch
//! the network. When it is set, these tests are the honest check that
//! the client, the sampling recipe, and the schema still work against
//! the servers that actually run on this machine.
//!
//! The chat probe is SLOW: thinking is on and the budget is 80k
//! tokens, so a judgment can run for many minutes. Run it manually,
//! not as part of every suite.

use memory_proto::llm::{
    ChatMessage, LlmClient, LlmConfig, UreqClient, extract_prompt, extract_schema,
};
use memory_proto::schema::EMBEDDING_DIM;

fn live() -> Option<UreqClient> {
    if std::env::var("WUNICORM_LLM_LIVE").is_err() {
        return None;
    }
    Some(UreqClient::new(LlmConfig::from_env()))
}

#[test]
fn live_chat_extracts_under_the_full_schema() {
    let Some(client) = live() else {
        return;
    };
    let schema = extract_schema().unwrap();
    let prompt = extract_prompt().unwrap();
    let note =
        "I am anxious about my job search. Every application feels like it disappears into a void.";
    let msg = client
        .chat(
            &[ChatMessage::system(&prompt), ChatMessage::user(note)],
            Some(&schema),
        )
        .unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&msg).unwrap_or_else(|e| panic!("not JSON ({e}): {msg}"));
    for key in ["entities", "facts", "events", "memories", "times", "edges"] {
        assert!(parsed[key].is_array(), "missing array {key} in {msg}");
    }
    assert!(
        !parsed["entities"].as_array().unwrap().is_empty(),
        "the note has entities: {msg}"
    );
}

#[test]
fn live_embed_returns_schema_width_vectors() {
    let Some(client) = live() else {
        return;
    };
    let out = client
        .embed(&["hello world", "job search anxiety"])
        .unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].len(), EMBEDDING_DIM);
    assert_ne!(out[0], out[1], "distinct texts must not embed identically");
}
