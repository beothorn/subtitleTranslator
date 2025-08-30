//! OpenAI-backed translator implementation.
//! This uses the GPT-5 nano model with JSON mode for subtitle translation.

use super::Translator;
use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use serde_json::{json, Value};

/// Translator that delegates to the OpenAI chat completion API.
pub struct OpenAiTranslator {
    client: Client,
    api_key: String,
}

impl OpenAiTranslator {
    /// Create a new translator reading the API key from `OPENAI_API_KEY`.
    pub fn new() -> Result<Self> {
        let key = std::env::var("OPENAI_API_KEY")?;
        Ok(Self {
            client: Client::new(),
            api_key: key,
        })
    }

    /// Send a JSON body to the chat completions endpoint and return the JSON response.
    fn post_chat(&self, body: Value) -> Result<Value> {
        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()?;
        let resp = resp.error_for_status()?;
        Ok(resp.json()?)
    }
}

impl Translator for OpenAiTranslator {
    /// Translate a batch of subtitle lines, using summary and previous context.
    fn translate_batch(
        &self,
        summary: &str,
        prev: &[String],
        lines: &[String],
        target_locale: &str,
    ) -> Result<Vec<String>> {
        let prev_text = prev.join("\n");
        let curr_text = lines.join("\n----\n");
        let messages = vec![
            json!({
                "role": "system",
                "content": "You translate English subtitles to Brazilian Portuguese and return JSON {\"lines\": []}."
            }),
            json!({
                "role": "user",
                "content": format!("Summary:\n{summary}\n\nPrevious lines:\n{prev_text}\n\nTranslate the following lines to {target_locale}. Return a JSON object with key 'lines' as an array, keeping order and line breaks. Lines:\n{curr_text}")
            }),
        ];
        let body = json!({
            "model": "gpt-5-nano",
            "response_format": {"type": "json_object"},
            "messages": messages,
        });
        let value = self.post_chat(body)?;
        let content = value["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("missing content"))?;
        let data: Value = serde_json::from_str(content)?;
        let arr = data["lines"]
            .as_array()
            .ok_or_else(|| anyhow!("no lines"))?;
        Ok(arr
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect())
    }

    /// Ask OpenAI for a summary and glossary based on sample lines.
    fn build_glossary(&self, sample: &[String]) -> Result<String> {
        let text = sample.join("\n");
        let messages = vec![
            json!({
                "role": "system",
                "content": "Summarize the video and provide a glossary to avoid mistranslations."
            }),
            json!({"role": "user", "content": text}),
        ];
        let body = json!({
            "model": "gpt-5-nano",
            "messages": messages,
        });
        let value = self.post_chat(body)?;
        let content = value["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("missing content"))?;
        Ok(content.to_string())
    }
}
