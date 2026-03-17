# gemini-cli-rs — Claude Code Context

Gemini CLI written in Rust. Optimized for use as an MCP tool in Claude Code.

## Commands

```bash
cargo build --release        # binary → target/release/gemini
cargo test                   # run all tests (41 tests)
cargo fmt --check            # check formatting (no auto-fix)
cargo clippy -- -D warnings  # lint (warnings = errors)
cargo audit --locked         # security audit (run in CI)
```

## Architecture

```
src/
├── main.rs     # CLI entry point (clap). Delegates to mcp::run() with --mcp-server
├── api.rs      # GeminiClient: SSE streaming. send()=stdout, collect()=String
├── mcp.rs      # JSON-RPC 2.0 MCP server (stdio). Tool: ask-gemini
├── models.rs   # Gemini API type definitions
└── context.rs  # GEMINI.md loader (cwd → .git boundary → ~/.gemini/GEMINI.md)
```

## Key Behaviors

- **API key**: `GEMINI_API_KEY` env var only (no `--api-key` flag)
- **Default model**: `gemini-3-flash-preview` (constant `DEFAULT_MODEL` in main.rs)
- **Model validation**: empty / >100 chars / non-ASCII rejected in both api.rs and mcp.rs
- **SSE buffer limit**: 10 MB
- **MCP prompt limit**: 1 MB
- **GEMINI.md size limit**: 1 MB
- **Google Search Grounding**: always-on. CLI → stderr. MCP → response text
- **Error messages**: internal details hidden via `TOOL_CALL_ERROR_MESSAGE`

## Coding Conventions

- `#![forbid(unsafe_code)]` at crate root
- Error handling: `anyhow::Result` throughout
- `is_empty()` check before `.chars().all()` (vacuous truth on empty string)
- Tests use `wiremock 0.6` (single-chunk delivery — no chunk-boundary testing)
- `reqwest`: use `.connect_timeout()` only, never `.timeout()` for SSE streams

## Release

- **release-plz** creates Release PRs for `feat:` / `fix:` commits
- **cargo-dist** builds binaries for macOS (arm64/x86_64) + Linux (x86_64)
- Merge via merge commit only (squash/rebase disabled on GitHub)
- See `dist-workspace.toml` and `release-plz.toml` for configuration
