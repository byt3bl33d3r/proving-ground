---
type: Decision
title: Adding a UI
description: How a future web, Tauri or egui UI joins the harness, and the before-and-after recording every UI change must ship with.
tags: [decision, ui, browser, evidence, recording]
status: stable
---

# Adding a UI

**Status.** Planned, not built. The template ships no UI; agents see the app through telemetry,
API snapshots and CLI transcripts. Build none of this until a plan needs a UI.

## Options by UI kind

| UI kind | Agent visibility | CI tests | Recording backend for `just record` |
| --- | --- | --- | --- |
| Web pages served by axum, or Leptos/Dioxus web | `chrome-devtools-mcp`: screenshots, DOM snapshots, console, network and performance traces | Playwright end-to-end tests | Playwright video (`video: "on"`) plus trace files (`trace: "on"`) |
| Tauri desktop | A Tauri MCP plugin registered in debug builds only (it can run arbitrary JS) | Tauri WebDriver (`tauri-driver`) | Xvfb plus `ffmpeg -f x11grab` while the WebDriver journey runs (Linux only) |
| egui native | `egui_kittest` via the accessibility tree | `egui_kittest` image snapshots | One frame per journey step (headless wgpu), stitched into MP4/GIF with `ffmpeg` |

## How a UI joins the harness

- A new crate under `crates/` that depends only on `cli::client` or the HTTP API, never on core
  directly, plus a matching edge in the [layer rules](/architecture/layers.md).
- `just ui-test` and `just ui-snap`, added to tier 1, writing results to `target/harness/`.
- The browser MCP server in `.mcp.json` and `.codex/config.toml` through `just mcp browser`, so it
  targets this worktree's app URL from `.harness/app.json`.
- W3C `traceparent` propagated from the page to the server, so one click can be followed into the
  backend span tree with `just trace`.
- An extra budget such as "no console errors during `just ui-test`".

## Before/after recordings are required for every UI change

- **One interface.** `just record <journey> before|after` (needs `just up`) works the same for
  every UI kind. Journeys are code (a Playwright spec, a WebDriver test, an `egui_kittest` test) in
  `harness/evidence/` or next to the plan.
- **Same outputs.** Each recording writes `.harness/evidence/<plan>/<label>.mp4` (or `.gif`) plus a
  JSON bundle: every `request_id` and `trace_id` logged during the run, any WARN/ERROR lines,
  browser console errors and failed requests, and the `just budgets` result.
- **Bug-fix loop.** Reproduce and write the journey; `just record <journey> before` must show the
  failure; write the failing test and the fix; `just record <journey> after` must be clean; open
  the PR with both bundle summaries. This becomes an AGENTS.md rule when the UI lands.
- **CI reproduces it.** `evidence.yml` records every changed journey on the base branch and the PR
  head, uploads the videos and bundles, and comments with a diff of the two bundles.
- **Enforced, not remembered.** The `evidence-required` check fails a PR that changes a UI crate
  without a new or updated journey: `UI change without evidence. Add or update a journey and run
  just record <journey> before|after; see docs/testing/evidence.md.` Pure refactors opt out with
  the human-applied `no-visual-change` label.
- **Budgets apply.** A journey also fails on console errors or a broken latency
  [budget](/observability/budgets.md), so the video is never the only signal.
