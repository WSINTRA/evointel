//! The OpenAI-compatible client layer — the dreamer's ears and mouth.
//!
//! One dialect: the OpenAI-compatible REST API spoken by `llama.cpp`
//! and most local-model servers. Two endpoints: `/v1/chat/completions`
//! for judgment, `/v1/embeddings` for meaning. Sync on purpose — the
//! dreamer is a nightly batch pass, not a request path, and the whole
//! memory layer below is synchronous.
//!
//! The [`LlmClient`] trait is the seam: [`UreqClient`] talks to real
//! servers, [`RecordedClient`] replays canned responses so tests stay
//! hermetic. On top of it, [`LlmJudge`] implements the dreamer's
//! `Judge` trait — the extraction brain — and [`LlmSermonWriter`]
//! implements `SermonWriter` — the message's living voice. Prompts
//! are data, not code: [`load_prompt`] reads `prompts/<name>` at
//! runtime when present (iterate without recompiling) and falls back
//! to the copy baked in at build time.

mod judge;
mod sermon;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ureq::Agent;

use crate::schema::EMBEDDING_DIM;

pub use judge::{LlmJudge, Pillar, RawJudgment};
pub use sermon::LlmSermonWriter;

/// Everything the client layer needs to know about the outside world.
///
/// The defaults describe this machine's local servers: the 27B chat
/// model on `:9931`, EmbeddingGemma-2 on `:9932`. The sampling
/// defaults are the verified recipe — greedy decoding on the
/// quantized 27B degenerates into repetition loops under grammar
/// constraints, and `repeat_penalty` is a `llama.cpp` extension that
/// breaks them.
///
/// Thinking is ON: the nightly dream pays for extraction quality with
/// wall-clock (a thinking-heavy judgment runs many minutes at
/// ~18 tok/s), and the 80k-token budget exists so chain-of-thought
/// cannot starve the content again — with 1500 tokens it did, and
/// the completion came back empty.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub chat_base_url: String,
    pub chat_model: String,
    pub embed_base_url: String,
    pub embed_model: String,
    pub temperature: f64,
    pub top_p: f64,
    pub repeat_penalty: f64,
    pub max_tokens: u32,
    pub enable_thinking: bool,
    pub timeout: Duration,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            chat_base_url: "http://localhost:9931/v1".to_string(),
            chat_model: "unsloth/Qwen3.8-27B-GGUF:UD-Q8_K_XL".to_string(),
            embed_base_url: "http://localhost:9932/v1".to_string(),
            embed_model: "unsloth/embeddinggemma-2-GGUF:Q8_0".to_string(),
            temperature: 0.6,
            top_p: 0.95,
            repeat_penalty: 1.1,
            max_tokens: 80_000,
            enable_thinking: true,
            timeout: Duration::from_mins(90),
        }
    }
}

impl LlmConfig {
    /// Defaults, then environment overrides: `WUNICORM_CHAT_URL`,
    /// `WUNICORM_CHAT_MODEL`, `WUNICORM_EMBED_URL`,
    /// `WUNICORM_EMBED_MODEL`.
    pub fn from_env() -> Self {
        fn var_or(name: &str, default: String) -> String {
            std::env::var(name).unwrap_or(default)
        }
        let d = Self::default();
        Self {
            chat_base_url: var_or("WUNICORM_CHAT_URL", d.chat_base_url),
            chat_model: var_or("WUNICORM_CHAT_MODEL", d.chat_model),
            embed_base_url: var_or("WUNICORM_EMBED_URL", d.embed_base_url),
            embed_model: var_or("WUNICORM_EMBED_MODEL", d.embed_model),
            ..d
        }
    }
}

/// What can go wrong at the boundary. Transport and protocol failures
/// carry what the server actually said — the dream report deserves
/// real words, not `reqwest` enums.
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("transport error: {0}")]
    Transport(#[from] ureq::Error),
    #[error("api error {status}: {body}")]
    Api { status: u16, body: String },
    #[error("malformed response: {0}")]
    Malformed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// One chat message. The assistant's thinking never appears here —
/// `llama.cpp` separates it into `reasoning_content`, which we drop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: Role::System,
            content: content.to_string(),
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: Role::User,
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: Role::Assistant,
            content: content.to_string(),
        }
    }
}

/// The client seam: talk to a model, get meaning back.
///
/// `chat` returns the assistant message content only. When `schema`
/// (a JSON Schema value) is given, the request carries it as
/// `response_format: json_schema` — `llama.cpp` constrains generation
/// at the grammar level, so structurally invalid JSON cannot occur.
///
/// `embed` returns one vector per input text, in input order, and
/// enforces [`EMBEDDING_DIM`] — a model/schema width mismatch fails
/// loudly here instead of corrupting graph writes downstream.
pub trait LlmClient {
    fn chat(
        &self,
        messages: &[ChatMessage],
        schema: Option<&serde_json::Value>,
    ) -> Result<String, LlmError>;

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, LlmError>;
}

// ─── The live client ────────────────────────────────────────────────

/// The real client: `ureq`, blocking, one agent with a global timeout.
///
/// HTTP status errors are handled here rather than thrown by `ureq`
/// so that error bodies (`llama.cpp` explains itself in JSON) make it
/// into [`LlmError::Api`].
pub struct UreqClient {
    config: LlmConfig,
    agent: Agent,
}

impl UreqClient {
    pub fn new(config: LlmConfig) -> Self {
        let agent = Agent::config_builder()
            .timeout_global(Some(config.timeout))
            .http_status_as_error(false)
            .build()
            .new_agent();
        Self { config, agent }
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    fn post_json<T: DeserializeOwned>(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<T, LlmError> {
        let mut resp = self.agent.post(url).send_json(body)?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            let body = resp.body_mut().read_to_string().unwrap_or_default();
            return Err(LlmError::Api { status, body });
        }
        Ok(resp.body_mut().read_json::<T>()?)
    }
}

impl LlmClient for UreqClient {
    fn chat(
        &self,
        messages: &[ChatMessage],
        schema: Option<&serde_json::Value>,
    ) -> Result<String, LlmError> {
        let mut body = serde_json::json!({
            "model": self.config.chat_model,
            "messages": messages,
            "temperature": self.config.temperature,
            "top_p": self.config.top_p,
            "repeat_penalty": self.config.repeat_penalty,
            "max_tokens": self.config.max_tokens,
            "chat_template_kwargs": {
                "enable_thinking": self.config.enable_thinking,
            },
        });
        if let Some(schema) = schema {
            body["response_format"] = serde_json::json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "judgment",
                    "strict": true,
                    "schema": schema,
                },
            });
        }
        let url = format!("{}/chat/completions", self.config.chat_base_url);
        let parsed: ChatCompletion = self.post_json(&url, &body)?;
        parsed.assistant_content()
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, LlmError> {
        let body = serde_json::json!({
            "model": self.config.embed_model,
            "input": texts,
        });
        let url = format!("{}/embeddings", self.config.embed_base_url);
        let parsed: EmbeddingsResponse = self.post_json(&url, &body)?;
        let mut data = parsed.data;
        data.sort_by_key(|d| d.index);
        if data.len() != texts.len() {
            return Err(LlmError::Malformed(format!(
                "asked for {} embeddings, got {}",
                texts.len(),
                data.len()
            )));
        }
        for (i, d) in data.iter().enumerate() {
            if d.embedding.len() != EMBEDDING_DIM {
                return Err(LlmError::Malformed(format!(
                    "embedding {i} is {}-wide, the schema declares {EMBEDDING_DIM} — \
                     is the embedding model right?",
                    d.embedding.len()
                )));
            }
        }
        Ok(data.into_iter().map(|d| d.embedding).collect())
    }
}

#[derive(Debug, Deserialize)]
struct ChatCompletion {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: Option<String>,
}

impl ChatCompletion {
    fn assistant_content(self) -> Result<String, LlmError> {
        self.choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| LlmError::Malformed("no assistant content in completion".to_string()))
    }
}

#[derive(Debug, Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

// ─── The hermetic client ────────────────────────────────────────────

#[derive(Debug, Default)]
struct RecordedState {
    chat_responses: VecDeque<Result<String, LlmError>>,
    embed_responses: VecDeque<Result<Vec<Vec<f32>>, LlmError>>,
    chat_calls: Vec<(Vec<ChatMessage>, Option<serde_json::Value>)>,
    embed_calls: Vec<Vec<String>>,
}

/// Canned-response client for tests and demos: chat and embed calls
/// are recorded (inspectable via `chat_calls()` / `embed_calls()`)
/// and answered from queues pushed by the test. An exhausted queue is
/// an error, never silence.
///
/// Clones share state — clone one into an [`LlmJudge`] and keep the
/// original to assert on what the judge sent.
#[derive(Debug, Default)]
pub struct RecordedClient {
    state: Rc<RefCell<RecordedState>>,
}

impl Clone for RecordedClient {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
        }
    }
}

impl RecordedClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_chat(&self, response: Result<String, LlmError>) {
        self.state.borrow_mut().chat_responses.push_back(response);
    }

    pub fn push_embed(&self, response: Result<Vec<Vec<f32>>, LlmError>) {
        self.state.borrow_mut().embed_responses.push_back(response);
    }

    pub fn chat_call_count(&self) -> usize {
        self.state.borrow().chat_calls.len()
    }

    pub fn embed_call_count(&self) -> usize {
        self.state.borrow().embed_calls.len()
    }

    pub fn chat_calls(&self) -> Vec<(Vec<ChatMessage>, Option<serde_json::Value>)> {
        self.state.borrow().chat_calls.clone()
    }

    pub fn embed_calls(&self) -> Vec<Vec<String>> {
        self.state.borrow().embed_calls.clone()
    }
}

impl LlmClient for RecordedClient {
    fn chat(
        &self,
        messages: &[ChatMessage],
        schema: Option<&serde_json::Value>,
    ) -> Result<String, LlmError> {
        self.state
            .borrow_mut()
            .chat_calls
            .push((messages.to_vec(), schema.cloned()));
        self.state
            .borrow_mut()
            .chat_responses
            .pop_front()
            .unwrap_or_else(|| Err(LlmError::Malformed("no recorded chat response".to_string())))
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, LlmError> {
        self.state
            .borrow_mut()
            .embed_calls
            .push(texts.iter().map(ToString::to_string).collect());
        self.state
            .borrow_mut()
            .embed_responses
            .pop_front()
            .unwrap_or_else(|| {
                Err(LlmError::Malformed(
                    "no recorded embed response".to_string(),
                ))
            })
    }
}

// ─── Prompts as data ────────────────────────────────────────────────

/// Loads a prompt by name: the file at `prompts/<name>` when present
/// (prompt iteration without recompiling), else the copy baked into
/// the binary at build time. Both are the same file in the repo —
/// the runtime copy is for iterating, the baked copy is the release.
pub fn load_prompt(name: &str) -> Result<String, LlmError> {
    let runtime = Path::new("prompts").join(name);
    if runtime.is_file() {
        return Ok(std::fs::read_to_string(runtime)?);
    }
    builtin_prompt(name).map_or_else(
        || Err(LlmError::Malformed(format!("no prompt named {name}"))),
        |s| Ok(s.to_string()),
    )
}

fn builtin_prompt(name: &str) -> Option<&'static str> {
    Some(match name {
        "extract-v1.md" => include_str!("../../prompts/extract-v1.md"),
        "schema-v1.json" => include_str!("../../prompts/schema-v1.json"),
        "sermon-v1.md" => include_str!("../../prompts/sermon-v1.md"),
        _ => return None,
    })
}

/// The extraction system prompt — the Dreamer's understanding of
/// nodes and the four pillars.
pub fn extract_prompt() -> Result<String, LlmError> {
    load_prompt("extract-v1.md")
}

/// The extraction output schema, parsed and ready to constrain a
/// chat call.
pub fn extract_schema() -> Result<serde_json::Value, LlmError> {
    let raw = load_prompt("schema-v1.json")?;
    Ok(serde_json::from_str(&raw)?)
}

// ─── Redaction seam ─────────────────────────────────────────────────

/// Redaction seam — everything that leaves the memory layer for a
/// model passes through here first.
///
/// The current model is fully local: text never leaves the machine,
/// so this hook is the identity. That changes the moment any remote
/// endpoint is configured.
// TODO(guard): real redactor (emails, phones, names) before ANY
// remote endpoint is configured — plan.md §4 is emphatic.
pub fn redact(text: &str) -> String {
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_describe_the_local_servers() {
        let c = LlmConfig::default();
        assert!(c.chat_base_url.ends_with(":9931/v1"));
        assert!(c.embed_base_url.ends_with(":9932/v1"));
        assert!((c.temperature - 0.6).abs() < f64::EPSILON);
        assert!(c.repeat_penalty > 1.0, "the recipe that avoids loops");
        assert!(c.enable_thinking, "quality over speed for a nightly dream");
        assert!(c.max_tokens >= 80_000, "thinking cannot starve the content");
        assert!(
            c.timeout.as_secs() >= 5_400,
            "80k tokens at ~18 tok/s is over an hour"
        );
    }

    #[test]
    fn prompt_loader_reads_and_parses() {
        let prompt = extract_prompt().unwrap();
        assert!(prompt.contains("four pillars"), "prompt: {prompt}");
        let schema = extract_schema().unwrap();
        assert!(schema["properties"]["entities"].is_object());
        assert!(schema["properties"]["edges"].is_object());
    }

    #[test]
    fn prompt_loader_rejects_unknown_names() {
        assert!(load_prompt("no-such.md").is_err());
    }

    #[test]
    fn recorded_client_replays_and_records_chats() {
        let c = RecordedClient::new();
        c.push_chat(Ok("{}".to_string()));
        c.push_chat(Err(LlmError::Malformed("boom".to_string())));

        let out = c.chat(&[ChatMessage::user("hi")], None).unwrap();
        assert_eq!(out, "{}");
        assert!(c.chat(&[], None).is_err());
        assert!(c.chat(&[], None).is_err(), "exhausted queue is an error");

        assert_eq!(c.chat_call_count(), 3);
        assert_eq!(c.chat_calls()[0].0[0].content, "hi");
        assert!(c.chat_calls()[0].1.is_none());
    }

    #[test]
    fn recorded_client_replays_and_records_embeddings() {
        let c = RecordedClient::new();
        c.push_embed(Ok(vec![vec![0.0; EMBEDDING_DIM]; 2]));

        let out = c.embed(&["a", "b"]).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].len(), EMBEDDING_DIM);
        assert_eq!(c.embed_calls()[0], vec!["a".to_string(), "b".to_string()]);
        assert!(c.embed(&["c"]).is_err(), "exhausted queue is an error");
        assert_eq!(c.embed_call_count(), 2);
    }

    #[test]
    fn recorded_client_clones_share_state() {
        let original = RecordedClient::new();
        original.push_chat(Ok("{}".to_string()));
        let twin = original.clone();

        // The twin answers; the original sees the call.
        let out = twin.chat(&[ChatMessage::user("hello")], None).unwrap();
        assert_eq!(out, "{}");
        assert_eq!(original.chat_call_count(), 1);
        assert_eq!(original.chat_calls()[0].0[0].content, "hello");
        // And the queue the original pushed is consumed once, by the twin.
        assert!(twin.chat(&[], None).is_err());
    }
}
