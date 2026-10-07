---
type: Guide
title: CI workflows
description: The GitHub Actions workflows (ci, perf, hardening, gardening), what each job runs, how failures are reported, and how to run workflows locally.
tags: [workflow, ci, github-actions, act, issues]
status: stable
code_refs: [.github/workflows/ci.yml, .github/workflows/perf.yml, .github/workflows/hardening.yml, .github/workflows/gardening.yml, justfile]
---

# CI workflows

Workflows hold no logic. Every job checks out the code (all actions are pinned by commit SHA),
installs the tools in `mise.toml` with `jdx/mise-action`, restores the Rust cache, runs one
`just` recipe and uploads `target/harness/` plus nextest's JUnit report, which is also published
as a job summary. Whatever passes locally passes in CI; fix logic in recipes, never in YAML.

| Workflow | Trigger | Jobs |
| --- | --- | --- |
| `ci.yml` | pull requests, pushes to main | `check` on ubuntu and macOS (`just check`); `ci` on ubuntu (`just ci-deps` installs Valgrind and Kani, then `just ci`; on a PR, Gungraun compares against the base commit); `e2e` (`just up`, `just e2e`, then always `just down`); `knowledge` (`just knowledge`, and on a PR `just knowledge-pr`, which requires a `docs/log.md` entry and comments with the changed concepts) |
| `perf.yml` | manual | `just perf`; uploads remarks, Criterion results and build timings; never fails on numbers |
| `hardening.yml` | nightly and manual | one job per recipe: each fuzz target for 10 minutes, DST in ten 10,000-seed shards (`START`), ASan, TSan, Kani full, mutants. A failed job runs `just report-failure`, which opens (or comments on) a `hardening: <recipe> failed` issue with the harness JSON and the repro lines |
| `gardening.yml` | weekly and manual | `just gardening --issue`: okf drift and staleness, concepts still unverified after 30 days, plans active for over 30 days, `#[expect]`s older than 90 days, `docs/generated/` drift and outdated dependencies, kept in one `gardening report` issue |

What each tier runs is in [check tiers](/hardening/tiers.md).

## When CI fails

Download the job's `harness-*` artifact and read `target/harness/<check>.json` (`summary`,
`repro`), then run the repro locally. Hardening artifacts also hold fuzz crashes and DST failure
logs: see [fuzzing](/hardening/fuzzing.md) and [DST](/testing/dst.md).

## Running workflows locally

- `just ci-validate`: `wrkflw validate` and `actionlint` on `.github/workflows/`. Run it whenever
  a workflow changes; the actionlint hook step also runs on commit.
- `just ci-local <job>`: one job in Docker with `act` (`ubuntu-latest` mapped to
  `catthehacker/ubuntu:act-latest`). Caching behaves differently, artifacts stay local, and steps
  that call the GitHub API need `-s GITHUB_TOKEN=...`.

The same checks run before a push: [git hooks](/workflows/git-hooks.md).
