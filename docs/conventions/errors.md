---
type: Guide
title: Errors and exit codes
description: Error types per layer, the JSON error body of the API, and the CLI exit codes 0 to 4.
tags: [conventions, errors, cli, exit-codes]
status: stable
code_refs: [crates/{{project-name}}-cli/src/error.rs, crates/{{project-name}}-server/src/error.rs]
---

# Errors and exit codes

- Libraries use `thiserror` enums (`ItemError`, `ValidationError`, `RepoError`, `ConfigError`,
  `TelemetryError`); binaries use `anyhow` at the edge. `ItemError::code()` gives the stable code.
- Every API error body is `{ "error": { "code", "message", "request_id" } }`. Agents grep logs by
  `request_id` (`just logs <request_id>`).
- The CLI never calls `std::process::exit` (clippy bans it): `main` returns `ExitCode`, and
  `CliError::exit_code` is the one mapping:

| Code | Meaning |
| --- | --- |
| 0 | success |
| 1 | usage error (bad arguments, rejected input) |
| 2 | not found |
| 3 | server error |
| 4 | server unreachable |

With `--output json` (the default when stdout is not a terminal) errors are JSON on stderr. See
[lints](/conventions/lints.md) for the rules that keep panics out.
