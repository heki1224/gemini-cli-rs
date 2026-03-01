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

    /// Send a request and collect the full response as a String.
    /// Grounding sources (if any) are appended to the returned text.
    pub async fn collect(&self, history: &[Content]) -> Result<String> {
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
        let mut result = String::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.context("Stream error")?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer.drain(..=newline_pos);

                let Some(json_str) = line.strip_prefix("data: ") else {
                    continue;
                };
                let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) else {
                    continue;
                };
                if let Some(mut candidates) = chunk.candidates {
                    if let Some(candidate) = candidates.drain(..).next() {
                        if let Some(content) = candidate.content {
                            for part in content.parts {
                                if let Some(text) = part.text {
                                    result.push_str(&text);
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

        if let Some(grounding) = pending_grounding {
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
                buffer.drain(..=newline_pos);

                let Some(json_str) = line.strip_prefix("data: ") else {
                    continue;
                };
                let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) else {
                    continue;
                };
                // Extract text and groundingMetadata from a single parse (last grounding wins)
                if let Some(mut candidates) = chunk.candidates {
                    if let Some(candidate) = candidates.drain(..).next() {
                        if let Some(content) = candidate.content {
                            for part in content.parts {
                                if let Some(text) = part.text {
                                    print!("{}", text);
                                    io::stdout().flush().ok();
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

