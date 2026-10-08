# Every task in this repo is a recipe here. Hooks (hk.pkl), CI (.github/workflows) and agents
# call the same recipes, so they never disagree. Run `just --list` for the menu: recipes are
# grouped by use; the plumbing that hooks, CI and the gates call is private (callable, unlisted).
#
# This file is copied verbatim by cargo-generate (it uses {{ }} natively): it must not contain
# template placeholders. The project name comes from PROJECT_NAME in mise.toml.

set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-path := ".env.local" # optional per-worktree overrides (HARNESS_PORT); never committed
set dotenv-override # .env.local beats inherited values
set quiet # no command echo; stdio MCP servers need a clean stdout

root := justfile_directory()
# PROJECT_NAME comes from mise.toml [env]; read the file directly when mise is not active
# (MCP clients and git hooks may start `just` without it).
project := env("PROJECT_NAME", shell('sed -n "s/^PROJECT_NAME *= *\"\\(.*\\)\"$/\\1/p" "$1/mise.toml"', root))
service := env("OTEL_SERVICE_NAME", project)

# ── Harness variables: one 10-port range per git worktree ──────────────────────────────────
# The range is derived from a hash of this worktree's path (20000-29990, step 10), so every
# worktree, agent and MCP config computes the same ports with no registry. Override a
# collision with HARNESS_PORT=<base> in .env.local. See docs/architecture/port-allocation.md.
path_crc := shell('printf %s "$1" | cksum | cut -d" " -f1', root)
wt := shell('basename "$1" | tr "[:upper:]" "[:lower:]" | tr -c "a-z0-9\n" "-"', root) + "-" + shell('printf %06x $(($1 % 16777216))', path_crc)
base := env("HARNESS_PORT", shell('echo $((20000 + ($1 % 1000) * 10))', path_crc))

export COMPOSE_PROJECT_NAME := project + "-" + wt
export VM_PORT := base
export VL_PORT := shell('echo $(($1 + 1))', base)
export VT_PORT := shell('echo $(($1 + 2))', base)
export OTEL_EXPORTER_OTLP_METRICS_ENDPOINT := "http://127.0.0.1:" + VM_PORT + "/opentelemetry/v1/metrics"
export OTEL_EXPORTER_OTLP_LOGS_ENDPOINT := "http://127.0.0.1:" + VL_PORT + "/insert/opentelemetry/v1/logs"
export OTEL_EXPORTER_OTLP_TRACES_ENDPOINT := "http://127.0.0.1:" + VT_PORT + "/insert/opentelemetry/v1/traces"
export OTEL_EXPORTER_OTLP_PROTOCOL := "http/protobuf"
export OTEL_RESOURCE_ATTRIBUTES := "worktree=" + wt
export OTEL_METRIC_EXPORT_INTERVAL := "5000"
export APP_LOG_JSON := root + "/.harness/logs/app.jsonl"

compose := "docker compose -f harness/stack/compose.yaml"

# Separate Cargo workspaces (harness/dst, harness/fuzz, harness/lints): fmt and clippy run in each.
workspaces := ". harness/dst harness/fuzz harness/lints"

# List recipes, grouped, in file order
[private]
default:
    just --list --unsorted

# ── Gate plumbing ──────────────────────────────────────────────────────────────────────────

# Run a command as a named check: log to target/harness/<check>.log, write
# target/harness/<check>.json {check, ok, summary, details_path, repro}, print a one-line verdict.
# VERBOSE=1 streams the output instead of capturing it.
[positional-arguments]
[private]
gate check *cmd:
    #!/usr/bin/env bash
    set -uo pipefail
    check="$1"; shift
    mkdir -p "{{ root }}/target/harness"
    log="target/harness/$check.log"
    json="target/harness/$check.json"
    repro="$*"
    rm -f "{{ root }}/$json"
    start=$(date +%s)
    if [ "${VERBOSE:-0}" = 1 ]; then "$@" 2>&1 | tee "{{ root }}/$log"; status=${PIPESTATUS[0]}
    else "$@" >"{{ root }}/$log" 2>&1; status=$?; fi
    secs=$(( $(date +%s) - start ))
    if [ "$status" = 0 ]; then ok=true; summary="passed in ${secs}s"
    else
      ok=false
      summary=$(grep -E '^(error|FAIL|failed|Error|warning: unused)' "{{ root }}/$log" | head -n 1 | cut -c1-200 || true)
      [ -n "$summary" ] || summary="exit status $status after ${secs}s"
    fi
    # A check may write its own richer JSON; only fill in what is missing.
    if jq -e --arg c "$check" '.check == $c and has("ok")' "{{ root }}/$json" >/dev/null 2>&1; then
      jq --argjson ok "$ok" '.ok = (.ok and $ok)' "{{ root }}/$json" > "{{ root }}/$json.tmp" && mv "{{ root }}/$json.tmp" "{{ root }}/$json"
    else
      jq -n --arg check "$check" --argjson ok "$ok" --arg summary "$summary" \
        --arg details "$log" --arg repro "$repro" \
        '{check: $check, ok: $ok, summary: $summary, details_path: $details, repro: $repro}' > "{{ root }}/$json"
    fi
    if [ "$ok" = true ]; then printf 'ok    %-14s %s\n' "$check" "$summary"
    else
      printf 'FAIL  %-14s %s\n' "$check" "$summary"
      [ "${VERBOSE:-0}" = 1 ] || tail -n 60 "{{ root }}/$log" | sed 's/^/      /'
      printf '      details: %s  repro: %s\n' "$log" "$repro"
    fi
    exit "$status"

# Run several gates, keep going after failures, exit non-zero if any failed. A spec is a recipe
# name, or `<check-name>=<recipe and args>` when the check runs a recipe with arguments
# (for example "fmt-check=fmt --check"); the check is named and reported by its left side.
[positional-arguments]
[private]
gates *specs:
    #!/usr/bin/env bash
    set -uo pipefail
    failed=()
    for spec in "$@"; do
      name=${spec%%=*}; cmd=${spec#*=}; [ "$spec" != "$name" ] || cmd=$name
      # shellcheck disable=SC2086 # the spec's recipe and arguments are split into words on purpose
      just gate "$name" just $cmd 2>/dev/null || failed+=("$name")
    done
    if [ ${#failed[@]} -gt 0 ]; then echo "FAILED: ${failed[*]}" >&2; exit 1; fi

# ── Setup ──────────────────────────────────────────────────────────────────────────────────

# One-time setup per clone: lockfile, git hooks, tool checks, first build
[group('setup')]
bootstrap:
    #!/usr/bin/env bash
    set -euo pipefail
    for tool in cargo hk jq cargo-nextest; do
      command -v "$tool" >/dev/null || { echo "Missing $tool. Run: mise install" >&2; exit 1; }
    done
    docker info >/dev/null 2>&1 || echo "warning: Docker is not available; just up and just e2e need it" >&2
    for ws in . harness/dst harness/fuzz; do [ -f "$ws/Cargo.lock" ] || (cd "$ws" && cargo generate-lockfile); done
    hk install --mise
    cargo build --workspace --all-targets --locked
    echo '{"bootstrap": "ok", "next": ["just up", "just check"]}'

# Regenerate docs/generated/ (crate graph, lint-exception ledger) from the source
[group('setup')]
docs:
    CHECKS_BLESS=1 cargo nextest run --locked -p checks -E 'binary(arch)' --no-fail-fast
    jq -n '{docs: "regenerated", files: ["docs/generated/crate-graph.txt", "docs/generated/lint-exceptions.txt"]}'

# Fail fast when the git hooks are not installed (skipped in CI, which runs `hk check --all`)
[no-exit-message]
[private]
hooks-installed:
    if [ "${CI:-}" != true ] && ! git hook list pre-commit 2>/dev/null | grep -qx hk-pre-commit \
      && ! grep -qs 'hk run pre-commit' "$(git rev-parse --git-common-dir)/hooks/pre-commit"; then \
      echo "Git hooks not installed. Run: just bootstrap" >&2; exit 1; fi

# Install the pinned analysis nightly ($NIGHTLY) if missing
[private]
nightly:
    rustup toolchain list | grep -q "^$NIGHTLY" || rustup toolchain install "$NIGHTLY" --profile minimal -c rust-src,llvm-tools-preview,miri,clippy,rustfmt >&2

# Host target triple, for sanitizer builds
[private]
host:
    rustc -vV | sed -n 's/^host: //p'

# ── Tiers ─────────────────────────────────────────────────────────────────────────────────

# Tier 0: fmt, clippy, tests, machete in every workspace, project memory (also runs on commit)
[group('tiers')]
check: hooks-installed
    just gates "fmt-check=fmt --check" clippy test machete knowledge lint-recipes

# Tier 1: everything CI runs on a pull request
[group('tiers')]
ci:
    just gates "fmt-check=fmt --check" clippy test machete knowledge lint-recipes hk-all deny dylint coverage panic-audit asm-snapshots gungraun miri kani dst

# Tier 2: performance signals (remarks, llvm-mca, Criterion, llvm-lines, build timings). Never fails.
[group('tiers')]
perf:
    -just gates remarks mca criterion llvm-lines timings
    jq -s '{check: "perf", informational: true, signals: .}' target/harness/remarks.json target/harness/mca.json target/harness/criterion.json target/harness/llvm-lines.json target/harness/timings.json | tee target/harness/perf.json

# Tier 3: tier 1 plus fuzzing, sanitizers, full Kani, DST at volume, mutation testing (budgets: FUZZ_SECS per target, default 600; DST_SEEDS, default 100000)
[group('tiers')]
harden:
    just ci
    just gates "fuzz=fuzz all ${FUZZ_SECS:-600}" asan tsan "kani-full=kani full" "dst-volume=dst SEEDS=${DST_SEEDS:-100000} START=${DST_START:-0}" mutants

# Tier 4: release checks (public API diff against the last tag)
[group('tiers')]
release:
    just gates public-api

# ── Per-worktree stack and server ─────────────────────────────────────────────────────────

# Start this worktree's telemetry stack and server; prints URLs as JSON
[group('stack')]
up: hooks-installed check-ports
    mkdir -p .harness/logs
    {{ compose }} up -d --quiet-pull >&2
    just wait-stack
    just stack-json > .harness/stack.json
    just env > .harness/env
    just start-server
    just wait-ready
    jq -n --slurpfile app .harness/app.json --slurpfile stack .harness/stack.json \
      '{app: $app[0].url, ui: $stack[0].ui, ports: $stack[0].ports, logs: "just logs", down: "just down"}'

# Stop the server and remove the stack, its volumes and .harness/
[group('stack')]
down:
    just stop-server
    {{ compose }} down -v --remove-orphans >&2 2>/dev/null || true
    rm -rf .harness

# Rebuild and restart only the server (keeps the stack and its data)
[group('stack')]
restart:
    just stop-server
    just start-server
    just wait-ready
    jq -c '{app: .url, pid}' .harness/app.json

# JSON: stack health, server process, readiness, ports and the Victoria UI URLs
[group('stack')]
status:
    #!/usr/bin/env bash
    set -uo pipefail
    health() { curl -fsS -m 2 "http://127.0.0.1:$1/health" >/dev/null 2>&1 && echo true || echo false; }
    pid=$(cat .harness/server.pid 2>/dev/null || echo "")
    alive=false; [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null && alive=true
    url=$(jq -r .url .harness/app.json 2>/dev/null || echo "")
    ready=false; [ -n "$url" ] && curl -fsS -m 2 "$url/readyz" >/dev/null 2>&1 && ready=true
    jq -n --arg wt "{{ wt }}" --argjson base "{{ base }}" --arg url "$url" --arg pid "$pid" \
      --argjson vm "$(health "$VM_PORT")" --argjson vl "$(health "$VL_PORT")" --argjson vt "$(health "$VT_PORT")" \
      --argjson alive "$alive" --argjson ready "$ready" \
      '{worktree: $wt, ports: {range: [$base, $base + 9], metrics: $base, logs: ($base + 1), traces: ($base + 2)},
        stack: {victoria_metrics: $vm, victoria_logs: $vl, victoria_traces: $vt},
        ui: {metrics: "http://127.0.0.1:\($base)/vmui/", logs: "http://127.0.0.1:\($base + 1)/select/vmui/", traces: "http://127.0.0.1:\($base + 2)/select/vmui/"},
        server: {url: (if $url == "" then null else $url end), pid: (if $pid == "" then null else ($pid | tonumber) end), alive: $alive, ready: $ready}}'

# Print harness variables for a shell: eval "$(just env)"
[group('stack')]
env:
    env | grep -E '^(COMPOSE_PROJECT_NAME|VM_PORT|VL_PORT|VT_PORT|OTEL_[A-Z_]+|APP_LOG_JSON)=' | sort | sed 's/^/export /'

# Remove stacks of this project whose worktree directory no longer exists
[group('stack')]
harness-gc:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose ls --all --format json | jq -r --arg p "{{ project }}-" \
      '.[] | select(.Name | startswith($p)) | "\(.Name)\t\(.ConfigFiles)"' |
    while IFS=$'\t' read -r name files; do
      if [ ! -e "${files%%,*}" ]; then
        docker compose -p "$name" down -v --remove-orphans >&2 && echo "{\"removed\":\"$name\"}"
      fi
    done

# Claude Code WorktreeRemove hook: stop the worktree's stack, then remove the worktree
[private]
worktree-remove:
    #!/usr/bin/env bash
    set -uo pipefail
    path=$(jq -r '.worktree_path // empty')
    [ -n "$path" ] && [ -d "$path" ] || exit 0
    (cd "$path" && just down) >&2 || true
    git worktree remove --force "$path" >&2

# Start a Victoria MCP server for this worktree (called by .mcp.json and .codex/config.toml)
[no-exit-message]
[private]
mcp kind:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ kind }}" in
      metrics) MCP_LOG_LEVEL=warn VM_INSTANCE_ENTRYPOINT="http://127.0.0.1:$VM_PORT" VM_INSTANCE_TYPE=single exec mcp-victoriametrics ;;
      logs)    MCP_LOG_LEVEL=warn VL_INSTANCE_ENTRYPOINT="http://127.0.0.1:$VL_PORT" exec mcp-victorialogs ;;
      traces)  MCP_LOG_LEVEL=warn VT_INSTANCE_ENTRYPOINT="http://127.0.0.1:$VT_PORT" exec mcp-victoriatraces ;;
      *) echo "usage: just mcp metrics|logs|traces" >&2; exit 2 ;;
    esac

# Fail if this worktree's ports are taken by something other than its own stack
[no-exit-message]
[private]
check-ports:
    #!/usr/bin/env bash
    set -uo pipefail
    ours=$({{ compose }} ps --format '{{{{.Publishers}}' 2>/dev/null || true)
    for p in $VM_PORT $VL_PORT $VT_PORT; do
      (exec 3<>"/dev/tcp/127.0.0.1/$p") 2>/dev/null || continue
      grep -q ":$p->" <<<"$ours" && continue
      who=$(lsof -nP -iTCP:"$p" -sTCP:LISTEN -Fc 2>/dev/null | sed -n 's/^c//p' | head -1)
      echo "Port $p is taken by ${who:-an unknown process}. Pick another range: echo HARNESS_PORT=<free base, multiple of 10> >> .env.local, then restart your agent's MCP servers." >&2
      exit 1
    done

[private]
wait-stack:
    #!/usr/bin/env bash
    set -uo pipefail
    for p in $VM_PORT $VL_PORT $VT_PORT; do
      for _ in $(seq 1 60); do curl -fsS -m 1 "http://127.0.0.1:$p/health" >/dev/null 2>&1 && continue 2; sleep 0.5; done
      echo "Victoria service on port $p not healthy after 30s. Run: {{ compose }} logs" >&2; exit 1
    done

[private]
stack-json:
    #!/usr/bin/env bash
    jq -n --argjson base "{{ base }}" --arg wt "{{ wt }}" --arg project "$COMPOSE_PROJECT_NAME" \
      --arg m "$OTEL_EXPORTER_OTLP_METRICS_ENDPOINT" --arg l "$OTEL_EXPORTER_OTLP_LOGS_ENDPOINT" --arg t "$OTEL_EXPORTER_OTLP_TRACES_ENDPOINT" \
      '{worktree: $wt, compose_project: $project,
        ports: {range: [$base, $base + 9], metrics: $base, logs: ($base + 1), traces: ($base + 2)},
        ui: {metrics: "http://127.0.0.1:\($base)/vmui/", logs: "http://127.0.0.1:\($base + 1)/select/vmui/", traces: "http://127.0.0.1:\($base + 2)/select/vmui/"},
        otlp: {metrics: $m, logs: $l, traces: $t}}'

[private]
start-server:
    cargo build -q -p "{{ project }}-server"
    rm -f .harness/app.json
    nohup "target/debug/{{ project }}-server" > .harness/logs/server.out 2>&1 & echo $! > .harness/server.pid

[private]
stop-server:
    #!/usr/bin/env bash
    set -uo pipefail
    pid=$(cat .harness/server.pid 2>/dev/null) || exit 0
    kill -TERM "$pid" 2>/dev/null || exit 0
    for _ in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || exit 0; sleep 0.2; done
    kill -KILL "$pid" 2>/dev/null || true

[no-exit-message]
[private]
wait-ready:
    #!/usr/bin/env bash
    set -uo pipefail
    for _ in $(seq 1 120); do
      if [ -f .harness/app.json ] && curl -fsS -m 1 "$(jq -r .url .harness/app.json)/readyz" >/dev/null 2>&1; then exit 0; fi
      sleep 0.25
    done
    echo "Server not ready after 30s. Last 40 lines of .harness/logs/server.out:" >&2
    tail -n 40 .harness/logs/server.out >&2 || true
    just logs >&2 || true
    exit 1

# ── Seeing what the app does (capped output; aggregate, never dump) ───────────────────────

# Local JSON log: the last 50 ERROR/WARN lines, or every line for one request id (max 200)
[group('observe')]
logs request_id="":
    #!/usr/bin/env bash
    set -euo pipefail
    [ -f .harness/logs/app.jsonl ] || { echo "No log file yet: run just up" >&2; exit 1; }
    if [ -z "{{ request_id }}" ]; then
      jq -c 'select(.level == "ERROR" or .level == "WARN") | {ts: .timestamp, level, msg: .fields.message, target, request_id: ([.spans[]?.request_id] | map(select(.)) | first), fields: (.fields | del(.message))}' .harness/logs/app.jsonl | tail -n 50
    else
      jq -c --arg id "{{ request_id }}" 'select([.span.request_id?, .spans[]?.request_id] | index($id)) | {ts: .timestamp, level, msg: .fields.message, target, fields: (.fields | del(.message))}' .harness/logs/app.jsonl | head -n 200
    fi

# Query this worktree's stack: `logs` or `traces` take LogsQL (newline JSON, max 200 lines), `metrics` takes PromQL/MetricsQL (instant result vector)
[group('observe')]
[positional-arguments]
query backend query:
    #!/usr/bin/env bash
    set -euo pipefail
    case "$1" in
      logs)    curl -fsS "http://127.0.0.1:$VL_PORT/select/logsql/query" --data-urlencode "query=$2" -d limit=200 ;;
      traces)  curl -fsS "http://127.0.0.1:$VT_PORT/select/logsql/query" --data-urlencode "query=$2" -d limit=200 ;;
      metrics) curl -fsS "http://127.0.0.1:$VM_PORT/api/v1/query" --data-urlencode "query=$2" | jq -c '.data.result' ;;
      *) echo "usage: just query logs|traces|metrics '<query>'" >&2; exit 2 ;;
    esac

# One trace via the Jaeger API, one span per line: name, duration_ms, parent, status
[group('observe')]
trace trace_id:
    curl -fsS "http://127.0.0.1:$VT_PORT/select/jaeger/api/traces/{{ trace_id }}" | jq -c '.data[0] as $t | $t.spans | sort_by(.startTime)[] | {name: .operationName, span_id: .spanID, parent: ((.references // []) | map(select(.refType == "CHILD_OF")) | .[0].spanID // null), duration_ms: (.duration / 1000), status: ((.tags | map(select(.key == "error")) | .[0].value) // "unset"), service: $t.processes[.processID].serviceName}' | head -n 500

# Run every budget in docs/observability/budgets.md; pass/fail JSON (target/harness/budgets.json)
[group('observe')]
budgets:
    #!/usr/bin/env bash
    set -uo pipefail
    mkdir -p target/harness
    start=$(cat .harness/e2e.start 2>/dev/null || date -u -v-15M +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -d '-15 min' +%Y-%m-%dT%H:%M:%SZ)
    end=$(cat .harness/e2e.end 2>/dev/null || date -u +%Y-%m-%dT%H:%M:%SZ)
    awk '/^```jsonl budgets/{f=1; next} /^```/{f=0} f' docs/observability/budgets.md > target/harness/budgets.jsonl
    results=()
    while IFS= read -r budget; do
      query=$(jq -r .query <<<"$budget" | sed -e "s|\$SERVICE|{{ service }}|g" -e "s|\$START|$start|g" -e "s|\$END|$end|g")
      value=null
      for _ in $(seq 1 30); do
        case $(jq -r .backend <<<"$budget") in
          logs)    value=$(curl -fsS "http://127.0.0.1:$VL_PORT/select/logsql/query" --data-urlencode "query=$query" | jq -s '.[0].value // "0" | tonumber' 2>/dev/null) ;;
          traces)  value=$(curl -fsS "http://127.0.0.1:$VT_PORT/select/logsql/query" --data-urlencode "query=$query" | jq -s '.[0].value // null | if . == null or . == "" then null else tonumber end' 2>/dev/null) ;;
          metrics) value=$(curl -fsS "http://127.0.0.1:$VM_PORT/api/v1/query" --data-urlencode "query=$query" | jq '.data.result[0].value[1] // null | if . == null or . == "NaN" then null else tonumber end' 2>/dev/null) ;;
        esac
        [ -n "$value" ] && [ "$value" != null ] && break
        value=null; sleep 1
      done
      results+=("$(jq -c --argjson value "$value" --arg q "$query" '{name, backend, value: $value, max, unit, query: $q, ok: ($value != null and $value <= .max)}' <<<"$budget")")
    done < target/harness/budgets.jsonl
    printf '%s\n' "${results[@]}" | jq -s --arg start "$start" --arg end "$end" \
      '{check: "budgets", ok: (length > 0 and all(.ok)), summary: "\(map(select(.ok)) | length)/\(length) budgets pass", window: {start: $start, end: $end}, budgets: ., details_path: "docs/observability/budgets.md", repro: "just budgets"}' \
      | tee target/harness/budgets.json
    jq -e .ok target/harness/budgets.json >/dev/null

# ── Checks (tier 0 and 1 building blocks; each writes target/harness/<check>.json via `gates`) ──

# Unit, property, snapshot, transcript and architecture tests (not e2e)
[group('checks')]
test *args:
    cargo nextest run --workspace --locked --no-tests=warn -E 'not binary(e2e) & not binary(asm)' {{ args }}

# End-to-end tests against `just up`, then the latency and error budgets
[group('checks')]
e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    [ -f .harness/app.json ] || { echo "No running server: run just up first" >&2; exit 1; }
    date -u +%Y-%m-%dT%H:%M:%SZ > .harness/e2e.start
    status=0
    cargo nextest run -p checks --profile "${NEXTEST_PROFILE:-default}" -E 'binary(e2e)' || status=$?
    date -u +%Y-%m-%dT%H:%M:%SZ > .harness/e2e.end
    [ "$status" = 0 ] || exit "$status"
    just budgets

# Format every workspace (`just fmt --check` only checks)
[group('checks')]
fmt *flags:
    for ws in {{ workspaces }}; do [ -f "$ws/Cargo.toml" ] || continue; (cd "$ws" && cargo fmt --all {{ flags }}); done

# Clippy with -D warnings in every workspace
[group('checks')]
clippy:
    for ws in {{ workspaces }}; do [ -f "$ws/Cargo.toml" ] || continue; (cd "$ws" && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings); done

# Unused dependencies
[group('checks')]
machete:
    cargo machete

# Validate the docs/ OKF bundle: schema, links, drift, staleness. Any warning fails.
[group('checks')]
knowledge:
    #!/usr/bin/env bash
    set -uo pipefail
    mkdir -p target/harness
    okf validate docs --strict --drift --stale --json > target/harness/okf-validate.json
    # AGENTS.md's okf memory block stays within okf's token budget (AAG rules); --json exits 0
    okf agents lint --strict --json AGENTS.md > target/harness/okf-agents.json 2>/dev/null
    jq --slurpfile agents target/harness/okf-agents.json '($agents[0] // {passed: false, findings: [{line: 0, rule_id: "lint", message: "okf agents lint produced no result"}]}) as $aag
      | {check: "knowledge", ok: (.gate_passed and .is_conformant and ((.warnings // []) | length) == 0 and ((.errors // []) | length) == 0 and $aag.passed),
         concepts: .concept_count, broken_links: ((.broken_links // []) | length), orphans: ((.orphans // []) | length),
         drifted: ([(.warnings // [])[] | select(test("differs from concept description|not listed in parent index|does not exist|non-existent"))] | length),
         stale: .stale_count, agents_md_tokens: $aag.token_stats.estimated_tokens,
         findings: ((.errors // []) + (.gate_findings // []) + (.warnings // []) + [($aag.findings // [])[] | "AGENTS.md:\(.line) \(.rule_id) \(.message)"]),
         summary: "\(.concept_count) concepts, \((.errors // []) | length) errors, \((.gate_findings // []) | length) gate findings, \((.warnings // []) | length) warnings; AGENTS.md lint \(if $aag.passed then "passed" else "failed" end)",
         details_path: "target/harness/okf-validate.json", repro: "okf validate docs --strict --drift --stale && okf agents lint --strict AGENTS.md"}' \
      target/harness/okf-validate.json > target/harness/knowledge.json
    jq -r '.findings[]' target/harness/knowledge.json >&2
    jq -e .ok target/harness/knowledge.json > /dev/null

# Lint the justfiles: `just --fmt` layout, then shellcheck on every bash recipe body
[group('checks')]
lint-recipes:
    #!/usr/bin/env bash
    set -euo pipefail
    fmt_hint="Run: just --unstable --fmt"
    just --unstable --fmt --check >/dev/null || { echo "justfile is not formatted. $fmt_hint" >&2; exit 1; }
    if [ -f template.just ]; then
      just --justfile template.just --working-directory . --unstable --fmt --check >/dev/null \
        || { echo "template.just is not formatted. Run: just --justfile template.just --working-directory . --unstable --fmt" >&2; exit 1; }
    fi
    # One file per recipe body. Interpolations become ${JUST_EXPR}; linewise recipes are checked
    # as one bash script, so line numbers match the recipe body.
    out=target/harness/recipes
    rm -rf "$out" && mkdir -p "$out"
    dump=$(just --dump --dump-format json)
    for name in $(jq -r '.recipes | keys[]' <<<"$dump"); do
      jq -r --arg n "$name" '.recipes[$n] as $r
        | ($r.body | map(map(if type == "string" then . else "${JUST_EXPR}" end) | join(""))) as $lines
        | if $r.shebang then (if ($lines[0] | test("bash")) then $lines else [] end)
          else ($lines | map(sub("^[@-]+"; ""))) end
        | join("\n")' <<<"$dump" > "$out/$name.sh"
    done
    cd "$out" && shellcheck --shell=bash --severity=warning --format=gcc ./*.sh \
      || { echo "shellcheck found problems in the recipes above (<recipe>.sh:<body line>). Fix the recipe in the justfile." >&2; exit 1; }

# `offline` (the pre-commit step) fetches first: offline metadata needs every platform's crates,
# and builds only download the host's.

# cargo-deny: every check (advisories need the network), or `just deny offline` for bans, licenses and sources only
[group('checks')]
deny mode="":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ mode }}" in
      "") cargo deny --locked check ;;
      offline) cargo fetch --locked -q && cargo deny --offline check bans licenses sources ;;
      *) echo "usage: just deny [offline]" >&2; exit 2 ;;
    esac

# Project lints from harness/lints (dylint, its own nightly); warnings fail
[group('checks')]
dylint:
    RUSTC_WRAPPER='' DYLINT_RUSTFLAGS="-D warnings" cargo dylint --all -- --all-targets

# Coverage gate: workspace region coverage must stay >= COV_MIN_REGIONS (mise.toml)
[group('checks')]
coverage:
    #!/usr/bin/env bash
    set -uo pipefail
    mkdir -p target/harness
    cargo llvm-cov clean --workspace   # stale profiles from earlier runs would inflate the number
    cargo llvm-cov nextest --workspace --locked --no-report -E 'not binary(e2e) & not binary(asm)' || exit $?
    cargo llvm-cov report --json --output-path target/harness/cov.json --show-missing-lines > target/harness/cov-missing.txt
    pct=$(jq '.data[0].totals.regions.percent' target/harness/cov.json)
    ok=$(jq -n --argjson p "$pct" --argjson m "${COV_MIN_REGIONS:-0}" '$p >= $m')
    jq -n --argjson ok "$ok" --arg pct "$pct" --arg min "${COV_MIN_REGIONS:-0}" \
      '{check: "coverage", ok: $ok, summary: "region coverage \($pct)% (minimum \($min)%)", details_path: "target/harness/cov-missing.txt", repro: "just coverage"}' > target/harness/coverage.json
    [ "$ok" = true ] || { echo "Region coverage $pct% is below COV_MIN_REGIONS=${COV_MIN_REGIONS:-0}%. Add tests for the lines in target/harness/cov-missing.txt; never lower the threshold." >&2; exit 1; }

# Raise COV_MIN_REGIONS to the measured coverage minus 0.5 (never lowers it)
[group('checks')]
cov-ratchet: coverage
    #!/usr/bin/env bash
    set -euo pipefail
    new=$(jq '.data[0].totals.regions.percent - 0.5 | . * 10 | floor / 10' target/harness/cov.json)
    if jq -e -n --argjson n "$new" --argjson m "${COV_MIN_REGIONS:-0}" '$n > $m' >/dev/null; then
      sed -i.bak "s/^COV_MIN_REGIONS = \".*\"/COV_MIN_REGIONS = \"$new\"/" mise.toml && rm -f mise.toml.bak
      echo "{\"cov_min_regions\": $new}"
    else
      echo "{\"cov_min_regions\": ${COV_MIN_REGIONS:-0}, \"unchanged\": true}"
    fi

# Uses $NIGHTLY with -Zcross-crate-inline-threshold=never: on stable, small functions are only
# codegen'd in the crates that call them, so their panic paths would be invisible here.

# Functions in core's release build that can panic, compared with docs/generated/panic-allowlist.txt
[group('checks')]
panic-audit: nightly
    #!/usr/bin/env bash
    set -uo pipefail
    mkdir -p target/harness
    crate=$(tr - _ <<<"{{ project }}")_core
    # --callers-of matches mangled names, hence fragments rather than paths
    regex='panic_bounds_check|panic_fmt|panic_const|panic_nounwind|panic_explicit|begin_panic|9panicking5panic|assert_failed|unwrap_failed|expect_failed|index_len_fail|index_order_fail|slice_error_fail|str_index_overflow_fail'
    RUSTFLAGS="-Zcross-crate-inline-threshold=never" CARGO_TARGET_DIR=target/panic-audit \
      cargo +"$NIGHTLY" asm --lib -p "{{ project }}-core" --llvm -s --json --callers-of "$regex" 1 2>target/harness/panic-audit.log \
      | jq -r '.[].name' | grep -E "^<?${crate}::| as [^ ]*${crate}::" | sort -u > target/harness/panic-callers.txt
    new=$(grep -v -e '^#' -e '^$' docs/generated/panic-allowlist.txt | sort -u | comm -23 target/harness/panic-callers.txt -)
    count=$(grep -c . <<<"$new" || true)
    jq -n --arg new "$new" --argjson count "$count" '{check: "panic-audit", ok: ($count == 0), summary: (if $count == 0 then "no new panic paths" else "\($count) new panic path(s)" end), new_paths: ($new | split("\n") | map(select(. != ""))), details_path: "target/harness/panic-callers.txt", repro: "just panic-audit"}' > target/harness/panic-audit.json
    [ "$count" = 0 ] && exit 0
    while IFS= read -r fn; do
      echo "New panic path in $fn. Remove it (iterators, hoisted assert, checked ops) or add it to the allowlist with a reason in the PR." >&2
    done <<<"$new"
    exit 1

# Assembly snapshots of docs/performance/hot-paths.md (review diffs with `cargo insta review`)
[group('checks')]
asm-snapshots:
    INSTA_UPDATE=no cargo nextest run --locked -p checks -E 'binary(asm)'

# Instruction counts (Gungraun, Linux + Valgrind): `just gungraun base` saves a baseline, `just gungraun "" base` compares and fails (exit 3) on a >2% regression
[group('checks')]
gungraun save="" baseline="":
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p target/harness
    if [ "$(uname -s)" != Linux ] || ! command -v valgrind >/dev/null; then
      jq -n '{check: "gungraun", ok: true, summary: "skipped: gungraun needs Valgrind (Linux)", details_path: null, repro: "just gungraun"}' > target/harness/gungraun.json
      echo "skipped: gungraun needs Valgrind (Linux)"; exit 0
    fi
    export GUNGRAUN_HOME="{{ root }}/target/gungraun"
    args=()
    [ -n "{{ save }}" ] && args+=(--save-baseline="{{ save }}")
    [ -n "{{ baseline }}" ] && args+=(--baseline="{{ baseline }}")
    cargo bench --locked -p "{{ project }}-core" --bench instructions -- ${args[@]+"${args[@]}"}

# Save a Gungraun baseline named `base` from another commit (CI: the PR's base)
[private]
gungraun-baseline sha:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "$(uname -s)" != Linux ] || ! command -v valgrind >/dev/null; then echo "skipped: gungraun needs Valgrind (Linux)"; exit 0; fi
    dir="{{ root }}/target/gungraun-base-src"
    rm -rf "$dir" && git worktree prune && git worktree add --detach "$dir" "{{ sha }}" >&2
    (cd "$dir" && GUNGRAUN_HOME="{{ root }}/target/gungraun" cargo bench -p "{{ project }}-core" --bench instructions -- --save-baseline=base) >&2 \
      || echo "base commit has no instruction benchmarks; comparing against nothing" >&2
    git worktree remove --force "$dir"

# Architecture tests only (pre-push gate; `just test` includes them)
[private]
arch:
    cargo nextest run --locked -p checks -E 'binary(arch)'

# All hk steps on every file (CI backstop for commits made with hooks bypassed)
[private]
hk-all:
    hk check --all

# Public API of each library crate, diffed against BASE (default: the latest tag). Breaking
# changes fail unless ALLOW_BREAKING=1 (say so in the PR). Run through `just release`.
[private]
public-api: nightly
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p target/harness/public-api
    base="${BASE:-$(git describe --tags --abbrev=0 2>/dev/null || true)}"
    [ -n "$base" ] || { echo "No tag to diff against; set BASE=<ref>" >&2; exit 1; }
    deny=(--deny=removed --deny=changed); [ "${ALLOW_BREAKING:-0}" = 1 ] && deny=()
    for crate in core runtime server cli; do
      cargo +"$NIGHTLY" public-api -p "{{ project }}-$crate" diff ${deny[@]+"${deny[@]}"} "$base..HEAD" | tee "target/harness/public-api/$crate.txt"
    done

# ── Hook steps (see hk.pkl) ────────────────────────────────────────────────────────────────

# Warn when a change touches files that hold thresholds, baselines or lint policy
[no-exit-message]
[private]
protected-files *files:
    #!/usr/bin/env bash
    set -uo pipefail
    [ -n "{{ files }}" ] || exit 0
    echo "Protected files changed: {{ files }}" >&2
    echo "These hold thresholds, baselines or lint policy (Cargo.toml: [workspace.lints]; mise.toml: COV_MIN_REGIONS)." >&2
    echo "Ask a human before weakening any of them; never lower a threshold or baseline to make a check pass." >&2
    exit 1

# Scan files for secrets (gitleaks); the pre-commit hook scans the staged diff instead
[private]
secrets *files:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
    for f in {{ files }}; do [ -f "$f" ] && mkdir -p "$tmp/$(dirname "$f")" && cp "$f" "$tmp/$f"; done
    gitleaks dir --no-banner --redact --log-level warn "$tmp"

# Run the tests of the packages that own the given files (all packages if core or the root manifest changed)
[private]
affected-tests *files:
    #!/usr/bin/env bash
    set -euo pipefail
    all=false; pkgs=()
    meta=$(cargo metadata --format-version 1 --no-deps --locked)
    for f in {{ files }}; do
      case "$f" in Cargo.toml|Cargo.lock|crates/*-core/*) all=true ;; esac
      dir=$(dirname "$f")
      while [ "$dir" != . ] && [ ! -f "$dir/Cargo.toml" ]; do dir=$(dirname "$dir"); done
      name=$(jq -r --arg m "$PWD/$dir/Cargo.toml" '.packages[] | select(.manifest_path == $m) | .name' <<<"$meta")
      [ -n "$name" ] && pkgs+=(-p "$name")
    done
    if $all; then just test; elif [ ${#pkgs[@]} -gt 0 ]; then cargo nextest run --locked --no-tests=warn -E 'not binary(e2e) & not binary(asm)' "${pkgs[@]}"; fi

# ── Hardening (tier 1 and 3 building blocks) ──────────────────────────────────────────────

# Deterministic simulation (harness/dst): `just dst` (200 seeds), `just dst SEEDS=1000 START=0`, `just dst SEED=17 TEST=items_survive_faults`
[group('hardening')]
dst *args:
    #!/usr/bin/env bash
    set -uo pipefail
    filter=()
    for arg in {{ args }}; do
      case "$arg" in
        SEED=*) export DST_SEED="${arg#SEED=}" ;;
        SEEDS=*) export DST_SEEDS="${arg#SEEDS=}" ;;
        START=*) export DST_START="${arg#START=}" ;;
        TEST=*) filter=(-E "test(=${arg#TEST=})") ;;
        *) echo "usage: just dst [SEED=<n>] [TEST=<name>] [SEEDS=<count>] [START=<first>]" >&2; exit 2 ;;
      esac
    done
    cd harness/dst && cargo nextest run --locked --no-fail-fast ${filter[@]+"${filter[@]}"}

# Fuzz one target, or all of them, for `secs` seconds each (harness/fuzz; tier 3 runs 600 s); crashes land in harness/fuzz/artifacts/
[group('hardening')]
fuzz target="all" secs="60": nightly
    #!/usr/bin/env bash
    set -uo pipefail
    targets="{{ target }}"
    [ "$targets" != all ] || targets=$(RUSTC_WRAPPER='' cargo +"$NIGHTLY" fuzz list --fuzz-dir harness/fuzz)
    for t in $targets; do
      RUSTC_WRAPPER='' cargo +"$NIGHTLY" fuzz run --fuzz-dir harness/fuzz "$t" -- -max_total_time="{{ secs }}" -timeout=10 && continue
      status=$?
      crash=$(ls -t "harness/fuzz/artifacts/$t"/* 2>/dev/null | head -1)
      echo "Fuzz crash in $t. Reproduce and minimize: just fuzz-crash $t $crash  Then save the minimized input as a regression test and fix it in the same PR." >&2
      exit "$status"
    done

# Reproduce one crashing input, then minimize it (cargo fuzz tmin prints the minimized input's path)
[group('hardening')]
fuzz-crash target artifact: nightly
    #!/usr/bin/env bash
    set -uo pipefail
    if RUSTC_WRAPPER='' cargo +"$NIGHTLY" fuzz run --fuzz-dir harness/fuzz "{{ target }}" "{{ artifact }}"; then
      echo "{{ artifact }} no longer crashes {{ target }}; nothing to minimize" >&2; exit 0
    fi
    RUSTC_WRAPPER='' cargo +"$NIGHTLY" fuzz tmin --fuzz-dir harness/fuzz "{{ target }}" "{{ artifact }}"

# Miri on core's tests (strict provenance); tests that touch files or the network are ignored
[group('hardening')]
miri: nightly
    RUSTC_WRAPPER='' MIRIFLAGS=-Zmiri-strict-provenance PROPTEST_CASES=8 cargo +"$NIGHTLY" miri nextest run -p "{{ project }}-core" --locked

# Install Kani's verifier bundle once (cargo-kani itself comes from mise)
[private]
kani-setup:
    [ -d "$HOME/.kani/kani-$(cargo kani --version 2>/dev/null | awk '{print $NF}')" ] || cargo kani setup >&2

# Every run has a per-harness timeout: CBMC can grow without bound in memory, so a slow harness
# is a bug to fix, not to wait out.

# Kani proofs on core: `just kani` runs the quick_* harnesses (tier 1), `just kani full` every harness with higher bounds (tier 3)
[group('hardening')]
kani tier="quick": kani-setup
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ tier }}" in
      quick) RUSTC_WRAPPER='' cargo kani -p "{{ project }}-core" --harness quick_ -Z unstable-options --harness-timeout 2m ;;
      full)  RUSTC_WRAPPER='' cargo kani -p "{{ project }}-core" -Z unstable-options --harness-timeout 15m ;;
      *) echo "usage: just kani [quick|full]" >&2; exit 2 ;;
    esac

# Mutation testing on core; surviving mutants are listed in target/harness/mutants.json
[group('hardening')]
mutants:
    #!/usr/bin/env bash
    set -uo pipefail
    mkdir -p target/harness
    cargo mutants --package "{{ project }}-core" --test-tool nextest --output target/harness --no-shuffle >&2; status=$?
    jq '{check: "mutants", ok: ([.outcomes[] | select(.summary == "MissedMutant")] | length == 0), survivors: [.outcomes[] | select(.summary == "MissedMutant") | .scenario.Mutant | "\(.file):\(.span.start.line) \(.name // .function.function_name // "")"], summary: "\([.outcomes[] | select(.summary == "MissedMutant")] | length) surviving mutant(s)", details_path: "target/harness/mutants.out", repro: "just mutants"}' \
      target/harness/mutants.out/outcomes.json > target/harness/mutants.json
    [ "$status" = 3 ] && status=0   # 3 = some mutants timed out, which means the tests caught them
    exit "$status"

# AddressSanitizer on core and server tests (stable gnuasan target on x86_64 Linux, nightly elsewhere)
[group('hardening')]
asan: nightly
    #!/usr/bin/env bash
    set -euo pipefail
    host=$(just host)
    if [ "$host" = x86_64-unknown-linux-gnu ]; then
      rustup target add x86_64-unknown-linux-gnuasan >&2
      RUSTC_WRAPPER='' CARGO_TARGET_DIR=target/asan cargo nextest run --locked --target x86_64-unknown-linux-gnuasan -p "{{ project }}-core" -p "{{ project }}-server"
    else
      RUSTC_WRAPPER='' CARGO_TARGET_DIR=target/asan RUSTFLAGS=-Zsanitizer=address \
        cargo +"$NIGHTLY" nextest run --locked --target "$host" -p "{{ project }}-core" -p "{{ project }}-server"
    fi

# ThreadSanitizer on core and server tests (nightly, rebuilds std)
[group('hardening')]
tsan: nightly
    RUSTC_WRAPPER='' CARGO_TARGET_DIR=target/tsan RUSTFLAGS=-Zsanitizer=thread \
      cargo +"$NIGHTLY" nextest run --locked -Zbuild-std --target "$(just host)" -p "{{ project }}-core" -p "{{ project }}-server"

# ── Performance signals (run through `just perf`; informational) ──────────────────────────

# Optimization remarks (inlining, vectorization) for core, one JSON object per remark
[private]
remarks: nightly
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf target/harness/remarks && mkdir -p target/harness/remarks
    cargo +"$NIGHTLY" clean -q --release -p "{{ project }}-core" --target-dir target/remarks   # remarks are emitted only when core compiles
    RUSTC_WRAPPER='' CARGO_TARGET_DIR=target/remarks RUSTFLAGS="-Cremark=loop-vectorize -Cremark=inline -Zremark-dir=$PWD/target/harness/remarks -Cdebuginfo=1" \
      cargo +"$NIGHTLY" build --locked --release -p "{{ project }}-core" >&2
    cat target/harness/remarks/*.yaml 2>/dev/null | awk -v root="^(crates/|$PWD/crates/)" '
      /^--- !/ { if (file ~ root) printf "{\"file\":\"%s\",\"line\":%d,\"pass\":\"%s\",\"name\":\"%s\",\"function\":\"%s\"}\n", file, line, pass, name, fn; file=""; line=0 }
      /^Pass:/ { pass=$2 } /^Name:/ { name=$2 } /^Function:/ { fn=$2 }
      /^DebugLoc:/ { match($0, /File: [^,]*/); file=substr($0, RSTART+6, RLENGTH-6); gsub(/\x27/, "", file); wrapped=1 }
      wrapped && /Line: [0-9]+/ { match($0, /Line: [0-9]+/); line=substr($0, RSTART+6, RLENGTH-6); wrapped=0 }
      END { if (file ~ root) printf "{\"file\":\"%s\",\"line\":%d,\"pass\":\"%s\",\"name\":\"%s\",\"function\":\"%s\"}\n", file, line, pass, name, fn }' \
      > target/harness/remarks.jsonl
    echo "$(wc -l < target/harness/remarks.jsonl) remarks for workspace code in target/harness/remarks.jsonl"

# llvm-mca throughput report for each hot path (needs llvm-mca on PATH; skipped otherwise)
[private]
mca:
    #!/usr/bin/env bash
    set -uo pipefail
    mkdir -p target/harness/mca
    for dir in /opt/homebrew/opt/llvm/bin /usr/local/opt/llvm/bin /usr/lib/llvm-*/bin; do [ -x "$dir/llvm-mca" ] && PATH="$PATH:$dir"; done
    command -v llvm-mca >/dev/null || { echo "skipped: llvm-mca not found (it ships with LLVM, not rustup)"; exit 0; }
    awk '/^```text hot-paths/{f=1; next} /^```/{f=0} f && NF' docs/performance/hot-paths.md | while IFS= read -r fn; do
      cargo asm --lib -p "{{ project }}-core" --mca "$fn" > "target/harness/mca/$(tr -c 'A-Za-z0-9\n' _ <<<"$fn").txt" 2>&1
    done

# Wall-clock benchmarks (Criterion; informational)
[private]
criterion:
    cargo bench --locked -p "{{ project }}-core" --bench wall_clock -- --noplot

# Top 30 functions by generated LLVM IR lines in core (compile-time signal)
[private]
llvm-lines:
    cargo llvm-lines --release -p "{{ project }}-core" | head -n 32 | tee target/harness/llvm-lines.txt

# Build timings report (target/cargo-timings/cargo-timing.html)
[private]
timings:
    cargo build --locked --release --timings -p "{{ project }}-server"

# ── CI ────────────────────────────────────────────────────────────────────────────────────

# Pre-push subset of tier 1 (see hk.pkl)
[group('ci')]
ci-fast:
    just gates deny dylint arch coverage asm-snapshots gungraun

# Validate the GitHub workflows (wrkflw + actionlint)
[group('ci')]
ci-validate:
    wrkflw validate .github/workflows
    actionlint

# Run one workflow job locally in Docker (act); pass -s GITHUB_TOKEN=... for API steps
[group('ci')]
ci-local job *flags:
    act -j "{{ job }}" -P ubuntu-latest=catthehacker/ubuntu:act-latest {{ flags }}

# Weekly upkeep report (target/harness/gardening.md); --issue opens or updates one issue
[group('ci')]
gardening *flags:
    #!/usr/bin/env bash
    set -uo pipefail
    mkdir -p target/harness
    report=target/harness/gardening.md
    month_ago=$(( $(date +%s) - 30 * 86400 )); quarter_ago=$(( $(date +%s) - 90 * 86400 ))
    {
      echo "# Gardening report $(date -u +%F)"
      echo; echo "## Knowledge (okf validate --drift --stale)"
      okf validate docs --drift --stale 2>&1 | grep -E '^(warn|gate|error)' || echo "clean"
      echo; echo "## Concepts unverified after 30 days"
      okf search "" docs --scope project --filter "verified=null" --limit 500 --json 2>/dev/null | jq -r '.[]?.concept_id' | while read -r id; do
        created=$(git log --diff-filter=A --format=%ct -- "docs/$id.md" | tail -1)
        [ -n "$created" ] && [ "$created" -lt "$month_ago" ] && echo "- $id"
      done
      echo; echo "## Plans active for more than 30 days"
      okf search "" docs --scope project --filter "type=Plan,status=draft" --limit 500 --json 2>/dev/null | jq -r '.[]?.concept_id' | while read -r id; do
        created=$(git log --diff-filter=A --format=%ct -- "docs/$id.md" | tail -1)
        [ -n "$created" ] && [ "$created" -lt "$month_ago" ] && echo "- $id"
      done
      echo; echo "## #[expect] older than 90 days"
      git grep -n -E '^\s*#!?\[expect\(' -- '*.rs' | while IFS=: read -r file line _; do
        when=$(git blame -L "$line,$line" --porcelain -- "$file" | sed -n 's/^author-time //p')
        [ -n "$when" ] && [ "$when" -lt "$quarter_ago" ] && echo "- $file:$line"
      done
      echo; echo "## docs/generated drift"
      just docs >/dev/null 2>&1; git diff --stat -- docs/generated || true
      echo; echo "## Outdated dependencies"
      cargo outdated --workspace --root-deps-only 2>/dev/null | tail -n +1 || echo "cargo outdated failed"
    } > "$report"
    cat "$report"
    if [[ " {{ flags }} " == *" --issue "* ]]; then
      number=$(gh issue list --state open --search 'in:title "gardening report"' --json number --jq '.[0].number // empty')
      if [ -n "$number" ]; then gh issue edit "$number" --body-file "$report"; else gh issue create --title "gardening report" --body-file "$report"; fi
    fi

# Linux CI runner extras not managed by mise (Valgrind for Gungraun, Kani's bundle)
[private]
ci-deps: kani-setup
    if [ "$(uname -s)" = Linux ] && ! command -v valgrind >/dev/null; then sudo apt-get update -qq && sudo apt-get install -y -qq valgrind >&2; fi

# PR knowledge check: docs/ changes need a docs/log.md entry; lists changed concepts (comments on PR `pr`)
[private]
knowledge-pr base pr="":
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p target/harness
    [ -n "$(git diff --name-only "{{ base }}"...HEAD -- docs/)" ] || { echo '{"docs_changed": false}'; exit 0; }
    if ! git diff "{{ base }}"...HEAD -- docs/log.md | grep -qE '^\+\* '; then
      echo "docs/ changed without a log.md entry; run okf update or add one" >&2; exit 1
    fi
    body=target/harness/knowledge-comment.md
    {
      echo "### Knowledge changes"
      git diff --name-only "{{ base }}"...HEAD -- 'docs/*.md' | grep -v -e '/index.md$' -e '^docs/log.md$' | while read -r file; do
        id=${file#docs/}; id=${id%.md}
        okf show "$id" docs --json 2>/dev/null | jq -r '"- **\(.title)** (`\(.id)`): \(.description)"' || echo "- \`$id\` (removed)"
      done
    } > "$body"
    if [ -n "{{ pr }}" ] && command -v gh >/dev/null; then gh pr comment "{{ pr }}" --body-file "$body"; else cat "$body"; fi

# Open or update one issue for a failed nightly check, with the harness JSON and repro lines
[private]
report-failure name:
    #!/usr/bin/env bash
    set -euo pipefail
    title="hardening: {{ name }} failed"
    body=$(mktemp)
    {
      echo "Nightly \`just {{ name }}\` failed on $(git rev-parse --short HEAD) ($(date -u +%F))."
      echo; echo '```json'; jq -s . target/harness/*.json 2>/dev/null || echo '[]'; echo '```'
      grep -rhs 'Reproduce:' target/harness harness/dst/target | sort -u | head -5
      echo; echo "Rerun: just {{ name }}"
    } > "$body"
    number=$(gh issue list --state open --search "in:title \"$title\"" --json number --jq '.[0].number // empty')
    if [ -n "$number" ]; then gh issue comment "$number" --body-file "$body"; else gh issue create --title "$title" --body-file "$body"; fi

# Template-repository recipes (template-ci, ...); the file is not part of generated projects.
import? 'template.just'
