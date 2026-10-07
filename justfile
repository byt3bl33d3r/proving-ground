# Every task in this repo is a recipe here. Hooks (hk.pkl), CI (.github/workflows) and agents
# call the same recipes, so they never disagree. Run `just --list` for the menu.
#
# This file is copied verbatim by cargo-generate (it uses {{ }} natively): it must not contain
# template placeholders. The project name comes from PROJECT_NAME in mise.toml.

set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-path := ".env.local"   # optional per-worktree overrides (HARNESS_PORT); never committed
set dotenv-override               # .env.local beats inherited values
set quiet                         # no command echo; stdio MCP servers need a clean stdout

root    := justfile_directory()
# PROJECT_NAME comes from mise.toml [env]; read the file directly when mise is not active
# (MCP clients and git hooks may start `just` without it).
project := env("PROJECT_NAME", shell('sed -n "s/^PROJECT_NAME *= *\"\\(.*\\)\"$/\\1/p" "$1/mise.toml"', root))
service := env("OTEL_SERVICE_NAME", project)

# ── Harness variables: one 10-port range per git worktree ──────────────────────────────────
# The range is derived from a hash of this worktree's path (20000-29990, step 10), so every
# worktree, agent and MCP config computes the same ports with no registry. Override a
# collision with HARNESS_PORT=<base> in .env.local. See docs/architecture/port-allocation.md.
path_crc := shell('printf %s "$1" | cksum | cut -d" " -f1', root)
wt       := shell('basename "$1" | tr "[:upper:]" "[:lower:]" | tr -c "a-z0-9\n" "-"', root) + "-" + shell('printf %06x $(($1 % 16777216))', path_crc)
base     := env("HARNESS_PORT", shell('echo $((20000 + ($1 % 1000) * 10))', path_crc))

export COMPOSE_PROJECT_NAME := project + "-" + wt
export VM_PORT := base
export VL_PORT := shell('echo $(($1 + 1))', base)
export VT_PORT := shell('echo $(($1 + 2))', base)
export OTEL_EXPORTER_OTLP_METRICS_ENDPOINT := "http://127.0.0.1:" + VM_PORT + "/opentelemetry/v1/metrics"
export OTEL_EXPORTER_OTLP_LOGS_ENDPOINT    := "http://127.0.0.1:" + VL_PORT + "/insert/opentelemetry/v1/logs"
export OTEL_EXPORTER_OTLP_TRACES_ENDPOINT  := "http://127.0.0.1:" + VT_PORT + "/insert/opentelemetry/v1/traces"
export OTEL_EXPORTER_OTLP_PROTOCOL := "http/protobuf"
export OTEL_RESOURCE_ATTRIBUTES := "worktree=" + wt
export OTEL_METRIC_EXPORT_INTERVAL := "5000"
export APP_LOG_JSON := root + "/.harness/logs/app.jsonl"

compose := "docker compose -f harness/stack/compose.yaml"

# Separate Cargo workspaces (harness/dst, harness/fuzz, harness/lints): fmt and clippy run in each.
workspaces := ". harness/dst harness/fuzz harness/lints"

# List recipes
default:
    just --list

# ── Gate plumbing ──────────────────────────────────────────────────────────────────────────

# Run a command as a named check: log to target/harness/<check>.log, write
# target/harness/<check>.json {check, ok, summary, details_path, repro}, print a one-line verdict.
# VERBOSE=1 streams the output instead of capturing it.
[private]
[positional-arguments]
gate check *cmd:
    #!/usr/bin/env bash
    set -uo pipefail
    check="$1"; shift
    mkdir -p "{{root}}/target/harness"
    log="target/harness/$check.log"
    json="target/harness/$check.json"
    repro="$*"
    rm -f "{{root}}/$json"
    start=$(date +%s)
    if [ "${VERBOSE:-0}" = 1 ]; then "$@" 2>&1 | tee "{{root}}/$log"; status=${PIPESTATUS[0]}
    else "$@" >"{{root}}/$log" 2>&1; status=$?; fi
    secs=$(( $(date +%s) - start ))
    if [ "$status" = 0 ]; then ok=true; summary="passed in ${secs}s"
    else
      ok=false
      summary=$(grep -E '^(error|FAIL|failed|Error|warning: unused)' "{{root}}/$log" | head -n 1 | cut -c1-200 || true)
      [ -n "$summary" ] || summary="exit status $status after ${secs}s"
    fi
    # A check may write its own richer JSON; only fill in what is missing.
    if jq -e --arg c "$check" '.check == $c and has("ok")' "{{root}}/$json" >/dev/null 2>&1; then
      jq --argjson ok "$ok" '.ok = (.ok and $ok)' "{{root}}/$json" > "{{root}}/$json.tmp" && mv "{{root}}/$json.tmp" "{{root}}/$json"
    else
      jq -n --arg check "$check" --argjson ok "$ok" --arg summary "$summary" \
        --arg details "$log" --arg repro "$repro" \
        '{check: $check, ok: $ok, summary: $summary, details_path: $details, repro: $repro}' > "{{root}}/$json"
    fi
    if [ "$ok" = true ]; then printf 'ok    %-14s %s\n' "$check" "$summary"
    else
      printf 'FAIL  %-14s %s\n' "$check" "$summary"
      [ "${VERBOSE:-0}" = 1 ] || tail -n 60 "{{root}}/$log" | sed 's/^/      /'
      printf '      details: %s  repro: %s\n' "$log" "$repro"
    fi
    exit "$status"

# Run several gates, keep going after failures, exit non-zero if any failed.
[private]
[positional-arguments]
gates *names:
    #!/usr/bin/env bash
    set -uo pipefail
    failed=()
    for name in "$@"; do
      just gate "$name" just "$name" 2>/dev/null || failed+=("$name")
    done
    if [ ${#failed[@]} -gt 0 ]; then echo "FAILED: ${failed[*]}" >&2; exit 1; fi

# ── Setup ──────────────────────────────────────────────────────────────────────────────────

# One-time setup per clone: lockfile, git hooks, tool checks, first build
bootstrap:
    #!/usr/bin/env bash
    set -euo pipefail
    for tool in cargo docker hk jq cargo-nextest; do
      command -v "$tool" >/dev/null || { echo "Missing $tool. Run: mise install (and install Docker)" >&2; exit 1; }
    done
    docker info >/dev/null 2>&1 || echo "warning: Docker is not running; just up needs it" >&2
    [ -f Cargo.lock ] || cargo generate-lockfile
    hk install --mise
    cargo build --workspace --all-targets --locked
    echo '{"bootstrap": "ok", "next": ["just up", "just check"]}'

# Fail fast when the git hooks are not installed (skipped in CI, which runs `hk check --all`)
[private]
[no-exit-message]
hooks-installed:
    if [ "${CI:-}" != true ] && ! git hook list pre-commit 2>/dev/null | grep -qx hk-pre-commit \
      && ! grep -qs 'hk run pre-commit' "$(git rev-parse --git-common-dir)/hooks/pre-commit"; then \
      echo "Git hooks not installed. Run: just bootstrap" >&2; exit 1; fi

# ── Tier 0 ─────────────────────────────────────────────────────────────────────────────────

# Tier 0: fmt, clippy, tests, machete in every workspace (also runs on commit)
check: hooks-installed
    just gates fmt-check clippy test machete

# Format every workspace
fmt:
    for ws in {{workspaces}}; do [ -f "$ws/Cargo.toml" ] || continue; (cd "$ws" && cargo fmt --all); done

# Check formatting in every workspace
fmt-check:
    for ws in {{workspaces}}; do [ -f "$ws/Cargo.toml" ] || continue; (cd "$ws" && cargo fmt --all --check); done

# Clippy with -D warnings in every workspace
clippy:
    for ws in {{workspaces}}; do [ -f "$ws/Cargo.toml" ] || continue; (cd "$ws" && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings); done

# Unit, property, snapshot, transcript and architecture tests (not e2e)
test *args:
    cargo nextest run --workspace --locked --no-tests=warn -E 'not binary(e2e)' {{args}}

# Unused dependencies
machete:
    cargo machete

# ── Hook steps (see hk.pkl) ────────────────────────────────────────────────────────────────

# Warn when a change touches files that hold thresholds, baselines or lint policy
[no-exit-message]
protected-files *files:
    #!/usr/bin/env bash
    set -uo pipefail
    [ -n "{{files}}" ] || exit 0
    echo "Protected files changed: {{files}}" >&2
    echo "These hold thresholds, baselines or lint policy (Cargo.toml: [workspace.lints]; mise.toml: COV_MIN_REGIONS)." >&2
    echo "Ask a human before weakening any of them; never lower a threshold or baseline to make a check pass." >&2
    exit 1

# Scan files for secrets (gitleaks); the pre-commit hook scans the staged diff instead
secrets *files:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
    for f in {{files}}; do [ -f "$f" ] && mkdir -p "$tmp/$(dirname "$f")" && cp "$f" "$tmp/$f"; done
    gitleaks dir --no-banner --redact --log-level warn "$tmp"

# Run the tests of the packages that own the given files (all packages if core or the root manifest changed)
affected-tests *files:
    #!/usr/bin/env bash
    set -euo pipefail
    all=false; pkgs=()
    meta=$(cargo metadata --format-version 1 --no-deps --locked)
    for f in {{files}}; do
      case "$f" in Cargo.toml|Cargo.lock|crates/*-core/*) all=true ;; esac
      dir=$(dirname "$f")
      while [ "$dir" != . ] && [ ! -f "$dir/Cargo.toml" ]; do dir=$(dirname "$dir"); done
      name=$(jq -r --arg m "$PWD/$dir/Cargo.toml" '.packages[] | select(.manifest_path == $m) | .name' <<<"$meta")
      [ -n "$name" ] && pkgs+=(-p "$name")
    done
    if $all; then just test; elif [ ${#pkgs[@]} -gt 0 ]; then cargo nextest run --locked --no-tests=warn -E 'not binary(e2e)' "${pkgs[@]}"; fi

# Functions in core's release build that can panic, compared with docs/generated/panic-allowlist.txt
panic-audit:
    #!/usr/bin/env bash
    set -uo pipefail
    mkdir -p target/harness
    crate=$(tr - _ <<<"{{project}}")_core
    regex='core::panicking::|std::panicking::begin_panic|unwrap_failed|expect_failed|index_len_fail|index_order_fail|slice_error_fail|str_index_overflow_fail'
    cargo asm --lib -p "{{project}}-core" --llvm -s --json --callers-of "$regex" 1 2>target/harness/panic-audit.log \
      | jq -r '.[].name' | grep "${crate}::" | grep -v '^core::ptr::drop_glue' | sort -u > target/harness/panic-callers.txt
    new=$(grep -v -e '^#' -e '^$' docs/generated/panic-allowlist.txt | sort -u | comm -23 target/harness/panic-callers.txt -)
    count=$(grep -c . <<<"$new" || true)
    jq -n --arg new "$new" --argjson count "$count" '{check: "panic-audit", ok: ($count == 0), summary: (if $count == 0 then "no new panic paths" else "\($count) new panic path(s)" end), new_paths: ($new | split("\n") | map(select(. != ""))), details_path: "target/harness/panic-callers.txt", repro: "just panic-audit"}' > target/harness/panic-audit.json
    [ "$count" = 0 ] && exit 0
    while IFS= read -r fn; do
      echo "New panic path in $fn. Remove it (iterators, hoisted assert, checked ops) or add it to the allowlist with a reason in the PR." >&2
    done <<<"$new"
    exit 1

# Pre-push subset of tier 1 (see hk.pkl)
ci-fast:
    just gates deny

# All cargo-deny checks (advisories need the network)
deny:
    cargo deny --locked check

# ── Per-worktree harness ───────────────────────────────────────────────────────────────────

# Print harness variables for a shell: eval "$(just env)"
env:
    env | grep -E '^(COMPOSE_PROJECT_NAME|VM_PORT|VL_PORT|VT_PORT|OTEL_[A-Z_]+|APP_LOG_JSON)=' | sort | sed 's/^/export /'

# Start this worktree's telemetry stack and server; prints URLs as JSON
up: hooks-installed check-ports
    mkdir -p .harness/logs
    {{compose}} up -d --quiet-pull >&2
    just wait-stack
    just stack-json > .harness/stack.json
    just env > .harness/env
    just start-server
    just wait-ready
    jq -n --slurpfile app .harness/app.json --slurpfile stack .harness/stack.json \
      '{app: $app[0].url, ui: $stack[0].ui, ports: $stack[0].ports, logs: "just logs-errors", down: "just down"}'

# Stop the server and remove the stack, its volumes and .harness/
down:
    just stop-server
    {{compose}} down -v --remove-orphans >&2 2>/dev/null || true
    rm -rf .harness

# Rebuild and restart only the server (keeps the stack and its data)
restart:
    just stop-server
    just start-server
    just wait-ready
    jq -c '{app: .url, pid}' .harness/app.json

# JSON: stack health, server process, readiness, ports
status:
    #!/usr/bin/env bash
    set -uo pipefail
    health() { curl -fsS -m 2 "http://127.0.0.1:$1/health" >/dev/null 2>&1 && echo true || echo false; }
    pid=$(cat .harness/server.pid 2>/dev/null || echo "")
    alive=false; [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null && alive=true
    url=$(jq -r .url .harness/app.json 2>/dev/null || echo "")
    ready=false; [ -n "$url" ] && curl -fsS -m 2 "$url/readyz" >/dev/null 2>&1 && ready=true
    jq -n --arg wt "{{wt}}" --argjson base {{base}} --arg url "$url" --arg pid "$pid" \
      --argjson vm "$(health $VM_PORT)" --argjson vl "$(health $VL_PORT)" --argjson vt "$(health $VT_PORT)" \
      --argjson alive "$alive" --argjson ready "$ready" \
      '{worktree: $wt, ports: {range: [$base, $base + 9], metrics: $base, logs: ($base + 1), traces: ($base + 2)},
        stack: {victoria_metrics: $vm, victoria_logs: $vl, victoria_traces: $vt},
        server: {url: (if $url == "" then null else $url end), pid: (if $pid == "" then null else ($pid | tonumber) end), alive: $alive, ready: $ready}}'

# End-to-end tests against `just up`, then the latency and error budgets
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

# Print the three Victoria UI URLs
ui:
    echo "VictoriaMetrics  http://127.0.0.1:$VM_PORT/vmui/"
    echo "VictoriaLogs     http://127.0.0.1:$VL_PORT/select/vmui/"
    echo "VictoriaTraces   http://127.0.0.1:$VT_PORT/select/vmui/"

# Remove stacks of this project whose worktree directory no longer exists
harness-gc:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose ls --all --format json | jq -r --arg p "{{project}}-" \
      '.[] | select(.Name | startswith($p)) | "\(.Name)\t\(.ConfigFiles)"' |
    while IFS=$'\t' read -r name files; do
      if [ ! -e "${files%%,*}" ]; then
        docker compose -p "$name" down -v --remove-orphans >&2 && echo "{\"removed\":\"$name\"}"
      fi
    done

# Start a Victoria MCP server for this worktree (called by .mcp.json and .codex/config.toml)
[no-exit-message]
mcp kind:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{kind}}" in
      metrics) MCP_LOG_LEVEL=warn VM_INSTANCE_ENTRYPOINT="http://127.0.0.1:$VM_PORT" VM_INSTANCE_TYPE=single exec mcp-victoriametrics ;;
      logs)    MCP_LOG_LEVEL=warn VL_INSTANCE_ENTRYPOINT="http://127.0.0.1:$VL_PORT" exec mcp-victorialogs ;;
      traces)  MCP_LOG_LEVEL=warn VT_INSTANCE_ENTRYPOINT="http://127.0.0.1:$VT_PORT" exec mcp-victoriatraces ;;
      *) echo "usage: just mcp metrics|logs|traces" >&2; exit 2 ;;
    esac

# Fail if this worktree's ports are taken by something other than its own stack
[private]
[no-exit-message]
check-ports:
    #!/usr/bin/env bash
    set -uo pipefail
    ours=$({{compose}} ps --format '{{{{.Publishers}}' 2>/dev/null || true)
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
      echo "Victoria service on port $p not healthy after 30s. Run: {{compose}} logs" >&2; exit 1
    done

[private]
stack-json:
    #!/usr/bin/env bash
    jq -n --argjson base {{base}} --arg wt "{{wt}}" --arg project "$COMPOSE_PROJECT_NAME" \
      --arg m "$OTEL_EXPORTER_OTLP_METRICS_ENDPOINT" --arg l "$OTEL_EXPORTER_OTLP_LOGS_ENDPOINT" --arg t "$OTEL_EXPORTER_OTLP_TRACES_ENDPOINT" \
      '{worktree: $wt, compose_project: $project,
        ports: {range: [$base, $base + 9], metrics: $base, logs: ($base + 1), traces: ($base + 2)},
        ui: {metrics: "http://127.0.0.1:\($base)/vmui/", logs: "http://127.0.0.1:\($base + 1)/select/vmui/", traces: "http://127.0.0.1:\($base + 2)/select/vmui/"},
        otlp: {metrics: $m, logs: $l, traces: $t}}'

[private]
start-server:
    cargo build -q -p "{{project}}-server"
    rm -f .harness/app.json
    nohup "target/debug/{{project}}-server" > .harness/logs/server.out 2>&1 & echo $! > .harness/server.pid

[private]
stop-server:
    #!/usr/bin/env bash
    set -uo pipefail
    pid=$(cat .harness/server.pid 2>/dev/null) || exit 0
    kill -TERM "$pid" 2>/dev/null || exit 0
    for _ in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || exit 0; sleep 0.2; done
    kill -KILL "$pid" 2>/dev/null || true

[private]
[no-exit-message]
wait-ready:
    #!/usr/bin/env bash
    set -uo pipefail
    for _ in $(seq 1 120); do
      if [ -f .harness/app.json ] && curl -fsS -m 1 "$(jq -r .url .harness/app.json)/readyz" >/dev/null 2>&1; then exit 0; fi
      sleep 0.25
    done
    echo "Server not ready after 30s. Last 40 lines of .harness/logs/server.out:" >&2
    tail -n 40 .harness/logs/server.out >&2 || true
    just logs-errors >&2 || true
    exit 1

# ── Seeing what the app does (capped output; aggregate, never dump) ───────────────────────

# Last 50 ERROR/WARN lines from the local JSON log, compact
logs-errors:
    [ -f .harness/logs/app.jsonl ] || { echo "No log file yet: run just up" >&2; exit 1; }
    jq -c 'select(.level == "ERROR" or .level == "WARN") | {ts: .timestamp, level, msg: .fields.message, target, request_id: ([.spans[]?.request_id] | map(select(.)) | first), fields: (.fields | del(.message))}' .harness/logs/app.jsonl | tail -n 50

# Every local log line for one request id (max 200)
logs-request request_id:
    jq -c --arg id "{{request_id}}" 'select([.span.request_id?, .spans[]?.request_id] | index($id)) | {ts: .timestamp, level, msg: .fields.message, target, fields: (.fields | del(.message))}' .harness/logs/app.jsonl | head -n 200

# VictoriaLogs LogsQL query, newline JSON, max 200 lines
q-logs query:
    curl -fsS "http://127.0.0.1:$VL_PORT/select/logsql/query" --data-urlencode 'query={{query}}' -d limit=200

# VictoriaMetrics PromQL/MetricsQL instant query, result vector as JSON
q-metrics query:
    curl -fsS "http://127.0.0.1:$VM_PORT/api/v1/query" --data-urlencode 'query={{query}}' | jq -c '.data.result'

# VictoriaTraces span search (LogsQL), newline JSON, max 200 lines
q-traces query:
    curl -fsS "http://127.0.0.1:$VT_PORT/select/logsql/query" --data-urlencode 'query={{query}}' -d limit=200

# One trace via the Jaeger API, one span per line: name, duration_ms, parent, status
trace trace_id:
    curl -fsS "http://127.0.0.1:$VT_PORT/select/jaeger/api/traces/{{trace_id}}" | jq -c '.data[0] as $t | $t.spans | sort_by(.startTime)[] | {name: .operationName, span_id: .spanID, parent: ((.references // []) | map(select(.refType == "CHILD_OF")) | .[0].spanID // null), duration_ms: (.duration / 1000), status: ((.tags | map(select(.key == "error")) | .[0].value) // "unset"), service: $t.processes[.processID].serviceName}' | head -n 500

# Run every budget in docs/observability/budgets.md; pass/fail JSON (target/harness/budgets.json)
budgets:
    #!/usr/bin/env bash
    set -uo pipefail
    mkdir -p target/harness
    start=$(cat .harness/e2e.start 2>/dev/null || date -u -v-15M +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -d '-15 min' +%Y-%m-%dT%H:%M:%SZ)
    end=$(cat .harness/e2e.end 2>/dev/null || date -u +%Y-%m-%dT%H:%M:%SZ)
    awk '/^```jsonl budgets/{f=1; next} /^```/{f=0} f' docs/observability/budgets.md > target/harness/budgets.jsonl
    results=()
    while IFS= read -r budget; do
      query=$(jq -r .query <<<"$budget" | sed -e "s|\$SERVICE|{{service}}|g" -e "s|\$START|$start|g" -e "s|\$END|$end|g")
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
