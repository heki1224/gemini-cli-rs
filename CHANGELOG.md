# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.6](https://github.com/heki1224/gemini-cli-rs/compare/v0.1.5...v0.1.6) - 2026-03-21

### Fixed

- update aws-lc-sys and rustls-webpki to resolve security advisories
- handle wrong MCP parameter names with alias fallback and helpful errors

### Other

- update dependencies (clap 4.6, anstream 1.0, zerocopy 0.8.47, etc.)
- document prompt parameter aliases in MCP tool reference

## [0.1.5](https://github.com/heki1224/gemini-cli-rs/compare/v0.1.4...v0.1.5) - 2026-03-20

### Added

- add high-performance model and thinking flag to MCP tool

### Fixed

- prevent OOM via special files and unbounded stdin lines

### Other

- add .agent-context/ to .gitignore
- update README and context files for dual-model support
- add AI agent context files (AGENTS.md, CLAUDE.md, GEMINI.md)

## [0.1.4](https://github.com/heki1224/gemini-cli-rs/compare/v0.1.3...v0.1.4) - 2026-03-11

### Added

- load global ~/.gemini/GEMINI.md as system prompt context

### Other

- update quinn-proto to 0.11.14 (RUSTSEC-2026-0037)
- skip cargo-dist CI staleness check for custom release steps
- fix cargo-dist release workflow to use gh release edit
- use RELEASE_PLZ_TOKEN for GitHub Release creation in cargo-dist
- replace release-plz release command with custom tag workflow

## [0.1.3](https://github.com/heki1224/gemini-cli-rs/compare/v0.1.2...v0.1.3) - 2026-03-08

### Fixed

- harden input validation and add unsafe_code prohibition
- address PR review feedback
- apply secure coding improvements

### Other

- add workflow_dispatch trigger to release-plz workflow
- add Japanese README and language switcher
- update Cargo.lock dependencies
- limit release triggers to feat/fix/perf/refactor

## [0.1.2](https://github.com/heki1224/gemini-cli-rs/compare/v0.1.1...v0.1.2) - 2026-03-08

### Other

- add pre-built binary download instructions and remove unversioned Rust badge

## [0.1.1](https://github.com/heki1224/gemini-cli-rs/compare/v0.1.0...v0.1.1) - 2026-03-08

### Fixed

- add git_only=true to release-plz config for publish=false package ([#9](https://github.com/heki1224/gemini-cli-rs/pull/9))
