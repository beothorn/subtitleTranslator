//! OpenAI-backed translator implementation.
//! This uses the GPT-5 nano model with JSON mode for subtitle translation.

use super::Translator;
use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tracing::{debug, info, trace};

/// Translator that delegates to the OpenAI chat completion API.
pub struct OpenAiTranslator {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAiTranslator {
    /// Create a new translator reading the API key from `OPENAI_API_KEY`.
    pub fn new() -> Result<Self> {
        trace!("OpenAiTranslator::new");
        let key = std::env::var("OPENAI_API_KEY")?;
        let base = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com".to_string());
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(90))
            .build()?;
        debug!("using base_url={base}");
        Ok(Self {
            client,
            api_key: key,
            base_url: base,
        })
    }

    /// Send a JSON body to the chat completions endpoint and return the JSON response.
    fn post_chat(&self, body: Value) -> Result<Value> {
        trace!("post_chat");
        debug!(request = %body);
        let url = format!("{}/v1/chat/completions", self.base_url);
        info!("sending request to OpenAI");
        let start = Instant::now();
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send();
        let resp = match resp {
            Ok(r) => r,
            Err(err) => {
                info!("openai request failed after {} ms", start.elapsed().as_millis());
                debug!(?err);
                return Err(err.into());
            }
        };
        let status = resp.status();
        let text = resp.text()?;
        info!(
            "openai responded in {} ms with status {}",
            start.elapsed().as_millis(),
            status
        );
        debug!(response = %text);
        if !status.is_success() {
            return Err(anyhow!("openai error: {status} {text}"));
        }
        Ok(serde_json::from_str(&text)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::MockServer;
    use serde_json::json;

    /// Verify that we can translate a batch using a mocked OpenAI server.
    #[test]
    fn translates_with_mock_server() {
        std::env::set_var("OPENAI_API_KEY", "test");
        let server = MockServer::start();
        std::env::set_var("OPENAI_BASE_URL", server.base_url());
        let _m = server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/v1/chat/completions");
            then.status(200).json_body(json!({
                "choices": [{
                    "message": {"content": "{\"lines\": [\"ola\"]}"}
                }]
            }));
        });
        let tr = OpenAiTranslator::new().unwrap();
        let out = tr
            .translate_batch("sum", &[], &["hi".to_string()], "pt-BR")
            .unwrap();
        assert_eq!(out, vec!["ola".to_string()]);
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
        trace!("translate_batch lines={} prev={}", lines.len(), prev.len());
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
        trace!("build_glossary sample_lines={}", sample.len());
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
