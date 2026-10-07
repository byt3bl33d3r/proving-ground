# Recipes for the template repository itself (imported by the justfile with `import?`;
# cargo-generate drops this file, so generated projects never see these recipes).

# Generate a project and run the template's acceptance steps on it (what template-ci.yml runs).
# LICENSE picks the license choice; KEEP=1 keeps the generated project for inspection.
template-ci:
    #!/usr/bin/env bash
    set -euo pipefail
    # This repo's mise.toml points hk at template.hk.pkl; the generated project must use its own
    # hk.pkl, and mise does not unset variables inherited from a parent directory's config.
    unset HK_FILE
    work=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/template-ci.XXXXXX")
    report="{{ root }}/target/harness/template-ci.json"; mkdir -p "$(dirname "$report")"
    [ "${KEEP:-0}" = 1 ] || trap 'cd /; (cd "$work/demo-app-2" 2>/dev/null && just down >/dev/null 2>&1); (cd "$work/demo-app" 2>/dev/null && just down >/dev/null 2>&1); rm -rf "$work"' EXIT
    step() { echo "── $*" >&2; }
    # Prints the seconds a command took; callers add `|| exit 1` because set -e does not reach
    # into command substitutions.
    timed() { local start; start=$(date +%s); "$@" >&2 || return 1; echo $(( $(date +%s) - start )); }

    step "1. generate demo-app (license: ${LICENSE:-MIT OR Apache-2.0})"
    cargo generate --path "{{ root }}" --name demo-app --destination "$work" --silent \
      --define description="A Rust service" --define gh_owner=acme --define license="${LICENSE:-MIT OR Apache-2.0}" >&2
    cd "$work/demo-app"

    step "2. no leftover Liquid in templated files"
    leftovers=$(grep -rnI -e '{{{{' -e '{%' . --exclude-dir=.git --exclude-dir=target \
      | grep -vE '^(\./)?(justfile|hk\.pkl|\.github/workflows/|harness/stack/|harness/lints/|harness/fuzz/corpus/)' \
      | grep -v -e '{{{{files}}' -e '${{{{ x }}' || true)   # both appear verbatim in docs/template/notes.md
    [ -z "$leftovers" ] || { echo "Leftover template syntax:" >&2; echo "$leftovers" >&2; exit 1; }

    if [ "${REPRODUCE:-0}" = 1 ]; then
      step "3. reproduction: generated tree equals the pre-template build (tag pre-template)"
      ref="$work/reference"; mkdir -p "$ref"; git -C "{{ root }}" archive pre-template | tar -x -C "$ref"
      rm -f "$ref/Cargo.lock" "$ref/harness/dst/Cargo.lock" "$ref/harness/fuzz/Cargo.lock" "$ref/SPEC.md"
      # Expected differences: README.md is new, CODEOWNERS takes the owner placeholder, and the
      # justfile (copied verbatim, so no placeholders) gained the template.just import.
      diff -r -x .git -x target -x README.md -x CODEOWNERS -x justfile "$ref" . >&2 \
        || { echo "Generated project differs from the pre-template tag" >&2; exit 1; }
    fi

    step "4. first commit, tools, bootstrap"
    git init -q 2>/dev/null || true
    git add -A && git -c user.name=template-ci -c user.email=template-ci@localhost commit -qm init
    mise trust -q && mise install -q >&2
    eval "$(mise env -s bash)"
    just bootstrap >&2
    hk validate 2>&1 | grep -q '/demo-app/hk.pkl is valid' || { echo "hk is not using the generated project's hk.pkl" >&2; exit 1; }
    git add -A && git -c user.name=template-ci -c user.email=template-ci@localhost commit -qm "lockfiles"

    step "5. tiers and harness"
    cold_check=$(timed just check) || exit 1
    warm_check=$(timed just check) || exit 1
    just ci >&2
    # okf smoke test: a broken index or a renamed concept changes the top hit.
    top=$(okf search "port allocation" docs --scope project --json | jq -r '.[0].concept_id // empty')
    [ "$top" = architecture/port-allocation ] \
      || { echo "okf search \"port allocation\" ranked '$top' first, expected architecture/port-allocation" >&2; exit 1; }
    just up >&2
    just e2e >&2
    just down >&2

    step "6. isolation: two worktrees at once"
    git worktree add -q ../demo-app-2
    (cd ../demo-app-2 && mise trust -q && just up >/dev/null)
    just up >/dev/null
    first=$(jq -c .ports .harness/stack.json); second=$(jq -c .ports ../demo-app-2/.harness/stack.json)
    [ "$first" != "$second" ] || { echo "Both worktrees got ports $first" >&2; exit 1; }
    (cd ../demo-app-2 && just down) && just down

    step "7. workflows and hook timings"
    actionlint >&2
    echo '// touch' >> crates/demo-app-core/src/fields.rs
    commit_secs=$(timed git -c user.name=template-ci -c user.email=template-ci@localhost commit -qam "one-file change") || exit 1
    dylint_warm=$(timed just dylint) || exit 1

    jq -n --arg cold "$cold_check" --arg warm "$warm_check" --arg commit "$commit_secs" --arg dylint "$dylint_warm" \
      '{check: "template-ci", ok: true, summary: "generated project passed bootstrap, ci, e2e and isolation",
        timings_seconds: {check_cold: ($cold|tonumber), check_warm: ($warm|tonumber), one_file_commit: ($commit|tonumber), dylint_warm: ($dylint|tonumber)},
        details_path: null, repro: "just template-ci"}' | tee "$report"
    [ -z "${GITHUB_STEP_SUMMARY:-}" ] || jq -r '"### template-ci timings (s)\n" + (.timings_seconds | to_entries | map("- \(.key): \(.value)") | join("\n"))' "$report" >> "$GITHUB_STEP_SUMMARY"
