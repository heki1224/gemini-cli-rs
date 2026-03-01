use anyhow::Result;
use serde_json::{json, Value};
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;

use crate::api::GeminiClient;
use crate::context;
use crate::models::Content;

const DEFAULT_MODEL: &str = "gemini-3-flash-preview";

/// Run the MCP server (JSON-RPC 2.0 over stdio).
/// All user-visible output goes to stderr; stdout is reserved for the protocol.
pub async fn run(api_key: String) -> Result<()> {
    eprintln!("[gemini-mcp] Server started");

    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[gemini-mcp] Failed to parse request: {}", e);
                continue;
            }
        };

        let id = request.get("id").cloned();
        let method = request["method"].as_str().unwrap_or("");

        eprintln!("[gemini-mcp] method={}", method);

        match method {
            "initialize" => {
                send_response(json!({
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
                }))?;
            }
            "initialized" => {
                // Notification — no response required
                eprintln!("[gemini-mcp] Handshake complete");
            }
            "ping" => {
                if let Some(ref id) = id {
                    send_response(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {}
                    }))?;
                }
            }
            "tools/list" => {
                send_response(json!({
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
                                        "description": "Gemini model to use (default: gemini-3-flash-preview)"
                                    }
                                },
                                "required": ["prompt"]
                            }
                        }]
                    }
                }))?;
            }
            "tools/call" => {
                match call_tool(&request, &api_key).await {
                    Ok(text) => {
                        send_response(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{ "type": "text", "text": text }]
                            }
                        }))?;
                    }
                    Err(e) => {
                        eprintln!("[gemini-mcp] Tool call error: {}", e);
                        send_response(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32000,
                                "message": format!("{}", e)
                            }
                        }))?;
                    }
                }
            }
            _ => {
                // Respond to unknown requests (not notifications) with an error
                if let Some(ref id) = id {
                    send_response(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method not found: {}", method)
                        }
                    }))?;
                }
            }
        }
    }

    eprintln!("[gemini-mcp] stdin closed, shutting down");
    Ok(())
}

async fn call_tool(request: &Value, api_key: &str) -> Result<String> {
    let params = &request["params"];
    let name = params["name"].as_str().unwrap_or("");

    if name != "ask_gemini_mcp" {
        anyhow::bail!("Unknown tool: {}", name);
    }

    let args = &params["arguments"];
    let prompt = args["prompt"].as_str().unwrap_or("").trim().to_string();
    let model = args["model"]
        .as_str()
        .unwrap_or(DEFAULT_MODEL)
        .to_string();

    if prompt.is_empty() {
        anyhow::bail!("prompt must not be empty");
    }

    let system_prompt = context::load_context();
    let client = GeminiClient::new(api_key.to_string(), model, system_prompt);
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
