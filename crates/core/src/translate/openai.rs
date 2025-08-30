//! OpenAI-backed translator implementation.
//! This uses the GPT-5 nano model with JSON mode for subtitle translation.

use super::{IndexedLine, Translator};
use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tracing::{debug, info, trace};

/// Default human-readable language name used in prompts.
const DEFAULT_LANGUAGE: &str = "Brazilian Portuguese";

/// Replace the `$LANGUAGE` token in the provided template with `language`.
fn with_language(template: &str, language: &str) -> String {
    // Here we swap the language placeholder so prompts can be edited independently from code.
    template.replace("$LANGUAGE", language)
}

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
        let timeout = std::env::var("OPENAI_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(90);
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(timeout))
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
        loop {
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
                    if err.is_timeout() {
                        info!(
                            "openai request timed out after {} ms, retrying",
                            start.elapsed().as_millis()
                        );
                        debug!(?err);
                        continue;
                    }
                    info!(
                        "openai request failed after {} ms",
                        start.elapsed().as_millis()
                    );
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
            return Ok(serde_json::from_str(&text)?);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::MockServer;
    use serde_json::json;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Verify that we can translate a batch using a mocked OpenAI server.
    #[test]
    fn translates_with_mock_server() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("OPENAI_API_KEY", "test");
        let server = MockServer::start();
        std::env::set_var("OPENAI_BASE_URL", server.base_url());
        let _m = server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/v1/chat/completions");
            let content = serde_json::to_string(&json!({
                "translatedLines": [{"index": "1", "translation": "ola"}]
            }))
            .unwrap();
            then.status(200).json_body(json!({
                "choices": [{
                    "message": {"content": content}
                }]
            }));
        });
        let tr = OpenAiTranslator::new().unwrap();
        let out = tr
            .translate_batch(
                "sum",
                &[],
                &[IndexedLine {
                    index: 1,
                    text: "hi".into(),
                }],
                "pt-BR",
            )
            .unwrap();
        assert_eq!(
            out,
            vec![IndexedLine {
                index: 1,
                text: "ola".to_string()
            }]
        );
    }

    /// Verify the glossary prompt mentions Brazilian Portuguese.
    #[test]
    fn glossary_mentions_language() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("OPENAI_API_KEY", "test");
        let server = MockServer::start();
        std::env::set_var("OPENAI_BASE_URL", server.base_url());
        let m = server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/v1/chat/completions")
                .body_contains("Brazilian Portuguese");
            then.status(200).json_body(json!({
                "choices": [{
                    "message": {"content": "sum"}
                }]
            }));
        });
        let tr = OpenAiTranslator::new().unwrap();
        let out = tr.build_glossary(&["hi".to_string()]).unwrap();
        assert_eq!(out, "sum");
        m.assert();
    }

    /// Ensure we retry when the first request times out.
    #[test]
    fn retries_on_timeout() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("OPENAI_API_KEY", "test");
        std::env::set_var("OPENAI_TIMEOUT_SECS", "1");
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for (i, stream) in listener.incoming().enumerate() {
                let mut stream = stream.unwrap();
                let mut buf = [0; 1024];
                let _ = stream.read(&mut buf);
                if i == 0 {
                    thread::sleep(std::time::Duration::from_millis(1500));
                } else {
                    let content = serde_json::to_string(&json!({
                        "translatedLines": [{"index": "1", "translation": "ola"}]
                    }))
                    .unwrap();
                    let body = serde_json::to_string(&json!({
                        "choices": [{"message": {"content": content}}]
                    }))
                    .unwrap();
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    stream.write_all(resp.as_bytes()).unwrap();
                }
            }
        });
        std::env::set_var("OPENAI_BASE_URL", format!("http://{}", addr));
        let tr = OpenAiTranslator::new().unwrap();
        let out = tr
            .translate_batch(
                "sum",
                &[],
                &[IndexedLine {
                    index: 1,
                    text: "hi".into(),
                }],
                "pt-BR",
            )
            .unwrap();
        assert_eq!(
            out,
            vec![IndexedLine {
                index: 1,
                text: "ola".to_string()
            }]
        );
        std::env::remove_var("OPENAI_TIMEOUT_SECS");
    }
}

impl Translator for OpenAiTranslator {
    /// Translate a batch of subtitle lines, using summary and previous context.
    fn translate_batch(
        &self,
        summary: &str,
        prev: &[String],
        lines: &[IndexedLine],
        target_locale: &str,
    ) -> Result<Vec<IndexedLine>> {
        trace!("translate_batch lines={} prev={}", lines.len(), prev.len());
        let prev_text = prev.join("\n");
        let curr_json = json!({
            "translatedLines": lines
                .iter()
                .map(|l| json!({
                    "index": l.index.to_string(),
                    "translation": l.text.clone(),
                }))
                .collect::<Vec<_>>()
        });
        let curr_text = serde_json::to_string_pretty(&curr_json)?;
        let example_in = r#"{
  "translatedLines" :[
    {
      "index": "1",
      "translation": "<i>- Previously on</i> \n<i>\"President Alien\"...</i>"
    },{
      "index": "2",
      "translation": "<i>- There is a deadly blob</i>\n<i>running around.</i>"
    },{
      "index": "3",
      "translation": "- I called in\nAgent Baxter Boy"
    }]
}"#;
        let example_out = r#"{
  "translatedLines" :[
    {
      "index": "1",
      "translation": "<i>-Anteriormente em</i> \n<i>\"Presidente Alien\"...</i>"
    },{
      "index": "2",
      "translation": "<i>- Tem um blob assassino</i>\n<i>à solta.</i>"
    },{
      "index": "3",
      "translation": "- Eu chamei o \nAgente Baxter Boy"
    }]
}"#;
        let system_prompt = with_language(
            include_str!("prompts/translate_system.prompt"),
            DEFAULT_LANGUAGE,
        );
        let user_prompt = include_str!("prompts/translate_user.prompt")
            .replace("$SUMMARY", summary)
            .replace("$PREVIOUS_LINES", &prev_text)
            .replace("$TARGET_LOCALE", target_locale)
            .replace("$EXAMPLE_IN", example_in)
            .replace("$EXAMPLE_OUT", example_out)
            .replace("$LINES", &curr_text);
        let messages = vec![
            json!({ "role": "system", "content": system_prompt }),
            json!({ "role": "user", "content": user_prompt }),
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
        let arr = data["translatedLines"]
            .as_array()
            .ok_or_else(|| anyhow!("no translatedLines"))?;
        Ok(arr
            .iter()
            .filter_map(|v| {
                let idx = v["index"].as_str()?.parse().ok()?;
                let text = v["translation"].as_str()?.to_string();
                Some(IndexedLine { index: idx, text })
            })
            .collect())
    }
    /// Ask OpenAI for a summary and glossary based on sample lines.
    fn build_glossary(&self, sample: &[String]) -> Result<String> {
        trace!("build_glossary sample_lines={}", sample.len());
        let text = sample.join("\n");
        let system_prompt = with_language(
            include_str!("prompts/glossary_system.prompt"),
            DEFAULT_LANGUAGE,
        );
        let messages = vec![
            json!({ "role": "system", "content": system_prompt }),
            json!({ "role": "user", "content": text }),
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
