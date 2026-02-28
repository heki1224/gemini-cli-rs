use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};

use crate::models::{
    Content, GenerateRequest, GroundingMetadata, StreamChunk, SystemInstruction, Tool,
};

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Extract text from a single SSE `data:` line.
/// Returns `None` for keepalives, `[DONE]`, or lines without text content.
pub fn parse_sse_text(line: &str) -> Option<String> {
    let json_str = line.strip_prefix("data: ")?;
    if json_str == "[DONE]" {
        return None;
    }
    let chunk: StreamChunk = serde_json::from_str(json_str).ok()?;
    let part = chunk
        .candidates?
        .into_iter()
        .next()?
        .content?
        .parts
        .into_iter()
        .next()?;
    part.text
}

pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
    system_prompt: Option<String>,
}

impl GeminiClient {
    pub fn new(api_key: String, model: String, system_prompt: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            system_prompt,
        }
    }

    /// Send a request and stream the response to stdout.
    /// Displays grounding sources (if any) to stderr after the response.
    pub async fn send(&self, history: &[Content]) -> Result<()> {
        let url = format!("{}/{}:streamGenerateContent?alt=sse", API_BASE, self.model);

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

        let mut stream = response.bytes_stream();
        let mut pending_grounding: Option<GroundingMetadata> = None;
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.context("Stream error")?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if let Some(text) = parse_sse_text(&line) {
                    print!("{}", text);
                    io::stdout().flush().ok();
                }

                // Extract groundingMetadata from the same SSE line (last one wins)
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) {
                        if let Some(mut candidates) = chunk.candidates {
                            if let Some(candidate) = candidates.drain(..).next() {
                                if let Some(gm) = candidate.grounding_metadata {
                                    pending_grounding = Some(gm);
                                }
                            }
                        }
                    }
                }
            }
        }

        println!();

        // Display grounding sources if available
        if let Some(grounding) = pending_grounding {
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

    fn text_sse(text: &str) -> String {
        format!(
            r#"data: {{"candidates":[{{"content":{{"role":"model","parts":[{{"text":"{text}"}}]}}}}]}}"#
        )
    }

    #[test]
    fn parse_sse_text_returns_text() {
        let line = text_sse("Hello");
        assert_eq!(parse_sse_text(&line), Some("Hello".into()));
    }

    #[test]
    fn parse_sse_text_done_returns_none() {
        assert_eq!(parse_sse_text("data: [DONE]"), None);
    }

    #[test]
    fn parse_sse_text_empty_returns_none() {
        assert_eq!(parse_sse_text(""), None);
    }

    #[test]
    fn parse_sse_text_comment_returns_none() {
        assert_eq!(parse_sse_text(": keep-alive"), None);
    }

    #[test]
    fn parse_sse_text_malformed_json_returns_none() {
        assert_eq!(parse_sse_text("data: {not valid json}"), None);
    }

    #[test]
    fn parse_sse_text_no_candidates_returns_none() {
        assert_eq!(parse_sse_text(r#"data: {"candidates":[]}"#), None);
    }

    #[test]
    fn parse_sse_text_null_candidates_returns_none() {
        assert_eq!(parse_sse_text(r#"data: {"candidates":null}"#), None);
    }
}
