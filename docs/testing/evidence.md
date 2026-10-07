---
type: Guide
title: UI evidence
description: Planned before-and-after recordings of UI journeys, and what to do when the evidence-required check fails.
tags: [testing, ui, browser, playwright, evidence, recording]
status: draft
---

# UI evidence

**Planned, not built.** The template ships no UI, so there are no browser or UI end-to-end tests
yet: `just e2e` drives the API through the CLI client (see the [test strategy](/testing/strategy.md)).
The design, including which tool each UI kind uses (Playwright for web, WebDriver for Tauri,
`egui_kittest` for egui), is the [Adding a UI](/decisions/adding-a-ui.md) decision. This page
becomes the how-to once the first UI lands.

## When `evidence-required` fails

The check fails a PR that changes a UI crate without adding or updating a journey:

1. Write or update the journey (a Playwright spec, a WebDriver test or an `egui_kittest` test) in
   `harness/evidence/` or next to the plan.
2. `just up`, then `just record <journey> before` on the base branch: it must show the problem.
3. Write the failing test and the fix; `just record <journey> after` must be clean.
4. Put both bundle summaries (request and trace ids, WARN/ERROR lines, console errors, failed
   requests, the `just budgets` result) in the PR.

Pure refactors opt out with the `no-visual-change` label, which only a human applies.
