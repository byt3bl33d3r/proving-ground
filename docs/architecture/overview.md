---
type: Concept
title: Architecture overview
description: Crate layout (core, runtime, server, cli), the harness directory, and how one request flows through them.
tags: [architecture, crates]
status: stable
code_refs: [crates/{{project-name}}-core/src/lib.rs, crates/{{project-name}}-runtime/src/lib.rs, crates/{{project-name}}-server/src/lib.rs, crates/{{project-name}}-cli/src/lib.rs, harness]
---

# Architecture overview

Four crates ship; everything else verifies or observes them.

| Crate | Holds |
| --- | --- |
| `crates/<project>-core` | `types` (newtypes, errors), `platform` (Clock, Rng, HashMap alias, `net`, `buggify!`), `repo` (`ItemRepo`, in-memory store), `domain` (`ItemService`), `fields` (telemetry names) |
| `crates/<project>-runtime` | `config` (figment: defaults, `app.toml`, `APP_*` env) and `telemetry` (subscriber, OTLP providers, panic hook) |
| `crates/<project>-server` | `router(state)`, `AppState`, middleware, handlers; a thin axum `main.rs` |
| `crates/<project>-cli` | typed HTTP `client`, `commands`, `output`; a thin clap `main.rs` |

Two seams keep side effects in one place: environment and configuration are read only through
`runtime::config`, and user-facing CLI output goes only through `cli::output` (clippy bans
`std::env::var` and `println!` elsewhere, naming the seam).

`harness/` holds `checks` (architecture and e2e tests), `dst` and `fuzz` (separate workspaces),
`lints` (dylint) and `stack` (the local Victoria compose file).

A request: `SetRequestId` (UUID v7 from the platform clock and RNG), then the `http_request`
span, the duration metric, JSON error bodies, the timeout and the panic catcher, then the handler,
`ItemService` (`create_item` span) and `ItemRepo`. Dependencies only point down: see
[layers](/architecture/layers.md). Each worktree runs its own stack: see
[port allocation](/architecture/port-allocation.md).
