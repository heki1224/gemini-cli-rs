#![forbid(unsafe_code)]

mod api;
mod context;
mod mcp;
mod mcp_http;
mod models;

use anyhow::Result;
use clap::Parser;

use api::GeminiClient;
use models::Content;

pub(crate) const DEFAULT_MODEL: &str = "gemini-3.5-flash";
pub(crate) const HIGH_PERF_MODEL: &str = "gemini-3.1-pro-preview";

/// Returns the default model, overridable via `GEMINI_DEFAULT_MODEL`.
pub(crate) fn resolve_default_model() -> String {
    std::env::var("GEMINI_DEFAULT_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string())
}

/// Returns the high-performance model, overridable via `GEMINI_HIGH_PERF_MODEL`.
pub(crate) fn resolve_high_perf_model() -> String {
    std::env::var("GEMINI_HIGH_PERF_MODEL").unwrap_or_else(|_| HIGH_PERF_MODEL.to_string())
}

#[derive(Parser)]
#[command(name = "gemini", about = "Gemini CLI - Rust implementation")]
struct Cli {
    /// Run as MCP server (JSON-RPC 2.0 over stdio)
    #[arg(long)]
    mcp_server: bool,

    /// Run as MCP HTTP server (streamable-http transport) on the specified port
    #[arg(long)]
    mcp_http_port: Option<u16>,

    /// Gemini API key (set via GEMINI_API_KEY environment variable)
    #[arg(env = "GEMINI_API_KEY")]
    api_key: Option<String>,

    /// Model to use
    #[arg(short, long, default_value = DEFAULT_MODEL)]
    model: String,

    /// Prompt to send (required in CLI mode)
    #[arg(short, long)]
    prompt: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let api_key = match cli.api_key {
        Some(k) => k,
        None => {
            eprintln!("Error: API key not set. Set the GEMINI_API_KEY environment variable.");
            std::process::exit(1);
        }
    };

    if cli.mcp_server && cli.mcp_http_port.is_some() {
        eprintln!("Error: --mcp-server and --mcp-http-port are mutually exclusive.");
        std::process::exit(1);
    }

    if cli.mcp_server {
        mcp::run(api_key).await?;
    } else if let Some(port) = cli.mcp_http_port {
        mcp_http::run(api_key, port).await?;
    } else {
        let prompt = match cli.prompt {
            Some(p) => p,
            None => {
                eprintln!("Error: --prompt (-p) is required in CLI mode.");
                std::process::exit(1);
            }
        };

        let system_prompt = context::load_context();
        let client = GeminiClient::new(api_key, cli.model, system_prompt);
        let history = vec![Content::user(&prompt)];
        client.send(&history).await?;
    }

    Ok(())
}
