# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| latest (`main`) | :white_check_mark: |

Only the latest release on `main` is actively maintained. Please update to the latest version before reporting a vulnerability.

## Reporting a Vulnerability

**Please do not report security vulnerabilities via public GitHub Issues.**

To report a vulnerability, use one of the following:

- **GitHub Private Vulnerability Reporting** — [Report a vulnerability](https://github.com/heki1224/gemini-cli-rs/security/advisories/new) (preferred)
- **Email** — Contact the maintainer directly via the email on the [GitHub profile](https://github.com/heki1224)

### What to include

- A description of the vulnerability and its potential impact
- Steps to reproduce or a proof-of-concept
- Any suggested mitigations (optional)

### Response timeline

This project is maintained by a solo developer on a best-effort basis.
Response times are not guaranteed, but I will do my best to:

- Acknowledge the report as soon as possible
- Provide a status update when a fix is being worked on

Thank you for taking the time to report responsibly.

## Scope

This tool calls the [Gemini API](https://ai.google.dev/) and reads local files (`GEMINI.md`). Key areas of concern:

- **API key handling** — The key is passed via the `x-goog-api-key` header and read from the `GEMINI_API_KEY` environment variable. It is never written to disk or logged.
- **Local file access** — Only `GEMINI.md` files found by walking up from the current directory to the nearest `.git` root are read.

## Out of Scope

- Vulnerabilities in the Gemini API itself (report to [Google](https://bughunters.google.com/))
- Issues in third-party dependencies (report upstream; we will track via `cargo audit`)
