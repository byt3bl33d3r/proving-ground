---
type: Concept
title: Architecture overview
description: Crate layout (core, runtime, server, cli), the harness directory, and how one request flows through them.
tags: [architecture, crates]
status: stable
code_refs: [crates/demo-app-core/src/lib.rs, crates/demo-app-server/src/lib.rs]
---

# Architecture overview

Four crates ship; everything else verifies or observes them.

| Crate | Holds |
| --- | --- |
| `crates/<project>-core` | `types` (newtypes, errors), `platform` (Clock, Rng, HashMap alias, `net`, `buggify!`), `repo` (`ItemRepo`, in-memory store), `domain` (`ItemService`), `fields` (telemetry names) |
| `crates/<project>-runtime` | `config` (figment: defaults, `app.toml`, `APP_*` env) and `telemetry` (subscriber, OTLP providers, panic hook) |
| `crates/<project>-server` | `router(state)`, `AppState`, middleware, handlers; a thin axum `main.rs` |
| `crates/<project>-cli` | typed HTTP `client`, `commands`, `output`; a thin clap `main.rs` |

`harness/` holds `checks` (architecture and e2e tests), `dst` and `fuzz` (separate workspaces),
`lints` (dylint) and `stack` (the local Victoria compose file).

A request: `SetRequestId` (UUID v7 from the platform clock and RNG), then the `http_request`
span, the duration metric, JSON error bodies, the timeout and the panic catcher, then the handler,
`ItemService` (`create_item` span) and `ItemRepo`. Dependencies only point down: see
[layers](/architecture/layers.md). Each worktree runs its own stack: see
[port allocation](/architecture/port-allocation.md).
