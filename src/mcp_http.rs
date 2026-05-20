use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{header, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::mcp::{call_tool, make_response, TOOL_CALL_ERROR_MESSAGE};

struct AppState {
    api_key: String,
    http_client: reqwest::Client,
}

/// Run the MCP server (JSON-RPC 2.0 over HTTP, streamable-http transport).
/// Listens on `127.0.0.1:<port>` and serves `POST /mcp`.
pub async fn run(api_key: String, port: u16) -> Result<()> {
    let http_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;

    let state = Arc::new(AppState {
        api_key,
        http_client,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::HeaderName::from_static("mcp-protocol-version"),
        ])
        .expose_headers([header::HeaderName::from_static("mcp-session-id")]);

    // 10 MB body limit matches the stdio LinesCodec limit in mcp.rs.
    // The 1 MB prompt limit is enforced inside call_tool().
    // CorsLayer is outermost so OPTIONS preflight bypasses the body limit check.
    let app = Router::new()
        .route("/mcp", post(handle_mcp))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        .layer(cors)
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("[gemini-mcp-http] Listening on http://{addr}/mcp");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_mcp(State(state): State<Arc<AppState>>, body: Bytes) -> Response {
    let body_str = match std::str::from_utf8(&body) {
        Ok(s) => s,
        Err(_) => {
            return json_response(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": "Parse error" }
            }));
        }
    };

    let request: Value = match serde_json::from_str(body_str) {
        Ok(v) => v,
        Err(_) => {
            return json_response(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": "Parse error" }
            }));
        }
    };

    let method = request["method"].as_str().unwrap_or("");
    eprintln!("[gemini-mcp-http] method={method}");

    let response_id = request.get("id").cloned().filter(|v| !v.is_null());

    if method == "tools/call" {
        match call_tool(&request, &state.api_key, &state.http_client).await {
            Ok(text) => {
                if let Some(ref id) = response_id {
                    json_response(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": text }]
                        }
                    }))
                } else {
                    StatusCode::ACCEPTED.into_response()
                }
            }
            Err(e) => {
                eprintln!("[gemini-mcp-http] Tool call error: {e}");
                if let Some(ref id) = response_id {
                    json_response(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32000, "message": TOOL_CALL_ERROR_MESSAGE }
                    }))
                } else {
                    StatusCode::ACCEPTED.into_response()
                }
            }
        }
    } else {
        match make_response(&request) {
            Some(resp) => json_response(resp),
            None => StatusCode::ACCEPTED.into_response(),
        }
    }
}

fn json_response(value: Value) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&value).unwrap_or_default(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_response_sets_content_type() {
        let resp = json_response(json!({"jsonrpc": "2.0", "id": 1, "result": {}}));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn json_response_body_is_valid_json() {
        let value = json!({"jsonrpc": "2.0", "id": 42, "result": {"foo": "bar"}});
        let serialized = serde_json::to_string(&value).unwrap();
        assert!(!serialized.is_empty());
        assert!(serialized.contains("\"id\":42"));
    }
}
