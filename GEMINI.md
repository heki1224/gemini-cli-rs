# gemini-cli-rs — Gemini Context

You are assisting with development of `gemini-cli-rs`, a Gemini CLI written in Rust.
This file is loaded as a system prompt by the `gemini` binary itself (via `context.rs`).

## Project Role

- **Purpose**: Single-shot Gemini CLI optimized for Claude Code MCP integration
- **Binary**: `gemini` (`target/release/gemini`)
- **Key modes**: CLI (`-p <prompt>`) and MCP server (`--mcp-server`)

## Tech Stack

- Rust 2021 edition, tokio async runtime
- reqwest 0.13 (SSE streaming), clap 4 (CLI), anyhow (errors), serde_json
- Gemini API: `streamGenerateContent?alt=sse`, auth via `x-goog-api-key` header
- Default model: `gemini-3-flash-preview`

## Code Conventions

- `#![forbid(unsafe_code)]` — no unsafe allowed
- Validate model name: non-empty, ≤100 chars, ASCII alphanumeric + `-` + `.` only
- SSE buffer ≤10 MB, MCP prompt ≤1 MB, GEMINI.md ≤1 MB
- Hide internal errors: return `TOOL_CALL_ERROR_MESSAGE = "Internal error"` in MCP
- `connect_timeout()` only for SSE (never `timeout()`)

## Build & Test

```bash
cargo build --release
cargo test
cargo fmt --check && cargo clippy -- -D warnings
```

## Important Notes

- Model names must be verified against the API — do not invent model names
- Google Search Grounding is always-on; no explicit instruction needed
- `searchEntryPoint` display is intentionally not implemented (ToS requirement waived)
