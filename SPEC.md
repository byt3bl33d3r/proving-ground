# Rust Agent Scaffold: Build Instructions for a cargo-generate Template

Oct 7, 2026 · @Jok

## 1. How to use this document

Give this whole document to a coding agent (Claude Code or Codex) as its task spec. The agent builds a **cargo-generate template repository**. A developer then runs `cargo generate --path <template-repo>` and gets a Rust workspace where every harness piece works after `mise install && just bootstrap && just up`.

Rules for the building agent:

1. Build the template, not a project. Every project-specific name is a Liquid placeholder (Section 3).
2. Verify every crate version, tool flag and config schema against current docs while building, then pin exact versions. Versions named here are floors known as of October 2026.
3. If a tool behaves differently from what this document says, follow the tool and record the difference in `docs/template/notes.md`. Do not silently improvise.
4. Stop and ask the human before any fallback or deviation this document does not already allow, for example when a tool in Section 13's verify list turns out unusable. Everything else is decided, and Section 13 records the answers already given.
5. Finish only when every box in Section 12 is ticked.

### Locked decisions

| Area | Decision |
| --- | --- |
| Project shape | Layered Cargo workspace: library crates, an axum service binary, a clap CLI binary |
| Git hooks | hk (v2, Pkl config) |
| Tasks and tools | `just` for every task; `mise` pins tool versions; workz gives each git worktree its own port range and compose project |
| Database | None. In-memory repository behind a trait, so a DB can be added later |
| Hardening | Fuzzing, Kani, DST (turmoil + mad-turmoil + BUGGIFY), Miri, sanitizers, coverage, asm and panic audits, Gungraun. All wired into CI tiers |
| Telemetry | `tracing` → OTLP → VictoriaMetrics, VictoriaLogs, VictoriaTraces, one throwaway stack per git worktree, plus a local JSON log file |
| Agents | Claude Code and Codex. `AGENTS.md` is the single source of truth; `CLAUDE.md` imports it |
| CI | GitHub Actions workflows that only call `just` recipes; `act` and `wrkflw` validate them locally |
| Toolchain | Pinned stable for building and testing; a pinned nightly used only by analysis recipes |

### Design principles

1. **One recipe, three callers.** Every check is a `just` recipe. Hooks, CI and agents call the same recipes, so they can never disagree.
2. **Machine-readable first.** Recipes an agent reads print JSON or a one-line verdict. Human-pretty output is a flag, not the default.
3. **Deterministic signals gate; noisy signals inform.** Instruction counts, coverage, asm and proofs can fail a build. Wall-clock benchmarks and optimization remarks never do.
4. **Every failure message carries its fix.** Lint, hook and test failures name the rule, the reason and the exact remediation.
5. **A map, not a manual.** `AGENTS.md` stays near 100 lines and points into `docs/`index.md, the OKF knowledge bundle.
6. **Everything per worktree.** No shared ports, stacks, logs or state between parallel agents.

### Build order

Build a real project named `demo-app` first and convert names to placeholders last; a template full of `{{...}}` cannot be compiled while you work. Each milestone ends with a green `just check` before the next begins.

1. **Skeleton:** workspace `Cargo.toml`, toolchain, `mise.toml`, `justfile` with `check`, lint policy, four empty crates that compile.
2. **Vertical slice:** the items domain in `core`, router in `server`, commands in `cli`, with unit, snapshot and `trycmd` tests. Each crate starts under about 200 lines of non-test code; do not pad crates to fill the layout.
3. **Runtime:** config loading and telemetry to stderr and the JSON file.
4. **Harness:** compose stack, `just up`/`down`, OTLP export, query recipes, budgets, e2e tests.
5. **Hooks:** `hk.pkl`, install via `just bootstrap`, protected-files and secret scanning.
6. **Enforcement:** `clippy.toml` bans, `deny.toml`, architecture tests, dylint lints.
7. **Hardening:** DST workspace, fuzz targets, Kani, Miri, coverage, asm and panic audits, Gungraun, sanitizers, tiers 1-3.
8. **Agent setup:** `AGENTS.md`, `CLAUDE.md`, Claude Code and Codex configs, `docs/` as an OKF bundle (okf init) with the okf-memory MCP server.
9. **Templating:** replace names with placeholders, write `cargo-generate.toml`, add `template-ci.yml`, then confirm that generating `demo-app` reproduces milestone 8 exactly.

## 2. Template repository layout

The template repo root is the generated project root. Files only the template repo needs are listed in `cargo-generate.toml` `ignore` so they never reach generated projects. Names in `{{...}}` are Liquid placeholders that cargo-generate renders in file and directory names too.

```text
.
├── cargo-generate.toml            # template config (Section 3)
├── template-hooks/post.rhai       # prints next steps after generation
├── TEMPLATE_README.md             # template-repo docs (ignored on generate)
├── Cargo.toml                     # workspace: members = crates/*, harness/checks; [workspace.lints]
├── rust-toolchain.toml            # pinned stable + components
├── mise.toml                      # tool versions + [env]: PROJECT_NAME, NIGHTLY, COV_MIN_REGIONS
├── justfile                       # every task, harness variables, MCP launcher
├── hk.pkl                         # git hooks + agent stop-hook steps
├── clippy.toml                    # thresholds + bans with remediation reasons
├── deny.toml                      # licenses, advisories, bans, sources
├── rustfmt.toml
├── .workz.toml                    # worktree isolation: port range size, hooks, sync overrides
├── .config/nextest.toml           # profiles: default, ci (JUnit)
├── .cargo/config.toml
├── .gitignore                     # target/, .harness/
├── AGENTS.md                      # ~100-line map (Section 10)
├── CLAUDE.md                      # imports AGENTS.md + Claude-specific notes
├── .claude/settings.json          # permissions + Stop hook
├── .mcp.json                      # Claude Code MCP servers (Victoria via just mcp, okf-memory)
├── .codex/config.toml             # Codex config + the same MCP servers
├── crates/                        # only code that ships
│   ├── {{project-name}}-core/     # types, errors, domain services, repository, platform seams
│   ├── {{project-name}}-runtime/  # config loading + telemetry init (used by binaries)
│   ├── {{project-name}}-server/   # lib (router, state) + axum binary
│   └── {{project-name}}-cli/      # lib (HTTP client, commands, output) + clap binary
├── harness/                       # everything that verifies or observes what ships
│   ├── checks/                    # workspace member, publish = false: architecture + e2e tests
│   ├── dst/                       # separate workspace: turmoil DST, enables the sim feature
│   ├── fuzz/                      # separate workspace: cargo-fuzz (always run with --fuzz-dir)
│   ├── lints/                     # separate workspace: dylint lints, own pinned nightly
│   └── stack/compose.yaml         # local VictoriaMetrics + VictoriaLogs + VictoriaTraces
├── docs/                          # OKF v0.2 bundle: every file is a concept (Section 10)
│   ├── index.md  log.md           # root progressive-disclosure index + dated change log
│   ├── architecture/  observability/  testing/  hardening/  performance/  conventions/
│   ├── decisions/                 # ADRs (type Decision), e.g. adding-a-ui.md
│   ├── plans/                     # exec plans (type Plan, status: active | completed)
│   ├── template/notes.md          # deviations and verified values
│   └── generated/                 # machine output, not concepts: crate graph, API docs, allowlists
└── .github/
    ├── CODEOWNERS                 # protected files (Section 9)
    └── workflows/  ci.yml  perf.yml  hardening.yml  gardening.yml  template-ci.yml (ignored on generate)
```

**Why `harness/` sits outside `crates/`:** `crates/` holds only what ships, and `harness/` holds everything that verifies or observes it, so the repo root is just config, `crates/`, `harness/`, `docs/` and `.github/`. `harness/dst` is its own workspace because Cargo unifies features across a workspace: if a member enabled `core`'s `sim` feature, `cargo test --workspace` would build the real crates with the simulated clock and network. `harness/fuzz` is separate for the same reason, and `harness/lints` because dylint needs its own nightly. Separate workspaces have their own `Cargo.lock` and `target/`, and `just check` runs fmt and clippy in each of them. Tools that default to root-level directories are pointed at `harness/` by the `just` recipes: `cargo fuzz --fuzz-dir harness/fuzz`, dylint via `[workspace.metadata.dylint]`, and `docker compose -f harness/stack/compose.yaml`. `deploy/` is left free for real deployment files (a Dockerfile, manifests) if the project ever needs them.

There is no `scripts/` directory: every piece of logic is a `just` recipe (multi-line ones use a shebang). Anything that outgrows a screenful moves to a Rust test or binary in `harness/checks`, not to a shell file.

`Cargo.lock` is not in the template, because placeholder package names cannot be locked. `just bootstrap` generates it, and the generated project commits it. `.harness/` is created at runtime per worktree (`stack.json`, `app.json`, `env`, `logs/app.jsonl`, pid files) and is never committed.

## 3. cargo-generate configuration

Keep prompts to four. Everything else derives from `project-name` (builtin) and `crate_name` (builtin, snake\_case).

```toml
# cargo-generate.toml
[template]
cargo_generate_version = ">=0.23.0"
# Removed from generated projects
ignore = ["TEMPLATE_README.md", ".github/workflows/template-ci.yml", "target", ".harness"]
# Copied verbatim, never run through Liquid (they use {{ }} natively)
exclude = [".github/workflows/*.yml", "justfile", "harness/stack/**", "hk.pkl", "harness/lints/**"]

[placeholders.description]
type = "string"
prompt = "One-line project description"
default = "A Rust service"

[placeholders.gh_owner]
type = "string"
prompt = "GitHub owner or org (used in Cargo.toml repository URLs)"
default = "your-org"

[placeholders.license]
type = "string"
prompt = "License"
choices = ["MIT OR Apache-2.0", "Apache-2.0", "MIT", "Proprietary"]
default = "MIT OR Apache-2.0"

[placeholders.service_name]
type = "string"
prompt = "OTEL service.name (defaults to project name)"
default = "{{project-name}}"

[hooks]
post = ["template-hooks/post.rhai"]
```

### Liquid pitfalls the agent must handle

- **Native `{{ }}` collides with Liquid.** GitHub Actions (`${{ }}`), `just` interpolation, and some config formats use double braces. Files that only use native braces go in `exclude`. Files that need both a placeholder and native braces wrap the native parts in `{% raw %}...{% endraw %}`.
- **Excluded files cannot contain placeholders.** Recipes in excluded files must read the project name at runtime, for example `cargo metadata --format-version 1 --no-deps | jq -r '.workspace_members'` or a `PROJECT_NAME` value written into `mise.toml` (which is templated).
- **Verify the default placeholder self-reference** (`service_name` defaulting to `{{project-name}}`). If cargo-generate does not render placeholders inside defaults, drop that default and set `OTEL_SERVICE_NAME` from the project name in `mise.toml` instead.
- **Post-generation check:** the template's CI greps every generated file for leftover `{{`, `{%`, and mangled `${ }` sequences and fails if any are found outside excluded paths.

### Post-generation hook

`post.rhai` only prints next steps. It runs no commands, so users never need `--allow-commands`:

```text
Next steps:
  cd <project>
  mise trust && mise install
  just bootstrap      # lockfile, hk install, tool checks, first build
  just up             # per-worktree telemetry stack + server
  just check          # tier 0
```

### Toolchains

- `rust-toolchain.toml` pins an exact stable version (for example `1.9x.0`) with components `rustfmt`, `clippy`, `llvm-tools-preview`, `rust-src`.
- Analysis recipes call an exact nightly date stored once in `mise.toml` as `NIGHTLY = "nightly-YYYY-MM-DD"` and invoked as `cargo +$NIGHTLY ...`. Nothing outside those recipes uses nightly.
- `harness/lints/` has its own `rust-toolchain.toml` with the nightly that the installed dylint version requires.
- Edition 2024 for every crate. Set `rust-version` (MSRV) to the pinned stable.

### Template versioning

cargo-generate cannot update a project after generation. To keep upgrades possible by hand:

- Tag template releases (`v1.0.0`, ...). The templated `docs/template/notes.md` starts with `Generated from <template repo> at <tag>` (fill the tag from a `template_version` value in `cargo-generate.toml` `[template]` metadata, or a placeholder default if that is not supported).
- Keep harness behavior in `just` recipes, `hk.pkl` and config files rather than in crate code, so a project can diff those files against a newer template tag and copy changes across.
- `TEMPLATE_README.md` documents the upgrade procedure: generate the new tag into a temp directory with the same values, diff, and port changes.

## 4. Workspace and crate architecture

Four crates ship, and every dependency points down onto `core`. Crate boundaries enforce layering between crates; an architecture test enforces module layering inside `core`. Both fail on any edge not in the table below.

&#91;embedded content: crate layering · 4 shipping crates + 2 harness crates\]

Highlighted boxes are the two binaries; dashed boxes live in `harness/`. Arrows point from a crate to what it depends on.

| Crate | May depend on (internal) | Holds |
| --- | --- | --- |
| `core` | nothing | Modules `types` (newtypes, enums, `thiserror` errors), `platform` (`Clock`, `Rng`, `HashMap` alias, `net` re-export, `buggify!`; real impls, sim impls behind feature `sim`), `repo` (`ItemRepo` trait, `InMemoryItemRepo`), `domain` (services with `#[instrument]`, metric instruments), `fields` (telemetry field-name constants). No axum, no OTel SDK, no socket code |
| `runtime` | core | `config` (figment TOML + env, validation, port default `0`) and `telemetry` (subscriber, OTel providers, panic hook, `TelemetryGuard`, `Mode::Server` / `Mode::Cli`). Used only by binaries |
| `server` | core, runtime | Library: `router(state)`, `AppState`, middleware, handlers. Binary: `main.rs` under 80 lines. Feature `sim` forwards to `core/sim` |
| `cli` | core, runtime | Library: `client` (typed reqwest client), `commands`, `output`. Binary: `main.rs` under 80 lines |
| `harness/checks` (test-only member) | cli, core | `tests/arch.rs` (structure checks), `tests/e2e.rs` (against a running server) |
| `harness/dst` (separate workspace) | core and server, both with `sim` | turmoil DST scenarios |

**Inside `core`:** `types` uses nothing; `platform` uses `types`; `repo` uses `types` and `platform`; `domain` uses all three; every module may use `fields`. The architecture test scans `use crate::` paths to enforce this. If `core` passes about 3,000 lines, split along these module lines into separate crates; the module boundaries make that mechanical.

### Example domain

Ship one small, complete vertical slice so every harness feature has something real to observe: an **items** resource with `create`, `get`, `list` and `delete`. It must exercise spans, metrics, a validation error, a not-found error, and one deliberately slow path behind `buggify!` so DST and latency budgets have something to catch.

### Service (`server`) requirements

- axum (current 0.8.x) on tokio. Router built by `pub fn router(state: AppState) -> Router` so `harness/dst` and tests reuse it without a socket.
- Middleware, outermost first: `SetRequestIdLayer` (UUID v7), `TraceLayer` with a span named `http_request` carrying `http.request.method`, `http.route`, `request_id`, W3C `traceparent` extraction, `PropagateRequestIdLayer`, timeout, `CatchPanicLayer`.
- Error responses are JSON `{ "error": { "code", "message", "request_id" } }` so agents can grep logs by `request_id`.
- Binds `config.port` (default `0`). After binding, writes `.harness/app.json` with `{ "url", "pid", "started_at" }` and logs `server_listening` with the address.
- Emits a `startup` span covering config load to listening, so the 800 ms startup budget is checkable.
- `/readyz` returns 200 only after the router is serving and telemetry is initialized.

### CLI (`cli`) requirements

- clap 4 derive. Global flags: `--server-url` (default: read `.harness/app.json`, then env `APP_URL`), `--output json|text` (default `text` on a TTY, `json` otherwise), `-v/-q` mapped to `RUST_LOG`, and `--otel`.
- Subcommands mirror the API: `items create|get|list|delete`, plus `doctor`, which checks server reachability and prints the harness state as JSON.
- **Exit codes without `process::exit`:** the lint policy bans `std::process::exit`, so `main` returns `std::process::ExitCode`. One function maps the error type to a code: 0 ok, 1 usage error, 2 not found, 3 server error, 4 unreachable. Documented in `docs/conventions/errors.md`.
- **Telemetry:** `runtime::telemetry::init(Mode::Cli)` logs to stderr, and to the JSON file when `APP_LOG_JSON` is set. OTel export is off unless `--otel` is passed, so ordinary CLI runs never push spans to the stack.
- User-facing output goes only through `cli::output`, the single place allowed to print.
- Tested with `trycmd` transcripts in `crates/{{project-name}}-cli/tests/cmd/*.toml`.

### Workspace `Cargo.toml` essentials

- `members = ["crates/*", "harness/checks"]`, `exclude = ["harness/dst", "harness/fuzz", "harness/lints"]`, `resolver = "3"`, and `[workspace.package]` with edition, rust-version, license, repository.
- All third-party versions live in `[workspace.dependencies]`; member crates use `dep.workspace = true` only. `harness/dst` and `harness/fuzz` pin the same versions in their own manifests, and `tests/arch.rs` fails if they drift.
- OpenTelemetry crates (`opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `opentelemetry-appender-tracing`) and `tracing-opentelemetry` are pinned as one family, with a comment saying they upgrade together in one PR.
- `[profile.release]` with `debug = "line-tables-only"` so asm, profiles and backtraces map to source. `[profile.profiling]` inherits release with `debug = true`.

## 5. Telemetry with the Victoria stack

The app exports each signal over OTLP/HTTP straight to its Victoria backend, with no collector in between, and also writes a local JSON log file. The file is the agent's first stop; the Victoria stack is for queries across time, metrics and traces.

| Signal | Backend | Container port | OTLP/HTTP path | Query API |
| --- | --- | --- | --- | --- |
| Metrics | VictoriaMetrics single | 8428 | `/opentelemetry/v1/metrics` | PromQL/MetricsQL at `/api/v1/query` |
| Logs | VictoriaLogs | 9428 | `/insert/opentelemetry/v1/logs` | LogsQL at `/select/logsql/query` |
| Traces | VictoriaTraces | 10428 | `/insert/opentelemetry/v1/traces` | LogsQL at `/select/logsql/query`, Jaeger API at `/select/jaeger` |

Note that the metrics path has no `/insert` prefix. Paths are from the [VictoriaTraces OTLP docs](https://docs.victoriametrics.com/victoriatraces/data-ingestion/opentelemetry/) and a [Victoria stack walkthrough](https://dev.to/steph_baltus/go-sending-traces-logs-and-metrics-to-the-victoria-stack-3le4); the agent re-verifies them against the pinned image versions.

### Rust side (`runtime::telemetry`)

One `init(cfg) -> TelemetryGuard` function builds a single `tracing_subscriber::registry()` with these layers:

1. `EnvFilter` from `RUST_LOG`, default `info,{{crate_name}}_core=debug,{{crate_name}}_runtime=debug,{{crate_name}}_server=debug,{{crate_name}}_cli=debug,tower_http=debug,hyper=warn,h2=warn`.
2. JSON layer (`fmt::layer().json().with_current_span(true).with_span_list(true)`) writing to `.harness/logs/app.jsonl` through `tracing-appender` (non-blocking). Enabled when `APP_LOG_JSON` is set.
3. Compact human layer to stderr.
4. `tracing-opentelemetry` layer exporting spans.
5. `opentelemetry-appender-tracing` bridge exporting log events with `trace_id` and `span_id`.

Metrics use an OTel `MeterProvider` with a periodic OTLP exporter. Endpoints come from the standard env vars `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`, `OTEL_EXPORTER_OTLP_LOGS_ENDPOINT` and `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT` (full URLs including the paths above), protocol `http/protobuf`. When none are set, OTel export is off and only the JSON file and stderr remain, so tests and CI never need the stack.

Also required:

- Resource attributes: `service.name` (from the template placeholder), `service.version` (crate version), `deployment.environment=dev`, `worktree=<slot name>`.
- `TelemetryGuard` flushes and shuts down all providers on drop; `server` and `cli` hold it in `main`.
- A panic hook that emits `tracing::error!(panic.message, panic.location, backtrace)` before the default hook, so panics land in the JSON file and VictoriaLogs.
- Sampler `always_on` locally (`OTEL_TRACES_SAMPLER` respected).

**Modes.** `init` takes `Mode::Server` (all layers, OTel on when endpoints are set) or `Mode::Cli` (stderr, plus the JSON file when `APP_LOG_JSON` is set; OTel only with `--otel`). Every OTLP export is best-effort: if the stack is down, the app logs one warning and keeps running.

### Field and naming conventions

Put these in `core::fields` as `&'static str` constants and in `docs/observability/fields.md`. Lints enforce them (Section 9).

- OTel semantic-convention names where one exists: `http.request.method`, `http.route`, `http.response.status_code`, `error.type`.
- Domain fields in `snake_case`: `item_id`, `request_id`, `dst_seed`.
- Span names are `verb_noun`: `startup`, `create_item`, `load_config`.
- Failures always record `error = %e` and set `otel.status_code = "ERROR"`.
- Messages are constant strings; variable data goes in fields. `info!(item_id = %id, "item_created")`, never `info!("created item {id}")`.
- Metric instruments follow semconv: `http.server.request.duration` (histogram, seconds), plus domain counters such as `items.created`.

### `harness/stack/compose.yaml`

- Three services: `victoria-metrics`, `victoria-logs`, `victoria-traces`, each image pinned to an exact version tag (never `latest`).
- Ports published on `127.0.0.1` only, using `${VM_PORT}`, `${VL_PORT}`, `${VT_PORT}` from the harness variables (Section 6).
- Data on `tmpfs`; short retention flags (`-retentionPeriod=1d`). The stack is disposable by design.
- **No compose healthchecks.** The Victoria images are minimal, without a shell or curl, so a `healthcheck` command cannot run inside them. Readiness is checked from the host instead: `just up` runs `docker compose up -d`, then a private `wait-stack` recipe polls each service's `/health` endpoint with a 30-second timeout.
- VictoriaMetrics: enable Prometheus-style naming for OTLP metrics (verify the current flag, expected `-opentelemetry.usePrometheusNaming`) so queries read `http_server_request_duration_seconds_bucket`.
- No Grafana. VictoriaMetrics and VictoriaLogs serve a built-in UI at `/vmui`; VictoriaTraces has its own built-in UI (verify its path). `just ui` prints all three URLs.

### MCP servers for agents

Use the official VictoriaMetrics MCP servers, installed as binaries pinned in `mise.toml`:

| Server | Required env | Notes |
| --- | --- | --- |
| `mcp-victoriametrics` | `VM_INSTANCE_ENTRYPOINT`, `VM_INSTANCE_TYPE=single` | Metrics queries, metric discovery, docs search |
| `mcp-victorialogs` | `VL_INSTANCE_ENTRYPOINT` | LogsQL queries, field discovery, docs search |
| `mcp-victoriatraces` | verify env var name (expected `VT_INSTANCE_ENTRYPOINT`) | Trace search and lookup |

Env var names for the first two are from the [mcp-victorialogs](https://pkg.go.dev/github.com/VictoriaMetrics/mcp-victorialogs) and [mcp-victoriametrics](https://www.mdskills.ai/de/mcp-servers/mcp-victoriametrics) READMEs. Agent configs never call these binaries directly. They run `just mcp <metrics|logs|traces>`, a quiet recipe that computes this worktree's ports and then `exec`s the right binary with the right entrypoint (Section 6 explains why the ports are deterministic).

### `just` query recipes (always available, MCP or not)

| Recipe | Does |
| --- | --- |
| `just logs-errors` | `jq` over `.harness/logs/app.jsonl` for `ERROR`/`WARN`, last 50, compact |
| `just logs-request <request_id>` | Every log line for one request from the JSON file |
| `just q-logs '<LogsQL>'` | VictoriaLogs query, newline JSON, capped at 200 lines |
| `just q-metrics '<PromQL>'` | VictoriaMetrics instant query, result vector as JSON |
| `just q-traces '<LogsQL>'` | VictoriaTraces span search, capped at 200 lines |
| `just trace <trace_id>` | Full trace via the Jaeger API, flattened to one span per line with name, duration\_ms, parent, status |
| `just budgets` | Runs every budget query in `docs/observability/budgets.md` and prints pass/fail JSON |

### Canned queries in `docs/observability/queries.md`

The agent writes these after discovering the real field names with each component's field-names endpoint. Expected shapes:

```text
# VictoriaLogs (LogsQL): errors in the last 15 minutes for this service
_time:15m service.name:"<service>" severity:ERROR

# VictoriaMetrics (PromQL): p95 latency by route
histogram_quantile(0.95, sum by (le, http_route) (rate(http_server_request_duration_seconds_bucket[5m])))

# VictoriaTraces (LogsQL): startup span over the 800 ms budget
name:"startup" duration:>800ms
```

Cap every recipe and document the caps. Agents should query and aggregate, never dump raw logs into context.

## 6. Per-worktree harness

Each git worktree gets its own workz port range, its own Victoria stack, its own server process and its own `.harness/` directory. `just up` brings all of it up in under 60 seconds; `just down` removes all of it.

### Why deterministic port slots

Ports come from [workz](https://github.com/rohansx/workz) (Rust, pinned in `mise.toml`) instead of a homemade hash. `workz sync --isolated` gives each worktree its own 10-port range and a `COMPOSE_PROJECT_NAME`, written to a managed block in `.env.local`. Allocations are tracked in `~/.config/workz/ports.json`, so two worktrees never collide, and `workz done` releases them.

- **When it runs:** the agent's worktree-creation hook runs `workz sync --isolated --quiet` (Section 10), and `just bootstrap` runs it for the main checkout. Ports therefore exist before the agent session starts, which is what lets static MCP configs work: `just mcp` reads the same `.env.local` that `just up` will use.
- **Port use:** `PORT` is the first port of the range. The Victoria stack uses `PORT`, `PORT+1` and `PORT+2`; the rest of the range is spare. The server still binds port `0` and reports its address in `.harness/app.json`.
- **`.workz.toml`** (committed): `[isolation] base_port = 20000`, `port_range_size = 10`; `[hooks] pre_done = "just down"` so removing a worktree tears its stack down; `[sync.overrides] target = "ignore"` (see Build cache below).
- **Missing setup:** if `.env.local` has no `PORT`, `just up` and `just mcp` fail with `This worktree has no port range. Run: workz sync --isolated`.
- workz is young (0.x) and adds a dependency to every machine. If it is ever dropped, the fallback is the earlier hash-of-worktree-path scheme computed in the `justfile`.

### Harness variables (top of the `justfile`)

```text
set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-filename := ".env.local"          # workz writes PORT and COMPOSE_PROJECT_NAME here
set dotenv-load
set quiet                                    # no command echo; stdio MCP needs a clean stdout

project := env("PROJECT_NAME")               # from mise.toml [env]
root    := `git rev-parse --show-toplevel`
wt      := env("COMPOSE_PROJECT_NAME", "unsynced")
base    := env("PORT", "0")                  # 0 = workz has not run; port recipes refuse to start

export COMPOSE_PROJECT_NAME := project + "-" + wt   # prefix: two repos can share a branch name
export VM_PORT := base
export VL_PORT := shell('echo $(($1 + 1))', base)
export VT_PORT := shell('echo $(($1 + 2))', base)
export OTEL_EXPORTER_OTLP_METRICS_ENDPOINT := "http://127.0.0.1:" + VM_PORT + "/opentelemetry/v1/metrics"
export OTEL_EXPORTER_OTLP_LOGS_ENDPOINT    := "http://127.0.0.1:" + VL_PORT + "/insert/opentelemetry/v1/logs"
export OTEL_EXPORTER_OTLP_TRACES_ENDPOINT  := "http://127.0.0.1:" + VT_PORT + "/insert/opentelemetry/v1/traces"
export OTEL_EXPORTER_OTLP_PROTOCOL := "http/protobuf"
export OTEL_RESOURCE_ATTRIBUTES := "worktree=" + wt
export APP_LOG_JSON := root + "/.harness/logs/app.jsonl"

[private]
require-ports:
    if [ "$VM_PORT" = 0 ]; then echo "This worktree has no port range. Run: workz sync --isolated" >&2; exit 1; fi

# Print harness variables for a shell: eval "$(just env)"
env:
    env | grep -E '^(COMPOSE_PROJECT_NAME|VM_PORT|VL_PORT|VT_PORT|OTEL_|APP_LOG_JSON)=' | sed 's/^/export /'

# Start a Victoria MCP server for this worktree (called by .mcp.json and .codex/config.toml)
mcp kind: require-ports
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{kind}}" in
      metrics) VM_INSTANCE_ENTRYPOINT="http://127.0.0.1:$VM_PORT" VM_INSTANCE_TYPE=single exec mcp-victoriametrics ;;
      logs)    VL_INSTANCE_ENTRYPOINT="http://127.0.0.1:$VL_PORT" exec mcp-victorialogs ;;
      traces)  VT_INSTANCE_ENTRYPOINT="http://127.0.0.1:$VT_PORT" exec mcp-victoriatraces ;;  # verify var name
      *) echo "usage: just mcp metrics|logs|traces" >&2; exit 2 ;;
    esac
```

`PROJECT_NAME` comes from `mise.toml` `[env]` (templated), because the `justfile` is excluded from Liquid and uses `{{ }}` for its own interpolation. `shell()` and `dotenv-filename` need a recent `just`; pin it in `mise.toml`, and verify that values loaded from `.env.local` are visible to `env()` in variable assignments. `eval "$(just env)"` gives a human's shell the same values the recipes see.

### `just up`

1. Run `require-ports`. If any of the three ports is in use by something other than this compose project, fail with: `Port <p> is taken by <process>. Run: workz doctor --fix && workz sync --isolated, then restart your agent's MCP servers.`
2. `docker compose -f harness/stack/compose.yaml up -d`, then the private `wait-stack` recipe polls each service's `/health` from the host (30-second timeout).
3. Write `.harness/stack.json` (ports, UI URLs, OTLP endpoints) and `.harness/env` (the `just env` output, for tools that cannot run `just`).
4. `cargo build -p "$PROJECT_NAME-server"`, then start it in the background, stdout and stderr to `.harness/logs/server.out`, pid to `.harness/server.pid`.
5. The private `wait-ready` recipe polls `.harness/app.json` and `/readyz` with a 30-second timeout. On timeout it prints the last 40 lines of `server.out` and `just logs-errors`.
6. Print one JSON object: app URL, UI URLs, port range, and the commands `just logs-errors` and `just down`.

### Other harness recipes

| Recipe | Does |
| --- | --- |
| `just down` | Stop the server (pid file), `docker compose down -v`, delete `.harness/` |
| `just restart` | Rebuild and restart only the server, keep the stack and its data |
| `just status` | JSON: stack health, server pid alive, readyz, slot, ports |
| `just e2e` | Requires `just up`; runs the `e2e` tests in harness/checks against `.harness/app.json`, then `just budgets` |
| `just ui` | Print the three `/vmui` URLs for humans |
| `just harness-gc` | workz doctor --fix (orphaned port ranges), then remove compose projects whose worktree directory no longer exists |

### Budgets (checked by `just budgets`)

Start with these three, defined in `docs/observability/budgets.md`, each as a query plus a threshold:

- `startup` span under 800 ms.
- No `ERROR` log lines during `just e2e`.
- p95 `http.server.request.duration` under 50 ms for the items routes during `just e2e`.

### Build cache across worktrees

Keep a per-worktree `target/` (the default), so parallel agents never block on Cargo's build lock. workz symlinks target/ into new worktrees by default, which would bring the lock back, so .workz.toml turns that off with \[sync.overrides\] target = "ignore". Add `sccache` to `mise.toml` and set `RUSTC_WRAPPER=sccache` in `mise.toml` `[env]` so compiled dependencies are still shared.

## 7. Testing, determinism and DST

Every test is deterministic by construction: time, randomness, hashing order and network go through `core::platform`, and every randomized test prints the seed that reproduces it.

### Test kinds and where they live

| Kind | Tool | Location | Recipe |
| --- | --- | --- | --- |
| Unit | nextest, `tracing-test` | `#[cfg(test)]` modules in each crate | `just test` |
| Property | `proptest`, regressions committed | `core` | `just test` |
| API snapshots | `insta` (JSON, ids and timestamps redacted) | `server/tests` (router via `oneshot`, no socket) | `just test` |
| Golden logs | JSON layer into a `Vec<u8>` writer, normalized, `insta::assert_json_snapshot!` | `core/tests` | `just test` |
| CLI transcripts | `trycmd` | `cli/tests/cmd/*.toml` | `just test` |
| Architecture | `cargo metadata` + source scans | `harness/checks/tests/arch.rs` | `just test` |
| End to end | `cli::client` against `.harness/app.json` | `harness/checks/tests/e2e.rs` | `just e2e` |
| DST | turmoil + mad-turmoil + `buggify!` | `harness/dst` (own workspace) | `just dst` |

`just test` excludes the e2e binary with a nextest filter (`-E 'not binary(e2e)'`). `just e2e` runs only it and fails immediately with `No running server: run just up first` if `.harness/app.json` is missing, rather than skipping.

### nextest configuration (`.config/nextest.toml`)

- `default`: fail-fast, `slow-timeout = 30s`.
- `ci`: `fail-fast = false`, JUnit at `target/nextest/ci/junit.xml`, `retries = 0` (flaky tests are bugs, not retries).
- `harness/dst` has its own `.config/nextest.toml` with `slow-timeout = 5m` and `test-threads = num-cpus`.

### Deterministic time and order

- Async tests that involve timeouts or retries use `#[tokio::test(start_paused = true)]`.
- `core::platform` exports `HashMap`/`HashSet` aliases with a fixed hasher under feature `sim` and the std hasher otherwise. Clippy's `disallowed-types` bans the std types elsewhere (Section 9).
- `core::domain` gets time only from `Clock` and randomness only from `Rng`, both passed in through `AppState`.

### DST design (`harness/dst`)

1. **Workspace.** `harness/dst` is a separate workspace that depends on `core` and `server` by path with feature `sim` on. Because it is separate, the main workspace never builds with simulated time or network.
2. **Seed.** Read `DST_SEED` (default: from a seed list). Seed `SimRng`, `mad_turmoil::rand` and `fastrand`. Hold mad-turmoil's simulated-clock guard for the whole run.
3. **Topology.** One turmoil host runs `server::router(state)` on a `turmoil::net::TcpListener`, via a small adapter implementing axum's `Listener` trait. Two to four client hosts drive load with hyper over `turmoil::net::TcpStream`. `reqwest` cannot run inside turmoil, so sim clients do not use `cli::client`.
4. **Faults.** Random partitions and repairs, added latency, message hold and release, and `buggify!` sites in `core` (slow path, spurious retryable error, delayed write).
5. **Oracle.** A shadow model (`BTreeMap<ItemId, Item>`) applies every acknowledged operation. After the run, the server's `list` must equal the model, and every acknowledged create must be readable.
6. **Repro line.** On any failure, panic with exactly: `DST failure. Reproduce: just dst SEED=<n> TEST=<test name>`. Also record `dst_seed` as a field on the root span.
7. **Self-check.** Test `dst_is_deterministic` runs one seed twice and asserts the normalized JSON log hash matches. If it fails, a nondeterminism source leaked, and that is the first thing to fix.

### `buggify!` (in `core::platform`)

- Implement the FoundationDB pattern directly (about 50 lines): each call site is enabled or disabled once per run, chosen by the seeded RNG, and enabled sites fire with a configurable probability. Compiles to `false` without feature `sim`.
- Alternative if the hand-rolled version grows: the `buggify` module in `moonpool-sim`. Record the choice in `docs/testing/dst.md`.

### Seed volume by tier

| Where | Seeds | Time budget |
| --- | --- | --- |
| CI on every PR (`just ci`) | 200 | \~2 min |
| Nightly (`hardening.yml`) | 100,000 | \~1 h, sharded across jobs |
| On demand | `just dst SEEDS=<n> START=<k>` | — |

DST does not run in git hooks; pre-push stays under a few minutes.

### Future database note

There is no database now. `docs/testing/index.md` should say that when one is added, integration tests use `testcontainers` (random ports, automatic cleanup) and the DST model gains a storage fault layer behind the `ItemRepo` trait.

## 8. Hardening tiers and LLVM signals

Five tiers, each one `just` recipe that runs the previous tier's checks plus its own. Every check also writes a one-object JSON summary to `target/harness/<check>.json` with `{ "check", "ok", "summary", "details_path", "repro" }`, so agents read results without parsing tool output.

| Tier | Recipe | Runs on | Adds | Gate |
| --- | --- | --- | --- | --- |
| 0 | `just check` | pre-commit (which also runs affected tests and the panic audit); the agent Stop hook runs only the lint steps | fmt and clippy `-D warnings` in every workspace, nextest, machete, okf validate docs --strict --drift | Hard |
| 1 | `just ci` | every PR; pre-push runs the subset `just ci-fast` | `hk check --all`, cargo-deny, dylint, architecture tests, coverage gate, panic audit, asm snapshots, Gungraun, Miri, Kani quick, DST 200 seeds, docs freshness | Hard |
| 2 | `just perf` | manual / `perf.yml` dispatch | optimization remarks, llvm-mca, Criterion, `cargo llvm-lines`, `cargo build --timings` | Informational |
| 3 | `just harden` | nightly `hardening.yml` | fuzz campaigns, ASan, TSan, Kani full, DST 100k seeds, cargo-mutants | Hard, opens an issue on failure |
| 4 | `just release` | release tags | public API diff, optional PGO build | Hard for API diff |

### Tier 1 details

- **Coverage:** `cargo llvm-cov nextest --workspace --json --output-path target/harness/cov.json --fail-under-regions $COV_MIN_REGIONS`. `COV_MIN_REGIONS` lives in `mise.toml`. `just cov-ratchet` raises it to the current value minus 0.5 and never lowers it. `--show-missing-lines` output goes to `target/harness/cov-missing.txt` for the agent.
- **Panic audit:** `cargo asm --lib -p "$PROJECT_NAME-core" --callers-of panic` (release profile), compared against `docs/generated/panic-allowlist.txt`. New entries fail with: `New panic path in <fn>. Remove it (iterators, hoisted assert, checked ops) or add it to the allowlist with a reason in the PR.`
- **Asm snapshots:** functions listed in `docs/performance/hot-paths.md` as hot, marked `#[inline(never)]`, captured with `cargo asm --simplify` into `insta` snapshots. Diffs fail until reviewed with `cargo insta review`.
- **Gungraun:** instruction-count benchmarks in `crates/<core>/benches/`. Pin `gungraun-runner` to the exact library version in `mise.toml`. Fail on regressions over 2%. Linux only; on macOS the recipe prints `skipped: gungraun needs Valgrind (Linux)` with `ok: true`.
- **Miri:** `cargo +$NIGHTLY miri nextest run -p "$PROJECT_NAME-core"` with `MIRIFLAGS=-Zmiri-strict-provenance`. Tests that touch the network or files are `#[cfg_attr(miri, ignore)]`.
- **Kani quick:** harnesses in `core` named `quick_*` with `#[kani::unwind(8)]`, run with `cargo kani -p "$PROJECT_NAME-core" --harness 'quick_*'`. Start with: ID parsing never panics, validated newtypes uphold their invariant, and the pagination math never overflows.
- **DST:** 200 seeds in `harness/dst` (Section 7).
- **Docs freshness:** `okf validate docs --strict --drift` (schema conformance, broken concept links, description drift, concepts past `stale_after`), plus `harness/checks/tests/arch.rs` checking that every path mentioned in `AGENTS.md` exists, `AGENTS.md` is at most 120 lines, and `docs/generated/` matches a fresh `just docs`.

### Tier 2 details (never gates)

- **Remarks:** `cargo +$NIGHTLY remark build` (cargo-remark) or `RUSTFLAGS="-Cremark=loop-vectorize,inline -Zremark-dir=target/harness/remarks"`. A small script turns the YAML into `{file, line, pass, name, function}` lines filtered to this workspace.
- **llvm-mca:** `cargo asm --mca -M -mcpu=<target> <fn>` for the hot functions.
- **Criterion:** wall-clock confirmation only. A performance change is accepted when Gungraun improves, differential tests still pass, and Criterion agrees.
- **Compile time:** `cargo llvm-lines --release -p <core>` (top 30) and `cargo build --timings`.

### Tier 3 details

- **Fuzzing (`harness/fuzz/`):** targets `parse_item_id`, `decode_create_item` (JSON body), and `differential_core` (random operation sequences applied to `core` and to the oracle model, which must agree). Every `cargo fuzz` call in the `just` recipes passes `--fuzz-dir harness/fuzz`. Each target runs 10 minutes nightly with `-max_total_time=600`. Corpora are committed under `harness/fuzz/corpus/`. A crash is minimized with `cargo fuzz tmin`, saved as a regression test, and fixed in the same PR.
- **ASan:** prefer the stable Tier 2 target `x86_64-unknown-linux-gnuasan` if `rustup target add` provides it for the pinned toolchain. Otherwise `RUSTFLAGS=-Zsanitizer=address cargo +$NIGHTLY nextest run --target x86_64-unknown-linux-gnu`. Record which one worked in `docs/hardening/sanitizers.md`.
- **TSan:** `RUSTFLAGS=-Zsanitizer=thread cargo +$NIGHTLY nextest run -Zbuild-std --target x86_64-unknown-linux-gnu` on `core` and `server`.
- **Kani full:** all harnesses, higher unwind bounds, 60-minute budget.
- **DST:** 100,000 seeds, sharded across matrix jobs by `START`.
- **Mutation testing:** `cargo mutants --in-diff` on PRs is too slow for tier 1, so run full `cargo mutants -p <core>` nightly and write survivors to `target/harness/mutants.json`.

### Tier 4 details

- `cargo public-api diff` for library crates. Breaking changes need an explicit note in the PR.
- `cargo pgo` build documented in `docs/performance/index.md` as optional; not run by default.

## 9. Architecture enforcement and lints

Every rule an agent might break is enforced by a tool, and every tool's failure message says what to do instead. Prefer built-in clippy configuration, use the architecture tests in `harness/checks` for structure, and write dylint lints only for what clippy cannot express.

### Lint policy (`[workspace.lints]` + `clippy.toml`)

Start strict and loosen only with a written reason. Clippy lints only run under `cargo clippy`, so denying them never blocks an agent's plain `cargo build` while it iterates; they block commit and CI. The policy below follows Clippy's own guidance that `restriction` lints are opted into one by one, never enabled as a whole group ([Clippy README](https://github.com/y21/rust-clippy)), and borrows from public strict presets such as [leash](https://docs.rs/crate/leash/latest/source/README.md) and the agent-oriented [Rust Magic Linter](https://smithery.ai/skills/vicnaum/rust-magic-linter).

**How it is enforced**

- Member crates contain only `[lints] workspace = true`. Cargo does not let a member mix that with its own overrides, so stricter per-crate rules go in `lib.rs` as `#![deny(...)]`.
- Groups use `priority = -1` so the individual lint lines below them win.
- Every check runs `cargo clippy --workspace --all-targets --all-features -- -D warnings`, so rustc warnings fail too.
- The only escape hatch is `#[expect(lint, reason = "...")]` on the smallest item possible. `#[allow]` is a build error, and an `expect` that no longer fires is a build error (`unfulfilled_lint_expectations`).
- The architecture test counts `#[expect(` occurrences and compares them to `docs/generated/lint-exceptions.txt`. New exceptions fail until that file is updated in the same PR, which makes every new suppression visible in review.

**`Cargo.toml`**

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
rust_2018_idioms = { level = "deny", priority = -1 }
unreachable_pub = "deny"                # visibility is the API surface agents read
missing_debug_implementations = "deny"  # every type is printable in logs and test failures
missing_docs = "warn"                   # -D warnings makes it fail; binaries' private items are exempt
unused_qualifications = "deny"
unused_lifetimes = "deny"
redundant_lifetimes = "deny"
let_underscore_drop = "deny"
trivial_casts = "deny"
trivial_numeric_casts = "deny"
non_ascii_idents = "deny"
unit_bindings = "deny"
unfulfilled_lint_expectations = "deny"
unknown_lints = "deny"                  # a renamed lint after a toolchain bump fails loudly
renamed_and_removed_lints = "deny"
unexpected_cfgs = { level = "deny", check-cfg = ["cfg(kani)", "cfg(fuzzing)", "cfg(coverage_nightly)"] }

[workspace.lints.rustdoc]
broken_intra_doc_links = "deny"
private_intra_doc_links = "deny"

[workspace.lints.clippy]
# Groups
all = { level = "deny", priority = -1 }        # correctness, suspicious, style, complexity, perf
pedantic = { level = "deny", priority = -1 }
cargo = { level = "deny", priority = -1 }

# Pedantic/cargo relaxations (noise without much value here)
must_use_candidate = "allow"
missing_errors_doc = "allow"
module_name_repetitions = "allow"
similar_names = "allow"
multiple_crate_versions = "allow"   # cargo-deny reports duplicates instead

# Restriction: panics and silent failure
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"
exit = "deny"
unwrap_in_result = "deny"
panic_in_result_fn = "deny"
get_unwrap = "deny"
indexing_slicing = "deny"
string_slice = "deny"
unused_result_ok = "deny"
map_err_ignore = "deny"
let_underscore_must_use = "deny"
let_underscore_untyped = "deny"
mem_forget = "deny"

# Restriction: types and numbers
as_conversions = "deny"
float_cmp_const = "deny"
lossy_float_literal = "deny"
fn_to_numeric_cast_any = "deny"

# Restriction: output, lint hygiene, unsafe
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
allow_attributes = "deny"
allow_attributes_without_reason = "deny"
undocumented_unsafe_blocks = "deny"
multiple_unsafe_ops_per_block = "deny"

# Restriction: determinism and change safety
iter_over_hash_type = "deny"
wildcard_enum_match_arm = "deny"
rest_pat_in_fully_bound_structs = "deny"
unneeded_field_pattern = "deny"

# Restriction: readable, findable code
missing_assert_message = "deny"
tests_outside_test_module = "deny"
mod_module_files = "deny"
multiple_inherent_impl = "deny"
shadow_unrelated = "deny"
same_name_method = "deny"
min_ident_chars = "deny"
clone_on_ref_ptr = "deny"
rc_buffer = "deny"
rc_mutex = "deny"
str_to_string = "deny"
try_err = "deny"
if_then_some_else_none = "deny"
redundant_type_annotations = "deny"
renamed_function_params = "deny"
infinite_loop = "deny"
verbose_file_reads = "deny"
empty_drop = "deny"
empty_structs_with_brackets = "deny"
empty_enum_variants_with_brackets = "deny"

# Nursery picks (individually, never the whole group)
future_not_send = "deny"
significant_drop_in_scrutinee = "deny"
fallible_impl_from = "deny"
cognitive_complexity = "deny"
redundant_clone = "deny"
needless_collect = "deny"
or_fun_call = "deny"
use_self = "deny"
derive_partial_eq_without_eq = "deny"
trait_duplication_in_bounds = "deny"
type_repetition_in_bounds = "deny"
```

**Extra rule in `core`** (`lib.rs`): `#![deny(clippy::arithmetic_side_effects)]`. Overflow and division bugs in business logic must use `checked_*`, `saturating_*` or `wrapping_*` explicitly. This is too noisy for glue code, so it stays local to `core`.

**`clippy.toml` thresholds and test allowances** (the bans table below goes in the same file)

```toml
msrv = "<pinned stable>"
avoid-breaking-exported-api = false

# Small units are easier for agents to read, test and change
too-many-lines-threshold = 80
cognitive-complexity-threshold = 15
excessive-nesting-threshold = 4
too-many-arguments-threshold = 5
type-complexity-threshold = 200
max-fn-params-bools = 1
max-struct-bools = 3
min-ident-chars-threshold = 2
# Short names that stay readable in closures and loops
allowed-idents-below-min-chars = ["i", "j", "n", "x", "y", "e", "k", "v", "f", ".."]

# Tests may panic, index and print
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true
allow-print-in-tests = true
allow-dbg-in-tests = true
```

These option names come from Clippy's [configuration reference](https://docs.adacore.com/live/wave/rust/html/rust_ug/_static/clippy/lint_configuration.html); an example of thresholds in use is [quinjet's clippy.toml](https://docs.rs/crate/quinjet/latest/source/clippy.toml).

**Two lints that also apply inside tests, on purpose:**

- `missing_assert_message` has no test allowance, so every `assert!` and `assert_eq!` in tests needs a message too. This is the lint agents will hit most; it is kept because a failing test should explain itself in nextest output without opening the source.
- `wildcard_enum_match_arm` may also fire on matches over `#[non_exhaustive]` std enums such as `io::ErrorKind`, where a wildcard arm is mandatory. Verify with the pinned toolchain; if it fires, add `#[expect(clippy::wildcard_enum_match_arm, reason = "non_exhaustive std enum")]` at those few sites rather than relaxing the lint.

**The lints that help agents most**

| Lint | What agents tend to do | What the lint forces |
| --- | --- | --- |
| `allow_attributes`, `allow_attributes_without_reason` | Silence an error with `#[allow]` | A visible, reasoned `#[expect]` that breaks when obsolete |
| `wildcard_enum_match_arm` | Add `_ =>` arms | Adding an enum variant makes the compiler list every match to update |
| `unwrap_used`, `expect_used`, `indexing_slicing`, `panic_in_result_fn` | Reach for the quickest way to a value | Errors flow through `Result` to a logged, typed failure |
| `iter_over_hash_type` | Iterate a `HashMap` and depend on its order | Deterministic order, which DST relies on |
| `too_many_lines`, `cognitive_complexity`, `excessive_nesting` | Grow one function until it does everything | Small functions that fit in context and test cleanly |
| `missing_assert_message` | Write bare `assert!(x)` | Failures that explain themselves in test output and logs |
| `future_not_send`, `significant_drop_in_scrutinee` | Hold a lock or non-`Send` value across `.await` | An error at the definition, not a confusing axum handler error later |
| `map_err_ignore`, `unused_result_ok`, `let_underscore_must_use` | Drop errors to make code compile | The original error is kept or handled |
| `as_conversions`, pedantic `cast_*` | Use `as` casts that truncate silently | `From`/`TryFrom` with explicit failure |
| `unreachable_pub`, `missing_debug_implementations` | Make everything `pub`, skip `Debug` | A clear API surface and printable types in logs |
| `unknown_lints`, `renamed_and_removed_lints` | (not agents: toolchain upgrades) | Lint renames fail loudly instead of silently disabling a rule |

**When the toolchain is upgraded:** run `cargo clippy` before anything else. Fix or `#[expect]` (with reason) any new lint hits in the same PR. Never add a blanket allow to get the upgrade through.

**Verify while building:** every lint and option name above exists in the pinned toolchain's Clippy (an unknown name fails because of `unknown_lints`), whether `module_name_repetitions` is still pedantic or has moved to restriction, and which nursery picks have since moved to a stable group.

### `clippy.toml` bans, each with a remediation reason

| Banned | Reason text shown to the agent |
| --- | --- |
| `std::time::Instant::now`, `std::time::SystemTime::now` | Use `core::platform::Clock::now()`. Direct time breaks DST determinism. See docs/testing/determinism.md |
| `tokio::time::sleep`, `std::thread::sleep` | Use `core::platform::Clock::sleep()`, so simulated time controls it |
| `rand::thread_rng`, `rand::rng`, `rand::random` | Use the `core::platform::Rng` from `AppState`, so seeds reproduce runs |
| `std::env::var` | Read configuration through `runtime::config` |
| Types `std::collections::HashMap`, `HashSet` | Use `core::platform::HashMap`/`HashSet`; iteration order must be deterministic under simulation |
| Macros `std::println`, `std::eprintln`, `std::dbg`, `log::*` | Use `tracing` with structured fields. User-facing CLI output goes through `cli::output` only |

`core::platform`, `runtime::config` and `cli::output` each carry one `#[expect(..., reason = "...")]` at the single place they are allowed to break a rule.

### `deny.toml`

- Licenses: allowlist permissive licenses; fail on anything else.
- Advisories: deny vulnerabilities and unmaintained crates; ignores need an expiry comment.
- Bans: `openssl`, `openssl-sys` and `native-tls` (use rustls); warn on duplicate versions.
- Sources: crates.io only, no git dependencies without an explicit entry.

### Architecture tests (`harness/checks/tests/arch.rs`)

- **Crate layering:** parse `cargo metadata --format-version 1`, compare every internal edge to the table in Section 4, and fail with: `Forbidden dependency: <from> -> <to>. Allowed for <from>: [...]. See docs/architecture/layers.md. Move the shared code down a layer instead.`
- **Module layering in `core`:** scan `crates/<core>/src/**` for `use crate::<module>` and fail on edges outside the order `types` → `platform` → `repo` → `domain`, with the same message style.
- **File size:** any `.rs` file over 500 lines fails with: `<file> has <n> lines (limit 500). Split it by responsibility; see docs/conventions/modules.md.`
- **Thin binaries:** each `src/main.rs` under 80 lines; logic belongs in the library.
- **Version drift:** dependency versions in `harness/dst` and `harness/fuzz` match `[workspace.dependencies]`.
- **Lint exceptions:** the count of `#[expect(` matches `docs/generated/lint-exceptions.txt`.
- **Docs freshness and AGENTS.md size:** as in Section 8.

### dylint lints (`harness/lints/`)

A separate workspace with its own pinned nightly, built with `dylint_linting` and tested with `dylint_testing` UI tests. Registered in the main `Cargo.toml` under `[workspace.metadata.dylint]` with libraries = \[{ path = "harness/lints" }\], and run by `cargo dylint --all` in tier 1. dylint is the most expensive piece of the harness: template-ci records its cold and warm build times in `docs/template/notes.md`, and if a warm run takes more than about two minutes, move it to tier 3 (nightly) instead of slowing every PR. Write exactly these two lints to start:

1. **`tracing_message_interpolation`:** flags `tracing` macros whose message string contains format arguments. Message: `Put variable data in fields, not the message: info!(item_id = %id, "item_created"). See docs/observability/fields.md.`
2. **`telemetry_field_style`:** flags field keys in `tracing` macros and `#[instrument(fields(...))]` that are not `snake_case` or lowercase dotted OTel names. Message names the field, the expected style and the doc anchor.

### Protected files

Agents must not weaken checks to make them pass. These paths are listed in `.github/CODEOWNERS` and in `AGENTS.md` as "ask first": `clippy.toml`, `deny.toml`, `[workspace.lints]`, `.config/nextest.toml`, `COV_MIN_REGIONS` in `mise.toml`, Gungraun baselines, `docs/generated/lint-exceptions.txt`, `docs/generated/panic-allowlist.txt`, `harness/lints/`, and `harness/checks/tests/arch.rs`. An hk step prints a warning listing any of these files in a staged change, so the agent sees it before committing.

## 10. Agent configuration

`AGENTS.md` is the only place rules live. Codex reads it natively; `CLAUDE.md` imports it with `@AGENTS.md` and adds only Claude-specific notes. Tool configs for both agents point at the same `just` recipes and the same just mcp recipe.

### `AGENTS.md` skeleton (target 90-110 lines)

```markdown
# {{project-name}}: agent map

{{description}}. Rust workspace: core + runtime libraries, axum server, clap CLI.

## Commands (run these, not raw cargo)
| Need | Run |
| just check  | tier 0: fmt, clippy, tests (also runs on commit) |
| just ci     | tier 1: everything CI runs on a PR |
| just up / just down | per-worktree stack + server; URLs in .harness/app.json |
| just e2e    | end-to-end tests + latency budgets (needs just up) |
| just logs-errors / just q-logs / just q-metrics / just q-traces / just trace | see the app |
| just dst SEEDS=n | deterministic simulation |
| just perf   | performance signals (never gates) |

## Layout
crates/: core (types, domain, platform seams), runtime (config, telemetry), server, cli.
harness/: checks (architecture + e2e tests), dst (simulation), fuzz, lints (dylint), stack (local Victoria compose).
Details and allowed dependencies: docs/architecture/layers.md (enforced by tests).

## Rules
- Logging: tracing only, constant messages, data in fields (docs/observability/fields.md).
- Time, randomness, HashMap: only via core::platform (DST depends on it).
- No unwrap/expect outside tests. Exceptions: #[expect(lint, reason = "...")], never #[allow].
- Every bug fix adds a failing test first (unit, snapshot, DST seed, or fuzz regression).
- Never bypass git hooks (--no-verify, HK_SKIP_*). If a hook is wrong, say so and stop.
- Ask before editing protected files: <list>. Never lower a threshold or baseline.
- Perf claims need Gungraun numbers, not wall-clock alone (docs/performance/benchmarks.md).

## Seeing what the app does
1. just up, reproduce, then: just logs-errors (fast, local file).
2. Need history or cross-request data: Victoria MCP servers or just q-*.
3. Budgets: just budgets. Canned queries: docs/observability/queries.md.
Query and aggregate; never dump raw logs.

## When something fails
Read target/harness/<check>.json first: it has ok, summary, details_path and a repro command.
DST failures print a `just dst SEED=...` line; rerun exactly that.

## Plans and docs
Multi-step work: create a Plan concept in docs/plans/ (okf create); set status: completed when done.
Knowledge: docs/ is an OKF bundle. Search before you write or ask: okf-memory MCP or okf search "..." docs.
```

### `CLAUDE.md`

```markdown
@AGENTS.md

## Claude Code specifics
- MCP servers: victoriametrics, victorialogs, victoriatraces (this worktree's stack; start it with just up).
- The Stop hook runs hk on your unstaged changes. If it fails, fix what it reports before finishing.
```

### Claude Code files

- **`.claude/settings.json`**
  - `permissions.allow`: `Bash(just:*)`, `Bash(cargo:*)`, `Bash(git status:*)`, `Bash(git diff:*)`, `Bash(git log:*)`, `Bash(jq:*)`.
  - `permissions.ask`: `Edit` on every protected file from Section 9, and `Bash(git push:*)`.
  - `permissions.deny`: hook bypasses (see Git hooks below).
  - `hooks.Stop`: one command running `hk fix --unstaged && hk check --unstaged`. Generate the exact snippet with hk's agent snippet command (expected `hk agent hooks claude-code`; check `hk agent --help`) instead of handwriting it. Also add a WorktreeCreate hook running workz sync --isolated --quiet "$WORKTREE\_PATH" (workz hook claude prints it), so every new worktree has its ports before the session starts.
- **`.mcp.json`**: three stdio servers, `victoriametrics`, `victorialogs`, `victoriatraces`, each `"command": "just"` with args `["mcp", "metrics"]`, `["mcp", "logs"]` or `["mcp", "traces"]`. `just` finds the `justfile` from the project root, so no relative script paths are involved.

### Codex files

- **`.codex/config.toml`** (project-scoped config): the same three servers as `[mcp_servers.<name>]` tables with `command = "just"` and `args = ["mcp", "logs"]` and so on.
- If the installed Codex version ignores project-scoped config, add `just agent-setup-codex`, which runs `codex mcp add <name> -- just mcp <kind>` for each server. The `codex mcp add` form is documented in the [mcp-victorialogs README](https://pkg.go.dev/github.com/VictoriaMetrics/mcp-victorialogs).
- Codex stop-style hooks: use hk's Codex snippet (expected `hk agent hooks codex`). If Codex has no equivalent hook in the installed version, say so in `docs/template/notes.md`; the pre-commit hook still covers it. Worktrees created without a setup hook get their ports from the just up error message, which tells the agent to run workz sync --isolated.

### Project memory: `docs/` as an OKF bundle

`docs/` is not a folder of loose Markdown. It is an [OKF v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) knowledge bundle managed with [okf-agent-memory](https://github.com/okf-memory/okf-agent-memory): every file is a concept with YAML frontmatter (type, description, status, `stale_after`, sources, trust tier), each directory has an `index.md` for progressive disclosure, and `docs/log.md` records changes by date. Agents load only the concepts they need instead of whole guides, and the bundle is validated like code.

**Setup**

- Pin the `okf` binary in `mise.toml` (Go; use mise's `go:` or GitHub-release backend). Create the bundle with `okf init docs`, not `okf bootstrap`: bootstrap also writes its own `AGENTS.md` and `Makefile`, which this template already provides.
- Concept layout: `architecture/`, `observability/`, `testing/`, `hardening/`, `performance/`, `conventions/` (type Guide or Concept), `decisions/` (type Decision; ADRs such as `adding-a-ui`), `plans/` (type Plan with `status: active | completed`, replacing the old `exec-plans/active` and `completed` folders), `template/notes` (the template's deviation log). Every concept this document names (`docs/observability/fields.md`, `docs/testing/determinism.md`, and so on) is created in milestone 8 with at least a description, so links resolve from day one.
- `docs/generated/` holds machine output, not concepts. Verify how `okf validate` treats non-concept files; if it cannot be told to skip that directory, move it to `harness/generated/` and update the paths.

**Agent rules (in `AGENTS.md`)**

- Search before you write or ask: `okf search "..." docs` or the `okf-memory` MCP tools. Do not create a concept that an existing one covers; update it.
- Record decisions as `decisions/` concepts and multi-step work as `plans/` concepts via `okf create`, which also updates `index.md` and `log.md`.
- New facts an agent discovers are `trust: generated` until a human marks them `verified`. Agents never flip that field.

**Tooling**

- **MCP:** add `okf-memory` to `.mcp.json` and `.codex/config.toml` as `command: "okf"`, `args: ["mcp", "docs"]`. It needs no ports, so it does not go through `just mcp`.
- **`just knowledge`:** the single recipe every caller uses. It runs `okf validate docs --strict --drift`, writes `target/harness/knowledge.json` (`ok`, counts of concepts, broken links, drifted descriptions, stale concepts, and a repro line), and exits non-zero on any finding. It runs in tier 0 (`just check`), as an hk pre-commit step on the glob `docs/**`, and in CI.
- **CI uses the `okf` CLI, never ad-hoc parsing of the bundle:**
  - `ci.yml` gains a `knowledge` job: `just knowledge`; if the PR touches `docs/`, require a new dated entry in `docs/log.md` (fail with `docs/ changed without a log.md entry; run okf update or add one`); and post one PR comment listing the changed concepts with their titles and descriptions from `okf show <id> docs --json`, so reviewers see what the knowledge change says without opening the files.
  - `template-ci.yml` runs `just knowledge` in the generated project, so the template never ships an invalid bundle.
  - `gardening.yml` (weekly) runs `okf validate docs --drift` and lists concepts past `stale_after`, concepts still `trust: generated` after 30 days, and `plans/` concepts active for more than 30 days (from frontmatter via `okf show --json`), then regenerates `docs/generated/` and diffs, lists `#[expect]` entries older than 90 days, and runs `cargo outdated`. It opens or updates one issue with all of it.
- **Smoke test:** `okf search "port allocation" docs` must return the port-allocation concept first; `template-ci` asserts it, so a broken index or a renamed concept is caught.

**Risk.** okf-agent-memory is new (a single public commit at the time of writing). The format is plain Markdown and YAML, so if the tool stalls the knowledge stays readable and the validation can be reimplemented in `harness/checks`. Note the pinned version and this fallback in `docs/template/notes.md`.

### Git hooks with hk (`hk.pkl`)

Hooks are installed by `just bootstrap` and are never optional. Commits get fast formatting, lint and affected tests, plus an LLVM panic audit when hot-path crates change. Pushes get the heavier LLVM signals.

**Installation**

- `just bootstrap` runs `hk install` (with hk's mise integration, expected `hk install --mise`, so hooks find mise-pinned tools without an activated shell).
- Git worktrees share the hooks directory, so one install covers every worktree an agent creates.
- `just check` and `just up` fail fast with `Git hooks not installed. Run: just bootstrap` if `.git/hooks/pre-commit` does not invoke hk.
- `just ci` runs `hk check --all`, so commits made with hooks bypassed are still caught in CI.

**Steps by hook** (use hk builtins where they exist; verify names in the hk builtins list)

| Hook | Step | Runs when staged files match | Mode |
| --- | --- | --- | --- |
| pre-commit, check, fix | `cargo fmt` (every workspace) | `**/*.rs` | fix + stage |
| pre-commit, check, fix | `taplo fmt` (TOML) | `**/*.toml` | fix + stage |
| pre-commit, check, fix | `just clippy` (`--all-targets -D warnings`, every workspace) | `**/*.rs`, `**/Cargo.toml`, `clippy.toml` | check |
| pre-commit, check, fix | `typos` | any text file | fix + stage |
| pre-commit, check, fix | **secret scan** (`gitleaks` on staged changes; hk builtin expected `gitleaks_staged`) | any file | check |
| pre-commit, check, fix | `cargo machete` | `**/Cargo.toml` | check |
| pre-commit, check, fix | `cargo deny --offline check bans licenses sources` | `**/Cargo.toml`, `Cargo.lock`, `deny.toml` | check |
| pre-commit, check, fix | `actionlint` | `.github/workflows/*.yml` | check |
| pre-commit, check, fix | `just protected-files {{files}}` | protected paths (Section 9) | warn only |
| pre-commit | `just affected-tests {{files}}`: maps staged files to packages, then `cargo nextest run -p ...` (all packages if `core` or the root `Cargo.toml` changed) | `**/*.rs`, `**/Cargo.toml` | check |
| pre-commit | **panic audit** (`just panic-audit`): `cargo asm --callers-of panic` against the allowlist | `crates/*-core/src/**` | check |
| pre-push | `just ci-fast`: coverage gate, **asm snapshots**, **Gungraun** (Linux), full `cargo deny` with advisories, architecture tests, dylint | always | check |

**Why the LLVM checks sit where they do.** The panic audit needs a release build of one or two crates, which is incremental after the first run, and it catches the most common agent regression (a new bounds check or `unwrap` path in hot code) at the moment it is written. Coverage, asm snapshots and Gungraun need instrumented or benchmark builds of the whole workspace, so they run on pre-push, where a few minutes is acceptable.

**Speed and correctness settings**

- pre-commit uses `fix = true` with git stashing of unstaged changes, so formatters never sweep unrelated work-in-progress into a commit.
- Target: under 30 seconds warm for a one-file change. If pre-commit exceeds that, move steps to pre-push rather than loosening them.
- The `check` and `fix` hooks contain only the formatting and lint steps (no tests, no panic audit). The agent Stop hook runs `hk fix --unstaged && hk check --unstaged`, so it stays fast.

**Agents cannot skip hooks**

- `.claude/settings.json` `permissions.deny`: `Bash(git commit --no-verify:*)`, `Bash(git commit -n:*)`, `Bash(git push --no-verify:*)`, `Bash(HK_SKIP_STEPS=*)`, `Bash(HK_SKIP_HOOK=*)`. These are prefix rules, so a bypass buried mid-command can slip past them; `hk check --all` in CI is the backstop.
- `AGENTS.md` rule: never bypass hooks; if a hook is wrong, say so and stop. The Codex equivalent goes in its project config if supported; otherwise the `AGENTS.md` rule plus `hk check --all` in CI cover it.

**Shape of `hk.pkl`** (a sketch; match the exact v2 syntax and builtin names to the hk docs):

```text
amends "package://github.com/jdx/hk/releases/download/v2.X.Y/hk@2.X.Y#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.X.Y/hk@2.X.Y#/Builtins.pkl"

local lint = new Mapping<String, Step> {
  ["cargo-fmt"] { glob = List("**/*.rs"); check = "just fmt-check"; fix = "just fmt" }
  ["clippy"] { glob = List("**/*.rs", "**/Cargo.toml", "clippy.toml"); check = "just clippy" }
  ["taplo"] = Builtins.taplo
  ["typos"] = Builtins.typos
  ["secrets"] = Builtins.gitleaks_staged
  ["machete"] { glob = List("**/Cargo.toml"); check = "cargo machete" }
  ["deny-offline"] { glob = List("**/Cargo.toml", "Cargo.lock", "deny.toml"); check = "cargo deny --offline check bans licenses sources" }
  ["actionlint"] = Builtins.actionlint
  ["protected-files"] { check = "just protected-files {{files}}" }
}

hooks {
  ["pre-commit"] { fix = true; stash = "git"; steps { ...lint
    ["affected-tests"] { glob = List("**/*.rs", "**/Cargo.toml"); check = "just affected-tests {{files}}" }
    ["panic-audit"] { glob = List("crates/*-core/src/**"); check = "just panic-audit" } } }
  ["pre-push"] { steps { ["ci-fast"] { check = "just ci-fast" } } }
  ["check"] { steps = lint }
  ["fix"] { fix = true; steps = lint }
}
```

`hk.pkl` uses hk's own `{{files}}` templating, which is why it is excluded from Liquid (Section 3). Add `taplo`, `typos`, `gitleaks` and `actionlint` to `mise.toml`. There is no shell-script lint step because there are no shell scripts; logic lives in `just` recipes.

### Optional extras (add if they work cleanly; otherwise list in docs/template/notes.md)

- A rust-analyzer MCP server for go-to-definition and references.
- hk's own MCP server, if the pinned hk version provides one.
- A short project skill for each agent (`.claude/skills/harness/SKILL.md` and Codex's equivalent location) that walks through "reproduce, observe, fix, verify" with the recipes above. Verify each agent's current skills directory before adding.

## 11. CI and local CI

Workflows contain no logic: each job installs tools with mise, restores the Rust cache, runs one `just` recipe, and uploads `target/harness/`. Anything that works locally works in CI the same way.

### Shared job shape

1. `actions/checkout` (pinned by commit SHA, as are all actions).
2. `jdx/mise-action` to install everything in `mise.toml`.
3. `Swatinem/rust-cache`.
4. `just <recipe>`.
5. Always upload `target/harness/` and `target/nextest/ci/junit.xml` as artifacts, and publish the JUnit report as a job summary.

### Workflows

| File | Trigger | Jobs |
| --- | --- | --- |
| `ci.yml` | pull request, push to main | `check` on ubuntu + macOS (`just check`); `ci` on ubuntu (`just ci`); `e2e` on ubuntu (`just up && just e2e`, then `just down` always); `knowledge` on ubuntu (`just knowledge`, `docs/log.md` entry check, changed-concept PR comment) |
| `perf.yml` | manual dispatch | `just perf`; artifacts only, never fails the run on numbers |
| `hardening.yml` | nightly cron + manual | matrix: each fuzz target, DST shards (10 x 10,000 seeds by `START`), ASan, TSan, Kani full, mutants. On failure, opens or updates a GitHub issue with the harness JSON and the repro line |
| `gardening.yml` | weekly cron + manual | `okf validate docs --drift`, stale and unverified concepts, old lint exceptions, `docs/generated/` diff, `cargo outdated`; opens or updates one issue (Section 10) |
| `template-ci.yml` | template repo only (ignored on generate) | generation test below, including `just knowledge` and the `okf search` smoke test |

### `template-ci.yml` (the template's own test)

1. Generate non-interactively: `cargo generate --path . --name demo-app --define description="Demo" --define gh_owner=acme --define license="MIT OR Apache-2.0" --silent`. Repeat for each license choice in a matrix.
2. Fail if any generated, non-excluded file contains `{{`, `{%` or a mangled `${{`.
3. In the generated project, make the first commit (`git add -A && git commit -m init`). cargo-generate initializes the repository but does not commit, and `git worktree add` needs a commit.
4. `mise trust && mise install`, `just bootstrap`, `just ci`, `just up`, `just e2e`, `just down`.
5. Isolation test: `git worktree add ../demo-app-2`, run workz sync --isolated, `mise trust` and `just up` in both, assert the two `.harness/stack.json` files have different ports, then `just down` in both.
6. Lint generated workflows with `actionlint`.
7. Record timings in the job summary: cold and warm `just check`, a one-file pre-commit, and dylint's cold and warm build. These feed the thresholds in Sections 9 and 10.

While the template repository is local (Section 13), GitHub Actions cannot run this workflow. A `just template-ci` recipe runs the same steps locally, either directly or through `act`, and is part of the acceptance checklist. The workflow file is committed so it works as soon as the repo is pushed.

### Local CI

- `just ci-validate`: `wrkflw validate` and `actionlint` on `.github/workflows/`. Fast; run it whenever a workflow changes.
- `just ci-local job=<name>`: `act -j <name> -P ubuntu-latest=catthehacker/ubuntu:act-latest`. Use it to test triggers, matrices and the `e2e` job's Docker usage before pushing.
- Known `act` gaps to note in `docs/template/notes.md`: caching behaves differently, artifacts go to local storage, and `github.token` must be passed with `-s GITHUB_TOKEN=...` for steps that call the GitHub API. Fix logic in `just` recipes, not in workflow YAML.

## 12. Acceptance checklist

The template is done when every box below is ticked on a clean Linux machine and the tier 0 boxes also pass on macOS. Each item names the command that proves it.

**Generation**

- [ ] `cargo generate --path . --name demo-app --define ... --silent` succeeds with no prompts.
- [ ] No leftover `{{`, `{%` or mangled `${{` in non-excluded files; generated workflows pass `actionlint`.
- [ ] `mise install && just bootstrap` succeeds from a fresh clone.

**Build and tiers**

- [ ] `just check` passes in under 3 minutes on a warm cache.
- [ ] `just ci` passes and writes a JSON summary for every check into `target/harness/`.
- [ ] `just perf` runs and produces remarks, mca and Criterion output without failing.
- [ ] `just harden` runs with short budgets (`FUZZ_SECS=30 DST_SEEDS=1000`) and passes.

**Harness and telemetry**

- [ ] `just up` reaches ready in under 60 seconds and prints the app and UI URLs as JSON.
- [ ] Two worktrees run `just up` at the same time with different ports and no errors.
- [ ] After `just e2e`: `just q-logs` returns log lines carrying `trace_id`; `just trace <id>` shows the `http_request` span tree; `just q-metrics` returns `http_server_request_duration_seconds` buckets.
- [ ] `just budgets` reports all three budgets with pass/fail JSON.
- [ ] `just down` leaves no containers, volumes or server process behind.

**Agents**

- [ ] Claude Code lists all three Victoria MCP servers as connected (`/mcp`) and can answer "how many errors in the last 15 minutes?" from them.
- [ ] Codex lists the same servers and can run the same query.
- [ ] MCP servers started before `just up` begin answering once the stack is up, without restarting the agent.
- [ ] The Claude Code Stop hook runs hk and blocks on a deliberately misformatted file.
- [ ] `AGENTS.md` is at most 120 lines, and every path it mentions exists (architecture tests).

**Project memory**

- [ ] `okf validate docs --strict --drift` passes, and a concept with a broken link or an expired `stale_after` fails it with a clear message.
- [ ] The `okf-memory` MCP server answers a search such as "port allocation" with the right concept in both agents.
- [ ] `okf create` for a new plan updates `docs/plans/index.md` and `docs/log.md`.

**Git hooks**

- [ ] After `just bootstrap` in a fresh clone, `.git/hooks/pre-commit` and `pre-push` invoke hk.
- [ ] A second worktree runs the same hooks without reinstalling.
- [ ] A one-file change commits in under 30 seconds on a warm cache.
- [ ] A misformatted file is fixed and restaged; unstaged edits in the same file are left untouched.
- [ ] A new indexing panic in `core` fails the pre-commit panic audit with the remediation message.
- [ ] A staged fake API token is blocked by the secret scan.
- [ ] A coverage drop below the threshold fails pre-push.
- [ ] Claude Code is denied `git commit --no-verify`.
- [ ] `just ci` fails when a file that violates a hook step is committed with hooks bypassed.

**Enforcement (prove each rule bites, then revert)**

- [ ] `println!` in `core` fails clippy with the remediation text.
- [ ] `info!("created {id}")` fails the dylint lint with the remediation text.
- [ ] `Instant::now()` in `core::domain` fails clippy, pointing to `core::platform::Clock`.
- [ ] Adding `core -> server` as a dependency fails the architecture test with the allowed list.
- [ ] `use crate::domain` inside `core::types` fails the module-layering test.
- [ ] A bare `#[allow(...)]` fails; `#[expect(..., reason = "...")]` passes and is counted in `lint-exceptions.txt`.
- [ ] Editing `clippy.toml` triggers the hk protected-files warning.
- [ ] The CLI returns the documented exit codes through `ExitCode`, with no `process::exit` anywhere.

**Determinism and hardening**

- [ ] `dst_is_deterministic` passes.
- [ ] A bug injected into `core` (drop every 50th write) is caught by `just dst` and prints a `just dst SEED=...` repro line that fails again when rerun.
- [ ] `cargo test --workspace` in the main workspace never compiles `core` with the `sim` feature (check with `cargo tree -e features`).
- [ ] Each fuzz target runs 60 seconds without crashes; a planted panic in `parse_item_id` is found.
- [ ] Kani quick harnesses and Miri pass on `core`.
- [ ] The coverage gate fails when a test is deleted below the threshold.
- [ ] A new `panic!` path in a hot function fails the panic audit.

**Template hygiene**

- [ ] Every milestone in the build order (Section 1) ended with a green `just check`.
- [ ] `template-ci.yml` is green on the template repo.
- [ ] Generated projects record the template tag in `docs/template/notes.md`, and `TEMPLATE_README.md` documents the upgrade procedure.
- [ ] `docs/template/notes.md` lists every deviation from this document and every value verified (with versions).

## 13. Defaults, verification and open questions

The human made the locked decisions in Section 1. The defaults below were chosen on their behalf; the building agent keeps them unless a tool forces a change.

### Defaults chosen

| Area | Default | Why |
| --- | --- | --- |
| Platforms | Linux and macOS for development; Linux for CI tiers 1-3. Windows is not supported | Valgrind, sanitizers, Kani and the Victoria stack all assume Unix |
| Crate layout | 4 shipping crates (`core`, `runtime`, `server`, `cli`) in `crates/`; everything else under `harness/`: `checks` (workspace member), `dst`, `fuzz`, `lints` (separate workspaces) and `stack` (local compose) | Real layering with little ceremony; `core` splits along its module lines when it grows |
| Edition / MSRV | 2024 / pinned stable | Newest stable language; one toolchain everywhere |
| HTTP stack | axum 0.8, tower-http, hyper | Most used; best `tracing` integration |
| CLI | clap 4 derive, `ExitCode`, `trycmd` tests | Standard; transcripts are agent-readable |
| HTTP client | reqwest with rustls, in `cli::client` | No OpenSSL; matches the `deny.toml` ban |
| Errors | `thiserror` in libraries, `anyhow` in binaries | Typed where it matters, simple at the edges |
| Config | figment (TOML + env) | Layered config with clear provenance |
| Telemetry pipeline | Direct OTLP/HTTP per signal, no collector or Vector; CLI exports only with `--otel` | Fewer containers; Victoria accepts OTLP natively |
| Human UI | Built-in Victoria `/vmui`, no Grafana | One less container per worktree |
| Ports | workz port ranges (10 per worktree from 20000), tracked in \~/.config/workz/ports.json | MCP configs are static (Section 6) |
| Task logic | `just` recipes only, no `scripts/` | One place for every command; hooks, CI and agents share it |
| Example domain | In-memory items CRUD | Exercises every harness feature without a database |
| CI | GitHub Actions; macOS for tier 0 only | macOS minutes cost more; Gungraun and sanitizers need Linux |
| Thresholds | Coverage starts at measured value minus 0.5 and ratchets; Gungraun 2%; startup 800 ms; p95 50 ms; pre-commit 30 s | Start strict on a tiny codebase; loosen only with approval |

### Verify while building (record results in `docs/template/notes.md`)

- [ ] Latest compatible pair of `opentelemetry*` and `tracing-opentelemetry`, and that OTLP/HTTP honors the per-signal endpoint env vars.
- [ ] Victoria image versions, OTLP paths, `/health` endpoints, the VictoriaMetrics Prometheus-naming flag, and the exact metric names produced from `http.server.request.duration` (the canned PromQL depends on them).
- [ ] VictoriaTraces query field names for spans (name, duration, service).
- [ ] `mcp-victoriatraces` env var name and the install method for all three MCP binaries via mise.
- [ ] `just` version supporting `shell()` and `set quiet`, and that `just mcp <kind>` gives a clean stdout for stdio MCP.
- [ ] hk v2 builtin names (`taplo`, `typos`, `gitleaks_staged`, `actionlint`), `--unstaged` behavior, hk's mise install flag, and the `hk agent` snippet commands for Claude Code and Codex.
- [ ] Claude Code settings schema (permissions, `hooks.Stop`) and `@` import in `CLAUDE.md`.
- [ ] Codex project-scoped `.codex/config.toml` support; otherwise use `just agent-setup-codex`.
- [ ] cargo-generate `exclude` and `ignore` semantics, placeholder rendering inside defaults, and template metadata for the version tag.
- [ ] axum `Listener` trait shape for the turmoil adapter; mad-turmoil compatibility with the pinned turmoil.
- [ ] Lint names in the pinned Clippy; whether `wildcard_enum_match_arm` fires on `#[non_exhaustive]` std enums.
- [ ] okf: install via mise, how okf validate handles docs/generated/, and the okf mcp tool names; workz behavior on the pinned 0.x version (.env.local keys, the target override, the Claude Code hook form); dylint's required nightly; Kani setup on CI runners; `gungraun-runner` version match; whether rustup ships `x86_64-unknown-linux-gnuasan`.

### Future option: UI and browser automation

The template ships no UI, so agents see the app only through telemetry, API snapshots and CLI transcripts. Do not build any of this now. Instead, ship `docs/decisions/adding-a-ui.md` in the template so a future agent knows the intended path:

| UI kind | Agent visibility | CI tests | Recording backend for `just record` |
| --- | --- | --- | --- |
| Web pages served by axum, or Leptos/Dioxus web | `chrome-devtools-mcp`: screenshots, DOM snapshots, console, network and performance traces | Playwright end-to-end tests | Playwright video (`video: "on"`) plus trace files (`trace: "on"`), which also hold DOM snapshots and network logs |
| Tauri desktop | A Tauri MCP plugin registered in debug builds only (it can run arbitrary JS) | Tauri WebDriver (`tauri-driver`) | Run under a virtual display (Xvfb) and capture it with `ffmpeg -f x11grab` while the WebDriver journey runs; Linux only (verify `tauri-driver` platform support) |
| egui native | `egui_kittest` via the accessibility tree | `egui_kittest` image snapshots | Render a frame per journey step with `egui_kittest` (headless wgpu) and stitch the frames into MP4/GIF with `ffmpeg`; no display needed |

The design doc should also say how a UI joins the existing harness:

- A new crate under `crates/` that depends only on `cli::client` or the HTTP API, never on `core` directly, plus a matching edge in the architecture test.
- `just ui-test` and `just ui-snap` recipes, added to tier 1, writing results to `target/harness/`.
- The browser MCP server added to `.mcp.json` and `.codex/config.toml` through a `just mcp browser` recipe, so it targets this worktree's app URL from `.harness/app.json`.
- W3C `traceparent` propagated from the page to the server, so one click can be followed from the browser into the backend span tree with `just trace`.
- An extra budget such as "no console errors during `just ui-test`".

**Before/after recordings are required for every UI.** OpenAI's harness lets an agent reproduce a bug, record a video of the failure, fix it, record a second video of the fix, and open the PR with both ([OpenAI, Harness engineering](https://openai.com/index/harness-engineering/), "Increasing levels of autonomy"). The template does not build this now, because it has no UI. `adding-a-ui.md` specifies it so the first UI ships with it:

- **One interface.** `just record <journey> before|after` (requires `just up`) works the same for every UI kind; only the backend in the table above changes. Journeys are code: a Playwright spec for web, a WebDriver test for Tauri, an `egui_kittest` test for egui. They live in `harness/evidence/` (reusable) or next to the exec plan (task-specific).
- **Same outputs.** Each recording writes `.harness/evidence/<plan>/<label>.mp4` (or `.gif`) plus a JSON bundle: every `request_id` and `trace_id` logged during the run, any `WARN`/`ERROR` lines, browser console errors and failed network requests (web and Tauri), and the `just budgets` result.
- **Bug-fix loop.** Reproduce the bug and write the journey; `just record <journey> before` must show the failure; write the failing test and fix; `just record <journey> after` must be clean; open the PR with both bundle summaries. This becomes an `AGENTS.md` rule when the UI is added.
- **CI reproduces it.** `gh` cannot attach files to a PR description, and an agent's own recordings prove little. A workflow `evidence.yml` records every changed journey on the base branch (`before`) and the PR head (`after`), uploads videos and bundles as artifacts, and posts a PR comment linking them with a diff of the two bundles.
- **Enforced, not remembered.** A CI check `evidence-required` fails a PR that changes a UI crate without adding or updating a journey, with the message: `UI change without evidence. Add or update a journey and run just record <journey> before|after; see docs/testing/evidence.md.` Pure refactors opt out with a `no-visual-change` PR label, which a human applies.
- **Budgets apply.** A recorded journey also fails on console errors or a broken latency budget, so the video is never the only signal.

### ASK the human before proceeding

Answered by the human on 2026-10-07:

1. **Template location:** a local git repository for now, not on GitHub. Generate with `cargo generate --path <template-repo>`. Tag releases locally; switch to `--git` if the repo is pushed later.
2. **Docker:** required on every developer machine. `just up` and `just e2e` depend on it.
3. **Unusable tools:** if anything in the verify list turns out unusable (for example the traces MCP server), stop and ask the human before falling back to the `just q-*` recipes or any other substitute. Do not decide this alone.
4. **workz (asked 2026-10-07 during the build):** workz 0.11.0, the latest release, ignores `base_port`, refuses the main checkout, and keys port ranges by branch name across all repos (detached-HEAD worktrees share one range). Fixes exist only on unreleased upstream main. Decision: drop workz and use the hash-of-worktree-path fallback from Section 6, computed in the `justfile`; `.workz.toml` and the WorktreeCreate hook are not needed.

No other open questions remain. New ones go to the human before work continues, and the answer is recorded here.
