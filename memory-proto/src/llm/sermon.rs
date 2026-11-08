//! The sermon's living voice: [`SermonWriter`] over the client seam.
//!
//! The dreamer's reflection hands over grounded material — anchor,
//! insight summaries, the user's own words — and this writer speaks
//! it as a portrait of the psyche rather than a log of the graph.
//! The facts are never the writer's to invent: the prompt allows only
//! what the material says, and the citations block that follows the
//! prose is rendered mechanically by the dreamer, never by the model.

use crate::dreamer::{SelfReading, SermonWriter};
use crate::llm::{ChatMessage, LlmClient, load_prompt, redact};
use crate::message::{Citation, Insight};

/// The LLM sermon writer. Prose is unconstrained (no grammar — it is
/// not JSON), so the only guardrails are the prompt and the material
/// itself; grounding is enforced downstream by the shelf check on
/// insights and citations, which the writer cannot touch.
pub struct LlmSermonWriter {
    client: Box<dyn LlmClient>,
}

impl LlmSermonWriter {
    pub fn new(client: Box<dyn LlmClient>) -> Self {
        Self { client }
    }
}

impl SermonWriter for LlmSermonWriter {
    fn compose(
        &self,
        anchor: &str,
        insights: &[Insight],
        citations: &[Citation],
        self_reading: Option<&SelfReading>,
    ) -> Result<String, String> {
        let system = load_prompt("sermon-v1.md").map_err(|e| e.to_string())?;
        // The self block is movement language only — the dreamer
        // stripped the numbers at construction; nothing numeric is ever
        // handed to the voice.
        let self_block = self_reading.map(|r| {
            serde_json::json!({
                "movement": r.user_movement.as_deref().map(redact),
                "lean": r.wunicorm_lean.as_deref().map(redact),
                "tension": r.tension.as_deref().map(redact),
            })
        });
        let material = serde_json::json!({
            "anchor": redact(anchor),
            "facts": insights.iter().map(|i| redact(&i.summary)).collect::<Vec<_>>(),
            "notes": citations
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.note_id,
                        "words": redact(&c.snippet),
                    })
                })
                .collect::<Vec<_>>(),
            "self": self_block,
        });
        let messages = vec![
            ChatMessage::system(&system),
            ChatMessage::user(&material.to_string()),
        ];
        let body = self
            .client
            .chat(&messages, None)
            .map_err(|e| e.to_string())?;
        let body = body.trim();
        if body.is_empty() {
            return Err("the writer returned empty prose".to_string());
        }
        Ok(body.to_string())
    }
}
