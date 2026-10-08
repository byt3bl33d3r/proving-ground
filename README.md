<div align="center">

# proving-ground

**A cargo-generate template for Rust services built to be worked on by coding agents.**

Every change proves itself: tests, deterministic simulation, fuzzing, proofs, sanitizers and
latency budgets, all from one `just` menu that git hooks, CI and agents share.

[![template-ci](https://github.com/byt3bl33d3r/proving-ground/actions/workflows/template-ci.yml/badge.svg)](https://github.com/byt3bl33d3r/proving-ground/actions/workflows/template-ci.yml)

</div>

## Why

Agents write code faster than people can review it, so the leverage moves from reading lines to
building the environment that checks them: the harness. OpenAI's
[Harness engineering](https://openai.com/index/harness-engineering/) describes shipping a codebase
with no hand-written code by investing in that environment. Datadog's
[Closing the verification loop](https://www.datadoghq.com/blog/ai/harness-first-agents/) argues
for building the harness first, with deterministic simulation, formal checks and production
telemetry making verification fast and automatic. This template is that harness for a Rust
service, ready on day one.

## What you get

- **A layered workspace.** `core` (domain, typed errors, platform seams for time and randomness),
  `runtime` (config, telemetry), an axum `server` and a clap `cli`. Architecture tests keep
  dependencies pointing down.
- **One menu.** Every check is a `just` recipe, and hooks, CI and agents call the same ones. Each
  writes `target/harness/<check>.json` with a verdict and a repro command.
- **A telemetry stack per git worktree.** VictoriaMetrics, VictoriaLogs and VictoriaTraces over
  OTLP, on ports hashed from the worktree path, so parallel agents never collide. MCP servers give
  agents the same view.
- **Deterministic simulation.** turmoil network faults plus FoundationDB-style `buggify!` fault
  injection, seeded and reproducible: `just dst SEED=<n>`.
- **Hardening.** Fuzz targets, Kani proofs, Miri, ASan and TSan, mutation testing, a coverage
  ratchet, a panic audit of the release build, assembly snapshots and Gungraun instruction-count
  gates.
- **Enforced conventions.** Clippy pedantic and restriction lints with remediation messages, bans
  on raw time, randomness and `println!`, project dylint lints for telemetry naming, and hk git
  hooks that agents cannot bypass.
- **Project memory.** `docs/` is an OKF knowledge bundle agents search before acting and update
  after; `AGENTS.md` is a short map into it. Claude Code and Codex configs ship with it.

## Quick start

Requires [mise](https://mise.jdx.dev), Docker, and macOS or Linux.

```bash
cargo generate byt3bl33d3r/proving-ground --name my-service
cd my-service && git add -A && git commit -m "Initial commit"
mise trust && mise install
just bootstrap   # lockfiles, git hooks, first build
just up          # telemetry stack + server; prints URLs as JSON
just check       # tier 0
```

Then `just --list` for the menu and `AGENTS.md` for how agents work in the project.

## Tiers

| Tier | Recipe | Runs | Adds |
| --- | --- | --- | --- |
| 0 | `just check` | every commit | fmt, clippy, tests, unused deps, project memory, justfile lint |
| 1 | `just ci` | every PR | cargo-deny, dylint, coverage gate, panic audit, asm snapshots, Gungraun, Miri, Kani quick, 200 DST seeds |
| 2 | `just perf` | manual | optimization remarks, llvm-mca, Criterion, build timings; informational |
| 3 | `just harden` | nightly | fuzzing, ASan, TSan, Kani full, 100k DST seeds, mutation testing |
| 4 | `just release` | before a release | public API diff |

## Layout

```text
crates/   core, runtime, server, cli        # what ships
harness/  checks, dst, fuzz, lints, stack   # what verifies and observes it
docs/     OKF knowledge bundle              # what agents read and write
```

## Working on the template

The repository root is the generated project root, with Liquid placeholders in file and
directory names, so it does not compile here. `just template-ci` generates a project and runs the
full acceptance pass on it. How the template works, releases and upgrades:
[TEMPLATE_README.md](TEMPLATE_README.md).
