# Every task in this repo is a recipe here. Hooks (hk.pkl), CI (.github/workflows) and agents
# call the same recipes, so they never disagree. Run `just --list` for the menu.
#
# This file is copied verbatim by cargo-generate (it uses {{ }} natively): it must not contain
# template placeholders. The project name comes from PROJECT_NAME in mise.toml.

set shell := ["bash", "-euo", "pipefail", "-c"]
set quiet

project := env("PROJECT_NAME")
root    := justfile_directory()

# Separate Cargo workspaces (Section 2 of the template spec): fmt and clippy run in each one.
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

# ── Tier 0 ─────────────────────────────────────────────────────────────────────────────────

# Tier 0: fmt, clippy, tests, machete in every workspace (also runs on commit)
check:
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
