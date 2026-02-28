mod api;
mod context;
mod models;

use anyhow::Result;
use clap::Parser;

use api::GeminiClient;
use models::Content;

#[derive(Parser)]
#[command(name = "gemini", about = "Gemini CLI - Rust implementation")]
struct Cli {
    /// Gemini API key (falls back to GEMINI_API_KEY env var)
    #[arg(short, long, env = "GEMINI_API_KEY")]
    api_key: String,

    /// Model to use
    #[arg(short, long, default_value = "gemini-3-flash-preview")]
    model: String,

    /// Prompt to send
    #[arg(short, long, required = true)]
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let system_prompt = context::load_context();
    let client = GeminiClient::new(cli.api_key, cli.model, system_prompt);

    let history = vec![Content::user(&cli.prompt)];
    client.send(&history).await?;

    Ok(())
}
