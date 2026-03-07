mod api;
mod context;
mod mcp;
mod models;

use anyhow::Result;
use clap::Parser;

use api::GeminiClient;
use models::Content;

pub(crate) const DEFAULT_MODEL: &str = "gemini-3-flash-preview";

#[derive(Parser)]
#[command(name = "gemini", about = "Gemini CLI - Rust implementation")]
struct Cli {
    /// Run as MCP server (JSON-RPC 2.0 over stdio)
    #[arg(long)]
    mcp_server: bool,

    /// Gemini API key (falls back to GEMINI_API_KEY env var)
    #[arg(short, long, env = "GEMINI_API_KEY")]
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
            eprintln!("Error: API key not set. Use -a/--api-key or set GEMINI_API_KEY.");
            std::process::exit(1);
        }
    };

    if cli.mcp_server {
        mcp::run(api_key).await?;
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
