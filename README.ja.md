# gemini-cli-rs

[English](README.md) | 日本語

![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)

Rust 製の Gemini CLI。Claude Code の [MCP](https://modelcontextprotocol.io/) ツールとして利用することに特化しています。

## 特徴

- **MCP サーバーモード** — ネイティブ MCP サーバーとして動作（`--mcp-server`）、Claude Code と直接連携
- **SSE ストリーミング** — Gemini API からのレスポンスをリアルタイムにストリーミング
- **Google Search Grounding** — 常時有効。出典は CLI モードでは stderr に、MCP モードではレスポンス本文に含まれる
- **GEMINI.md コンテキスト** — `GEMINI.md` からプロジェクト固有のコンテキストを自動読み込み（cwd から `.git` ルートまで探索）
- **シングルショットモード** — 非インタラクティブ・スクリプト用途向けの設計

## 前提条件

- Gemini API キー（`GEMINI_API_KEY`） — [Google AI Studio](https://aistudio.google.com/apikey) で取得

## インストール

### ビルド済みバイナリをダウンロード

macOS・Linux 向けのビルド済みバイナリは [Releases ページ](https://github.com/heki1224/gemini-cli-rs/releases/latest) から入手できます。

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

### ソースからビルド

```bash
git clone https://github.com/heki1224/gemini-cli-rs
cd gemini-cli-rs
cargo build --release
cp target/release/gemini ~/.local/bin/gemini
```

## 使い方

### CLI モード

```bash
# API キーを環境変数に設定
export GEMINI_API_KEY="your-api-key"

# プロンプトを送信
gemini -p "フランスの首都はどこですか？"

# 別のモデルを使用
gemini -m gemini-2.5-pro -p "Rust の借用チェッカーを説明して"
```

グラウンディングの出典は、レスポンス後に **stderr** へ出力されます。

### MCP サーバーモード

MCP モードでは Claude Code がバイナリを自動的に起動するため、通常は直接実行する必要はありません。登録手順は [MCP セットアップ](#mcp-セットアップclaude-code) を参照してください。

```bash
# 手動起動（デバッグ用）
GEMINI_API_KEY="your-api-key" gemini --mcp-server
```

グラウンディングの出典はレスポンス本文に**含まれます**（stderr ではありません）。

## オプション

| フラグ | 説明 | デフォルト |
|--------|------|-----------|
| `-p, --prompt` | 送信するプロンプト（CLI モードでは必須） | — |
| `-m, --model` | 使用するモデル | `gemini-3-flash-preview` |
| `--mcp-server` | MCP サーバーとして起動（JSON-RPC 2.0 over stdio） | — |

API キーは `GEMINI_API_KEY` 環境変数からのみ読み込まれます（`--api-key` フラグはありません）。

## MCP セットアップ（Claude Code）

このツールは Claude Code 内で MCP サーバーとして使用し、Google Search Grounding 付きの Gemini をセカンダリ AI アシスタントとして提供することを目的として設計されています。

### 1. バイナリをビルド

```bash
cargo build --release
```

### 2. MCP サーバーとして登録

```bash
claude mcp add gemini /path/to/target/release/gemini --scope user -- --mcp-server
```

`--scope user` で全プロジェクト共通、`--scope project` で現在のプロジェクト限定になります。

### 3. API キーを設定

シェルの設定ファイル（`~/.zshrc`、`~/.bashrc` など）に `GEMINI_API_KEY` を追加します：

```bash
export GEMINI_API_KEY="your-api-key"
```

### 4. Claude Code を再起動して確認

```bash
claude mcp list
```

登録後、`ask_gemini_mcp` ツールが利用可能になります。Claude Code はプロンプトを Gemini にルーティングし、リアルタイムの Google Search グラウンディングを提供します。

> **注意:** バイナリはプロトコルメッセージを stdout に、すべてのログを stderr に出力します。デバッグ出力を確認したい場合は stderr をリダイレクトしないでください。

## GEMINI.md

カレントディレクトリ（または親ディレクトリ）に `GEMINI.md` ファイルを置くと、すべてのリクエストにシステムレベルのコンテキストが自動的に注入されます。最も近い `.git` ディレクトリまで探索し、`.git` が見つからない場合はファイルシステムのルートまで遡ります。プロジェクト固有の指示を記述するのに便利です。

```
your-project/
├── .git/
├── GEMINI.md   ← 自動的に読み込まれる
└── src/
```

> **注意:** 1 MB を超えるファイルは無視されます。

## ライセンス

MIT — [LICENSE](LICENSE) を参照
