# gemini-cli-rs — AI Agent Guide

Gemini CLI written in Rust, optimized for use as an MCP tool in Claude Code.

## Setup

```bash
# Prerequisites: Rust toolchain (rustup)
git clone https://github.com/heki1224/gemini-cli-rs
cd gemini-cli-rs
cargo build --release
export GEMINI_API_KEY="your-api-key"
./target/release/gemini -p "Hello"
```

## Project Structure

```
src/
├── main.rs     # CLI entry point (clap parser, --mcp-server flag)
├── api.rs      # GeminiClient: SSE streaming to stdout / String
├── mcp.rs      # JSON-RPC 2.0 MCP server over stdio
├── models.rs   # Gemini API request/response types
└── context.rs  # GEMINI.md loader (local → ~/.gemini/GEMINI.md)
```

## Common Workflows

### Run tests
```bash
cargo test
cargo fmt --check && cargo clippy -- -D warnings
cargo audit
```

### Add a feature
1. Write tests first (wiremock for HTTP, unit tests for pure functions)
2. Implement in the relevant module
3. Verify: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
4. Commit with Conventional Commits (`feat:`, `fix:`, etc.)

### Release
- Push `feat:` or `fix:` commit to `main` → release-plz creates a Release PR
- Merge PR (merge commit only, not squash) → cargo-dist builds binaries automatically

## Key Constraints

- No `unsafe` code (`#![forbid(unsafe_code)]`)
- API key via `GEMINI_API_KEY` env var only
- Model name: validated by `validate_model_name()` in models.rs (non-empty, ≤100 chars, ASCII alphanumeric + `-` + `.`)
- SSE streaming: use `connect_timeout()`, never `timeout()`
- MCP errors must not leak internal details to callers
