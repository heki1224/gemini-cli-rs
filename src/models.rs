use serde::{Deserialize, Serialize};

// --- Request types ---

#[derive(Serialize, Clone)]
pub struct GenerateRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<SystemInstruction>,
    pub tools: Vec<Tool>,
}

#[derive(Serialize, Clone)]
pub struct SystemInstruction {
    pub parts: Vec<Part>,
}

impl SystemInstruction {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            parts: vec![Part::text(text)],
        }
    }
}

/// Tool definition. Currently only Google Search Grounding is supported.
#[derive(Serialize, Clone)]
pub struct Tool {
    pub google_search: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Part {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
        }
    }
}

impl Content {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            parts: vec![Part::text(text)],
        }
    }
}

// --- Response types ---

#[derive(Deserialize)]
pub struct StreamChunk {
    pub candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
pub struct Candidate {
    pub content: Option<Content>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
    #[serde(rename = "groundingMetadata")]
    pub grounding_metadata: Option<GroundingMetadata>,
}

#[derive(Deserialize, Default)]
pub struct GroundingMetadata {
    #[serde(rename = "groundingChunks", default)]
    pub grounding_chunks: Vec<GroundingChunk>,
}

#[derive(Deserialize)]
pub struct GroundingChunk {
    pub web: Option<GroundingChunkWeb>,
}

#[derive(Deserialize)]
pub struct GroundingChunkWeb {
    pub uri: Option<String>,
    pub title: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_user_sets_role_and_text() {
        let c = Content::user("hello");
        assert_eq!(c.role, "user");
        assert_eq!(c.parts.len(), 1);
        assert_eq!(c.parts[0].text.as_deref(), Some("hello"));
    }

    #[test]
    fn generate_request_serializes_correctly() {
        let req = GenerateRequest {
            contents: vec![Content::user("test")],
            system_instruction: None,
            tools: vec![],
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["contents"][0]["role"], "user");
        assert_eq!(json["contents"][0]["parts"][0]["text"], "test");
        assert!(json.get("system_instruction").is_none());
    }

    #[test]
    fn generate_request_includes_system_instruction() {
        let req = GenerateRequest {
            contents: vec![Content::user("hi")],
            system_instruction: Some(SystemInstruction::new("Be helpful")),
            tools: vec![],
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["system_instruction"]["parts"][0]["text"], "Be helpful");
    }

    #[test]
    fn tool_serializes_google_search() {
        let tool = Tool {
            google_search: serde_json::json!({}),
        };
        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["google_search"], serde_json::json!({}));
    }

    #[test]
    fn stream_chunk_deserializes_with_text() {
        let json = r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"hi"}]}}]}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        let text = chunk.candidates.unwrap()[0].content.as_ref().unwrap().parts[0]
            .text
            .clone()
            .unwrap();
        assert_eq!(text, "hi");
    }

    #[test]
    fn stream_chunk_deserializes_without_candidates() {
        let json = r#"{}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert!(chunk.candidates.is_none());
    }

    #[test]
    fn candidate_deserializes_finish_reason_safety() {
        let json = r#"{"candidates":[{"finishReason":"SAFETY"}]}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        let candidate = &chunk.candidates.unwrap()[0];
        assert_eq!(candidate.finish_reason.as_deref(), Some("SAFETY"));
        assert!(candidate.content.is_none());
    }

    #[test]
    fn candidate_finish_reason_none_when_absent() {
        let json = r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"hi"}]}}]}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        let candidate = &chunk.candidates.unwrap()[0];
        assert!(candidate.finish_reason.is_none());
    }

    #[test]
    fn candidate_deserializes_grounding_metadata() {
        let json = r#"{
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "answer"}]},
                "groundingMetadata": {
                    "groundingChunks": [
                        {"web": {"uri": "https://example.com", "title": "Example"}}
                    ]
                }
            }]
        }"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        let candidate = &chunk.candidates.unwrap()[0];
        let gm = candidate.grounding_metadata.as_ref().unwrap();
        assert_eq!(gm.grounding_chunks.len(), 1);
        let web = gm.grounding_chunks[0].web.as_ref().unwrap();
        assert_eq!(web.uri.as_deref(), Some("https://example.com"));
        assert_eq!(web.title.as_deref(), Some("Example"));
    }
}
