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
| `-p, --prompt` | Prompt to send **(required)** | — |
| `-a, --api-key` | Gemini API key (or `GEMINI_API_KEY` env) | — |
| `-m, --model` | Model to use | `gemini-3-flash-preview` |

## MCP Setup (Claude Code)

This tool is designed to be used as an MCP server inside Claude Code, providing Gemini with Google Search Grounding as a secondary AI assistant.

### 1. Register as an MCP server

```bash
claude mcp add gemini /path/to/target/release/gemini --scope user
```

Use `--scope user` to make it available across all projects, or `--scope project` to limit it to the current project.

### 2. Set your API key

```bash
export GEMINI_API_KEY="your-api-key"
```

Or add it to your shell profile (`~/.zshrc`, `~/.bashrc`, etc.) to persist it.

### 3. Verify the connection

```bash
claude mcp list
```

Once registered, Claude Code can use the `gemini` tool to delegate prompts to Gemini with real-time Google Search results.

## GEMINI.md

Place a `GEMINI.md` file in your current working directory (or any parent directory up to `.git`) to inject system-level context into every request. Useful for project-specific instructions.

When loaded, the tool prints `[context] Loaded <path>` to stderr.

```
your-project/
├── .git/
├── GEMINI.md   ← loaded automatically
└── src/
```

## License

MIT — see [LICENSE](LICENSE)
