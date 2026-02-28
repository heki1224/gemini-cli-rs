# gemini-cli-rs

Gemini CLI written in Rust, optimized for use as an [MCP](https://modelcontextprotocol.io/) tool in Claude Code.

## Features

- **SSE Streaming** — Streams responses from the Gemini API in real time
- **Google Search Grounding** — Automatically searches the web and appends sources
- **GEMINI.md Context** — Loads project-specific context from `GEMINI.md` (walks up to the nearest `.git` root)
- **Single-shot mode** — Designed for non-interactive, scriptable use

## Installation

```bash
git clone https://github.com/heki1224/gemini-cli-rs
cd gemini-cli-rs
cargo build --release
# Binary is at target/release/gemini
```

Optionally, copy to your PATH:

```bash
cp target/release/gemini ~/.local/bin/gemini
```

## Usage

```bash
# Set your API key
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

## GEMINI.md

Place a `GEMINI.md` file in your project root (or any parent directory up to `.git`) to inject system-level context into every request. Useful for project-specific instructions.

```
your-project/
├── .git/
├── GEMINI.md   ← loaded automatically
└── src/
```

## License

MIT — see [LICENSE](LICENSE)
