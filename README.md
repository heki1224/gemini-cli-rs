# gemini-cli-rs

English | [日本語](README.ja.md)

![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)

Gemini CLI written in Rust, optimized for use as an [MCP](https://modelcontextprotocol.io/) tool in Claude Code.

## Features

- **MCP Server mode** — Runs as a native MCP server (`--mcp-server`); integrates directly with Claude Code
- **SSE Streaming** — Streams responses from the Gemini API in real time
- **Google Search Grounding** — Always-on; sources are printed to stderr in CLI mode and included in the response in MCP mode
- **GEMINI.md Context** — Loads project-specific context from `GEMINI.md` (walks up to the nearest `.git` root)
- **Single-shot mode** — Designed for non-interactive, scriptable use

## Prerequisites

- A Gemini API key (`GEMINI_API_KEY`) — get one at [Google AI Studio](https://aistudio.google.com/apikey)

## Installation

### Download pre-built binary

Pre-built binaries are available for macOS and Linux on the [Releases page](https://github.com/heki1224/gemini-cli-rs/releases/latest).

```bash
# Apple Silicon macOS
curl -L https://github.com/heki1224/gemini-cli-rs/releases/latest/download/gemini-cli-rs-aarch64-apple-darwin.tar.xz | tar -xJ
mv gemini ~/.local/bin/gemini

# Intel macOS
curl -L https://github.com/heki1224/gemini-cli-rs/releases/latest/download/gemini-cli-rs-x86_64-apple-darwin.tar.xz | tar -xJ
mv gemini ~/.local/bin/gemini

# Linux x86_64
curl -L https://github.com/heki1224/gemini-cli-rs/releases/latest/download/gemini-cli-rs-x86_64-unknown-linux-gnu.tar.xz | tar -xJ
mv gemini ~/.local/bin/gemini
```

### Build from source

```bash
git clone https://github.com/heki1224/gemini-cli-rs
cd gemini-cli-rs
cargo build --release
cp target/release/gemini ~/.local/bin/gemini
```

## Usage

### CLI mode

```bash
# Set your API key
export GEMINI_API_KEY="your-api-key"

# Send a prompt
gemini -p "What is the capital of France?"

# Use a different model
gemini -m gemini-2.5-pro -p "Explain Rust's borrow checker"
```

Grounding sources are printed to **stderr** after the response.

### MCP server mode

In MCP mode the binary is launched automatically by Claude Code — you do not normally run it directly. See [MCP Setup](#mcp-setup-claude-code) for registration steps.

```bash
# Manual launch (for debugging)
GEMINI_API_KEY="your-api-key" gemini --mcp-server
```

Grounding sources are included **in the response text** (not stderr).

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `-p, --prompt` | Prompt to send (required in CLI mode) | — |
| `-m, --model` | Model to use | `gemini-3-flash-preview` |
| `--mcp-server` | Run as MCP server (JSON-RPC 2.0 over stdio) | — |

The API key is read from the `GEMINI_API_KEY` environment variable only (no `--api-key` flag).

## MCP Setup (Claude Code)

This tool is designed to be used as an MCP server inside Claude Code, providing Gemini with Google Search Grounding as a secondary AI assistant.

### 1. Build the binary

```bash
cargo build --release
```

### 2. Register as an MCP server

```bash
claude mcp add gemini /path/to/target/release/gemini --scope user -- --mcp-server
```

Use `--scope user` to make it available across all projects, or `--scope project` to limit it to the current project.

### 3. Set your API key

Add `GEMINI_API_KEY` to your shell profile (`~/.zshrc`, `~/.bashrc`, etc.):

```bash
export GEMINI_API_KEY="your-api-key"
```

### 4. Restart Claude Code and verify

```bash
claude mcp list
```

Once registered, the `ask_gemini_mcp` tool is available. Claude Code routes prompts to Gemini with real-time Google Search grounding.

> **Note:** The binary writes protocol messages to stdout and all logs to stderr. Do not redirect stderr if you want to see debug output.

## GEMINI.md

Place a `GEMINI.md` file in your current working directory (or any parent directory) to inject system-level context into every request. The search walks up to the nearest `.git` directory; if no `.git` is found, it continues to the filesystem root. Useful for project-specific instructions.

> **Note:** Files larger than 1 MB are silently ignored.

```
your-project/
├── .git/
├── GEMINI.md   ← loaded automatically
└── src/
```

## License

MIT — see [LICENSE](LICENSE)
