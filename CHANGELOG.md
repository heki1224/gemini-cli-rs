# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
