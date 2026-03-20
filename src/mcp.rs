use anyhow::Result;
use serde_json::{json, Value};
use std::io::{self, Write};

use crate::api::GeminiClient;
use crate::context;
use crate::models::{validate_model_name, Content};

const TOOL_CALL_ERROR_MESSAGE: &str = "Internal error";

/// Run the MCP server (JSON-RPC 2.0 over stdio).
/// All user-visible output goes to stderr; stdout is reserved for the protocol.
pub async fn run(api_key: String) -> Result<()> {
    eprintln!("[gemini-mcp] Server started");

    let http_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;

    const MAX_LINE_BYTES: usize = 1024 * 1024; // 1 MB per JSON-RPC line
    let codec = tokio_util::codec::LinesCodec::new_with_max_length(MAX_LINE_BYTES);
    let mut framed = tokio_util::codec::FramedRead::new(tokio::io::stdin(), codec);

    while let Some(result) = futures_util::StreamExt::next(&mut framed).await {
        let line_owned = match result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[gemini-mcp] Input line too long or read error: {}", e);
                continue;
            }
        };
        let line = line_owned.trim();
        if line.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[gemini-mcp] Failed to parse request: {}", e);
                send_response(json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": "Parse error" }
                }))?;
                continue;
            }
        };

        let id = request.get("id").cloned();
        let method = request["method"].as_str().unwrap_or("");

        eprintln!("[gemini-mcp] method={}", method);

        let response_id = id.filter(|v| !v.is_null());

        match method {
            "tools/call" => match call_tool(&request, &api_key, &http_client).await {
                Ok(text) => {
                    if let Some(ref id) = response_id {
                        send_response(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{ "type": "text", "text": text }]
                            }
                        }))?;
                    }
                }
                Err(e) => {
                    eprintln!("[gemini-mcp] Tool call error: {}", e);
                    if let Some(ref id) = response_id {
                        send_response(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32000,
                                "message": TOOL_CALL_ERROR_MESSAGE
                            }
                        }))?;
                    }
                }
            },
            _ => {
                if let Some(resp) = make_response(&request) {
                    send_response(resp)?;
                }
            }
        }
    }

    eprintln!("[gemini-mcp] stdin closed, shutting down");
    Ok(())
}

/// Build a JSON-RPC 2.0 response for synchronous (non-tool-call) methods.
/// Returns None for notifications (no `id`) or when no response is required.
fn make_response(request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request["method"].as_str().unwrap_or("");

    match method {
        "initialize" => id.filter(|v| !v.is_null()).map(|id| json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "gemini-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        })),
        "initialized" => {
            eprintln!("[gemini-mcp] Handshake complete");
            None
        }
        "ping" => id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {}
            })
        }),
        "tools/list" => id.filter(|v| !v.is_null()).map(|id| json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [{
                    "name": "ask_gemini_mcp",
                    "description": "Ask Google Gemini a question. Supports Google Search grounding for up-to-date information. Use for web search, code review, and technical advice.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "prompt": {
                                "type": "string",
                                "description": "The prompt to send to Gemini"
                            },
                            "model": {
                                "type": "string",
                                "description": format!("Gemini model to use (default: {}). Ignored when thinking=true.", crate::DEFAULT_MODEL)
                            },
                            "thinking": {
                                "type": "boolean",
                                "description": format!("Use the high-performance model ({}) for complex reasoning, deep analysis, or multi-step tasks. Default: false (uses {}).", crate::HIGH_PERF_MODEL, crate::DEFAULT_MODEL)
                            }
                        },
                        "required": ["prompt"]
                    }
                }]
            }
        })),
        _ => id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            })
        }),
    }
}

pub(crate) async fn call_tool(
    request: &Value,
    api_key: &str,
    http_client: &reqwest::Client,
) -> Result<String> {
    let params = &request["params"];
    let name = params["name"].as_str().unwrap_or("");

    if name != "ask_gemini_mcp" {
        anyhow::bail!("Unknown tool: {}", name);
    }

    let args = &params["arguments"];
    let prompt = args["prompt"].as_str().unwrap_or("").trim().to_string();
    let thinking = args["thinking"].as_bool().unwrap_or(false);
    let model = if thinking {
        crate::resolve_high_perf_model()
    } else {
        args["model"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(crate::resolve_default_model)
    };

    if prompt.is_empty() {
        anyhow::bail!("prompt must not be empty");
    }

    const MAX_PROMPT_BYTES: usize = 1024 * 1024; // 1 MB
    if prompt.len() > MAX_PROMPT_BYTES {
        anyhow::bail!("prompt exceeds maximum size (1 MB)");
    }

    validate_model_name(&model)?;

    let system_prompt = context::load_context();
    let client = GeminiClient::with_client(
        http_client.clone(),
        api_key.to_string(),
        model,
        system_prompt,
    );
    let history = vec![Content::user(prompt)];

    client.collect(&history).await
}

/// Write a single JSON-RPC response line to stdout and flush.
fn send_response(value: Value) -> Result<()> {
    let json = serde_json::to_string(&value)?;
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{}", json)?;
    handle.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(id: u64, method: &str) -> Value {
        json!({"jsonrpc":"2.0","id":id,"method":method,"params":{}})
    }

    fn notif(method: &str) -> Value {
        json!({"jsonrpc":"2.0","method":method})
    }

    #[test]
    fn initialize_returns_protocol_version() {
        let resp = make_response(&req(1, "initialize")).unwrap();
        assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
        assert!(resp["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn initialize_echoes_id() {
        let resp = make_response(&req(42, "initialize")).unwrap();
        assert_eq!(resp["id"], 42);
    }

    #[test]
    fn initialized_notification_returns_none() {
        assert!(make_response(&notif("initialized")).is_none());
    }

    #[test]
    fn initialize_notification_returns_none() {
        assert!(make_response(&notif("initialize")).is_none());
    }

    #[test]
    fn tools_list_notification_returns_none() {
        assert!(make_response(&notif("tools/list")).is_none());
    }

    #[test]
    fn tools_list_contains_ask_gemini_mcp() {
        let resp = make_response(&req(2, "tools/list")).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "ask_gemini_mcp");
        assert!(tools[0]["inputSchema"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("prompt")));
    }

    #[test]
    fn unknown_method_with_id_returns_method_not_found() {
        let resp = make_response(&req(3, "unknown/method")).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }

    fn fake_client() -> reqwest::Client {
        reqwest::Client::new()
    }

    #[tokio::test]
    async fn call_tool_unknown_tool_name_errors() {
        let request = json!({"params":{"name":"unknown_tool","arguments":{}}});
        let err = call_tool(&request, "fake-key", &fake_client())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn call_tool_empty_prompt_errors() {
        let request = json!({"params":{"name":"ask_gemini_mcp","arguments":{"prompt":"  "}}});
        let err = call_tool(&request, "fake-key", &fake_client())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("prompt must not be empty"));
    }

    #[tokio::test]
    async fn call_tool_empty_model_name_errors() {
        let request =
            json!({"params":{"name":"ask_gemini_mcp","arguments":{"prompt":"hi","model":""}}});
        let err = call_tool(&request, "fake-key", &fake_client())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Invalid model name"));
    }

    #[tokio::test]
    async fn call_tool_too_long_model_name_errors() {
        let long_model = "a".repeat(101);
        let request = json!({"params":{"name":"ask_gemini_mcp","arguments":{"prompt":"hi","model":long_model}}});
        let err = call_tool(&request, "fake-key", &fake_client())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Invalid model name"));
    }

    #[tokio::test]
    async fn call_tool_oversized_prompt_errors() {
        let big_prompt = "x".repeat(1024 * 1024 + 1);
        let request = json!({"params":{"name":"ask_gemini_mcp","arguments":{"prompt":big_prompt}}});
        let err = call_tool(&request, "fake-key", &fake_client())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("exceeds maximum size"));
    }

    #[tokio::test]
    async fn call_tool_invalid_model_name_errors() {
        let request = json!({"params":{"name":"ask_gemini_mcp","arguments":{"prompt":"hi","model":"../../etc/passwd"}}});
        let err = call_tool(&request, "fake-key", &fake_client())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Invalid model name"));
    }

    #[tokio::test]
    async fn call_tool_unicode_model_name_rejected() {
        // Unicode alphanumeric (e.g. Japanese) must be rejected by ascii-only validation
        let request = json!({"params":{"name":"ask_gemini_mcp","arguments":{"prompt":"hi","model":"モデル"}}});
        let err = call_tool(&request, "fake-key", &fake_client())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Invalid model name"));
    }

    #[test]
    fn call_tool_valid_model_name_passes_validation() {
        assert!(validate_model_name("gemini-1.5-flash").is_ok());
        assert!(validate_model_name("gemini-3-flash-preview").is_ok());
        assert!(validate_model_name("gemini-2.5-pro").is_ok());
    }

    #[test]
    fn call_tool_invalid_model_names_rejected_by_validator() {
        assert!(validate_model_name("../../etc/passwd").is_err());
        assert!(validate_model_name("モデル").is_err());
        assert!(validate_model_name("").is_err());
        assert!(validate_model_name("model name with spaces").is_err());
    }

    #[tokio::test]
    async fn call_tool_thinking_true_uses_high_perf_model_passes_validation() {
        // thinking=true should select HIGH_PERF_MODEL, which is a valid model name.
        // The call fails at the API level (fake key), not at model validation.
        let request =
            json!({"params":{"name":"ask_gemini_mcp","arguments":{"prompt":"hi","thinking":true}}});
        let err = call_tool(&request, "fake-key", &fake_client())
            .await
            .unwrap_err();
        assert!(!err.to_string().contains("Invalid model name"));
    }

    #[tokio::test]
    async fn call_tool_thinking_false_falls_back_to_default() {
        // thinking=false without model arg should use DEFAULT_MODEL (or env override).
        // Fails at API level, not model validation.
        let request = json!({"params":{"name":"ask_gemini_mcp","arguments":{"prompt":"hi","thinking":false}}});
        let err = call_tool(&request, "fake-key", &fake_client())
            .await
            .unwrap_err();
        assert!(!err.to_string().contains("Invalid model name"));
    }

    #[test]
    fn tools_list_schema_contains_thinking_parameter() {
        let resp = make_response(&req(2, "tools/list")).unwrap();
        let tool = &resp["result"]["tools"][0];
        let props = &tool["inputSchema"]["properties"];
        assert!(props.get("thinking").is_some());
        assert_eq!(props["thinking"]["type"], "boolean");
    }

    #[test]
    fn error_response_omits_internal_details() {
        // Build the same JSON that the run() loop sends on tool-call failure,
        // and verify the message field is the generic constant — not a raw error string.
        let id = json!(1);
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32000,
                "message": TOOL_CALL_ERROR_MESSAGE
            }
        });
        let message = response["error"]["message"].as_str().unwrap();
        assert_eq!(message, "Internal error");
        assert!(!message.contains("key"));
        assert!(!message.contains("path"));
        assert!(!message.contains("token"));
    }
}
