use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};

use crate::models::{
    Content, GenerateRequest, GroundingMetadata, StreamChunk, SystemInstruction, Tool,
};

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
    system_prompt: Option<String>,
    api_base: String,
}

impl GeminiClient {
    /// Create a new client with a fresh `reqwest::Client`. Used in CLI mode.
    pub fn new(api_key: String, model: String, system_prompt: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            system_prompt,
            api_base: API_BASE.to_string(),
        }
    }

    /// Create a client reusing an existing `reqwest::Client`. Used in MCP mode.
    pub fn with_client(
        client: Client,
        api_key: String,
        model: String,
        system_prompt: Option<String>,
    ) -> Self {
        Self {
            client,
            api_key,
            model,
            system_prompt,
            api_base: API_BASE.to_string(),
        }
    }

    /// Core SSE streaming logic. Calls `on_text` for each text chunk received.
    /// Returns the final grounding metadata (if any).
    async fn stream_sse<F>(
        &self,
        history: &[Content],
        mut on_text: F,
    ) -> Result<Option<GroundingMetadata>>
    where
        F: FnMut(&str),
    {
        if self.model.is_empty()
            || self.model.len() > 100
            || !self
                .model
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        {
            anyhow::bail!("Invalid model name: {}", self.model);
        }

        let url = format!(
            "{}/{}:streamGenerateContent?alt=sse",
            self.api_base, self.model
        );

        let body = GenerateRequest {
            contents: history.to_vec(),
            system_instruction: self.system_prompt.as_deref().map(SystemInstruction::new),
            tools: vec![Tool {
                google_search: json!({}),
            }],
        };

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .context("Failed to connect to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, text);
        }

        const MAX_BUFFER_BYTES: usize = 10 * 1024 * 1024; // 10 MB

        let mut stream = response.bytes_stream();
        let mut pending_grounding: Option<GroundingMetadata> = None;
        let mut buffer: Vec<u8> = Vec::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.context("Stream error")?;
            if buffer.len() + bytes.len() > MAX_BUFFER_BYTES {
                anyhow::bail!("Response exceeded maximum buffer size (10 MB)");
            }
            buffer.extend_from_slice(&bytes);

            while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                let line = String::from_utf8_lossy(&buffer[..newline_pos])
                    .trim()
                    .to_string();
                buffer.drain(..=newline_pos);

                let Some(json_str) = line.strip_prefix("data: ") else {
                    continue;
                };
                let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) else {
                    continue;
                };
                if let Some(mut candidates) = chunk.candidates {
                    if let Some(candidate) = candidates.drain(..).next() {
                        // Check finish_reason before accessing content (content may be null on SAFETY)
                        if candidate.finish_reason.as_deref() == Some("SAFETY") {
                            anyhow::bail!("Response blocked by safety filters");
                        }
                        if let Some(content) = candidate.content {
                            for part in content.parts {
                                if let Some(text) = part.text {
                                    on_text(&text);
                                }
                            }
                        }
                        if let Some(gm) = candidate.grounding_metadata {
                            pending_grounding = Some(gm);
                        }
                    }
                }
            }
        }

        Ok(pending_grounding)
    }

    /// Send a request and collect the full response as a String.
    /// Grounding sources (if any) are appended to the returned text.
    pub async fn collect(&self, history: &[Content]) -> Result<String> {
        let mut result = String::new();
        let grounding = self
            .stream_sse(history, |text| result.push_str(text))
            .await?;

        if let Some(grounding) = grounding {
            let sources: Vec<_> = grounding
                .grounding_chunks
                .iter()
                .filter_map(|c| c.web.as_ref())
                .collect();
            if !sources.is_empty() {
                result.push_str("\n\n[Sources]\n");
                for web in sources {
                    let title = web.title.as_deref().unwrap_or("Unknown");
                    let uri = web.uri.as_deref().unwrap_or("");
                    result.push_str(&format!("- {} ({})\n", title, uri));
                }
            }
        }

        Ok(result)
    }

    /// Send a request and stream the response to stdout.
    /// Displays grounding sources (if any) to stderr after the response.
    pub async fn send(&self, history: &[Content]) -> Result<()> {
        let grounding = self
            .stream_sse(history, |text| {
                print!("{}", text);
                io::stdout().flush().ok();
            })
            .await?;

        println!();

        if let Some(grounding) = grounding {
            let sources: Vec<_> = grounding
                .grounding_chunks
                .iter()
                .filter_map(|c| c.web.as_ref())
                .collect();
            if !sources.is_empty() {
                eprintln!("\n[Sources]");
                for web in sources {
                    let title = web.title.as_deref().unwrap_or("Unknown");
                    let uri = web.uri.as_deref().unwrap_or("");
                    eprintln!("- {} ({})", title, uri);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sse_body(text: &str) -> String {
        let json = format!(
            r#"{{"candidates":[{{"content":{{"role":"model","parts":[{{"text":"{text}"}}]}}}}]}}"#
        );
        format!("data: {json}\n\n")
    }

    fn safety_body() -> String {
        let json = r#"{"candidates":[{"finishReason":"SAFETY"}]}"#;
        format!("data: {json}\n\n")
    }

    async fn mock_client(server: &MockServer) -> GeminiClient {
        GeminiClient {
            client: Client::new(),
            api_key: "test-key".to_string(),
            model: "test-model".to_string(),
            system_prompt: None,
            api_base: server.uri(),
        }
    }

    #[tokio::test]
    async fn collect_returns_text_from_sse() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse_body("hello world")),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let history = vec![Content::user("hi")];
        let result = client.collect(&history).await.unwrap();
        assert_eq!(result, "hello world");
    }

    #[tokio::test]
    async fn collect_returns_error_on_safety_block() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(safety_body()),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let history = vec![Content::user("hi")];
        let err = client.collect(&history).await.unwrap_err();
        assert!(err.to_string().contains("safety filters"));
    }

    #[tokio::test]
    async fn collect_returns_error_on_http_4xx() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Too Many Requests"))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let history = vec![Content::user("hi")];
        let err = client.collect(&history).await.unwrap_err();
        assert!(err.to_string().contains("429"));
    }

    #[tokio::test]
    async fn collect_handles_multibyte_characters_correctly() {
        // Verifies that the Vec<u8> buffer correctly handles multibyte characters
        // (e.g. Japanese). The previous String buffer would corrupt them if a stream
        // chunk boundary fell inside a multibyte sequence.
        let japanese = "日本語テスト";
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse_body(japanese)),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let history = vec![Content::user("hi")];
        let result = client.collect(&history).await.unwrap();
        assert_eq!(result, japanese);
    }

    #[tokio::test]
    async fn stream_sse_rejects_invalid_model_name() {
        // No server needed — validation fires before any HTTP request
        let client = GeminiClient {
            client: Client::new(),
            api_key: "test-key".to_string(),
            model: "../../etc/passwd".to_string(),
            system_prompt: None,
            api_base: "http://localhost".to_string(),
        };
        let history = vec![Content::user("hi")];
        let err = client.collect(&history).await.unwrap_err();
        assert!(err.to_string().contains("Invalid model name"));
    }

    #[tokio::test]
    async fn stream_sse_rejects_empty_model_name() {
        let client = GeminiClient {
            client: Client::new(),
            api_key: "test-key".to_string(),
            model: "".to_string(),
            system_prompt: None,
            api_base: "http://localhost".to_string(),
        };
        let history = vec![Content::user("hi")];
        let err = client.collect(&history).await.unwrap_err();
        assert!(err.to_string().contains("Invalid model name"));
    }

    #[tokio::test]
    async fn stream_sse_rejects_too_long_model_name() {
        let client = GeminiClient {
            client: Client::new(),
            api_key: "test-key".to_string(),
            model: "a".repeat(101),
            system_prompt: None,
            api_base: "http://localhost".to_string(),
        };
        let history = vec![Content::user("hi")];
        let err = client.collect(&history).await.unwrap_err();
        assert!(err.to_string().contains("Invalid model name"));
    }

    #[tokio::test]
    async fn stream_sse_rejects_oversized_response() {
        let server = MockServer::start().await;
        // Body slightly over 10 MB to trigger the buffer limit
        let oversized = "x".repeat(10 * 1024 * 1024 + 1);
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(oversized),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let history = vec![Content::user("hi")];
        let err = client.collect(&history).await.unwrap_err();
        assert!(err.to_string().contains("maximum buffer size"));
    }

    #[tokio::test]
    async fn collect_appends_grounding_sources() {
        let body = format!(
            "data: {}\n\n",
            r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"answer"}]},"groundingMetadata":{"groundingChunks":[{"web":{"uri":"https://example.com","title":"Example"}}]}}]}"#
        );

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(body),
            )
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let history = vec![Content::user("hi")];
        let result = client.collect(&history).await.unwrap();
        assert!(result.contains("answer"));
        assert!(result.contains("[Sources]"));
        assert!(result.contains("Example"));
        assert!(result.contains("https://example.com"));
    }
}
