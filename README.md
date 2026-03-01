# gemini-cli-rs

![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)
![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)

Gemini CLI written in Rust, optimized for use as an [MCP](https://modelcontextprotocol.io/) tool in Claude Code.

## Features

- **SSE Streaming** — Streams responses from the Gemini API in real time
- **Google Search Grounding** — Always-on; automatically searches the web and prints sources to stderr
- **GEMINI.md Context** — Loads project-specific context from `GEMINI.md` (walks up to the nearest `.git` root)
- **Single-shot mode** — Designed for non-interactive, scriptable use

## Prerequisites

- Rust 1.70+
- A Gemini API key — get one at [Google AI Studio](https://aistudio.google.com/apikey)

## Installation

```bash
git clone https://github.com/heki1224/gemini-cli-rs
cd gemini-cli-rs
cargo build --release
```

Optionally, copy to your PATH:

```bash
cp target/release/gemini ~/.local/bin/gemini
```

## Usage

```bash
# Set your API key (recommended)
export GEMINI_API_KEY="your-api-key"

# Send a prompt
gemini -p "What is the capital of France?"

# Use a different model
gemini -m gemini-2.5-pro -p "Explain Rust's borrow checker"

# Pass the API key directly
gemini -a "your-api-key" -p "Hello"
```

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `-p, --prompt` | Prompt to send (required in CLI mode) | — |
| `-a, --api-key` | Gemini API key (or `GEMINI_API_KEY` env) | — |
| `-m, --model` | Model to use | `gemini-3-flash-preview` |
| `--mcp-server` | Run as MCP server (JSON-RPC 2.0 over stdio) | — |

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

Place a `GEMINI.md` file in your current working directory (or any parent directory up to the `.git` root, or the filesystem root) to inject system-level context into every request. Useful for project-specific instructions.

When loaded, the tool prints `[context] Loaded <path>` to stderr.

```
your-project/
├── .git/
├── GEMINI.md   ← loaded automatically
└── src/
```

## License

MIT — see [LICENSE](LICENSE)
