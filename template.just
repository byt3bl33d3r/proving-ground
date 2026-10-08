# Recipes for the template repository itself (imported by the justfile with `import?`;
# cargo-generate drops this file, so generated projects never see these recipes).

# What template-ci.yml runs. REPRODUCE=1 also diffs the generated tree against the pre-template tag.

# template-ci.yml generates first so the Rust cache step can see the generated workspaces.
# Generate demo-app into $TEMPLATE_CI_DIR (LICENSE=<choice>)
[group('template')]
template-generate:
    #!/usr/bin/env bash
    set -euo pipefail
    dir="${TEMPLATE_CI_DIR:?set TEMPLATE_CI_DIR to the directory to generate demo-app into}"
    mkdir -p "$dir"
    cargo generate --path "{{ root }}" --name demo-app --destination "$dir" --silent \
      --define description="A Rust service" --define gh_owner=acme --define license="${LICENSE:-MIT OR Apache-2.0}" >&2

# Cargo.toml's license field is the only output that depends on the license choice.
# Render every license choice and check Cargo.toml's license field
[group('template')]
template-licenses:
    #!/usr/bin/env bash
    set -euo pipefail
    work=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/template-licenses.XXXXXX"); trap 'rm -rf "$work"' EXIT
    taplo get -f "{{ root }}/cargo-generate.toml" placeholders.license.choices | while IFS= read -r license; do
      expected=$license; [ "$license" != Proprietary ] || expected=LicenseRef-Proprietary
      dir="$work/$(printf %s "$license" | tr -c 'A-Za-z0-9' -)"; mkdir -p "$dir"
      out=$(cargo generate --path "{{ root }}" --name demo-app --destination "$dir" --silent --vcs none \
        --define description="A Rust service" --define gh_owner=acme --define license="$license" 2>&1) \
        || { echo "$out" >&2; exit 1; }
      actual=$(sed -n 's/^license = "\(.*\)"$/\1/p' "$dir/demo-app/Cargo.toml")
      [ "$actual" = "$expected" ] || { echo "license \"$license\" rendered Cargo.toml license = \"$actual\", expected \"$expected\"" >&2; exit 1; }
      echo "ok    $license" >&2
    done

# TEMPLATE_CI_DIR=<dir> runs on the demo-app that template-generate just put there and keeps the dir.
# Generate a project and run the template's acceptance steps on it (LICENSE=<choice>; KEEP=1 keeps the project)
[group('template')]
template-ci:
    #!/usr/bin/env bash
    set -euo pipefail
    # This repo's mise.toml points hk at template.hk.pkl; the generated project must use its own
    # hk.pkl, and mise does not unset variables inherited from a parent directory's config.
    unset HK_FILE
    work=${TEMPLATE_CI_DIR:-$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/template-ci.XXXXXX")}
    report="{{ root }}/target/harness/template-ci.json"; mkdir -p "$(dirname "$report")"
    # A fixed TEMPLATE_CI_DIR outlives the run: template-ci.yml saves the Rust cache from it afterwards.
    [ "${KEEP:-0}" = 1 ] || trap 'cd /; (cd "$work/demo-app-2" 2>/dev/null && just down >/dev/null 2>&1); (cd "$work/demo-app" 2>/dev/null && just down >/dev/null 2>&1); [ -n "${TEMPLATE_CI_DIR:-}" ] || rm -rf "$work"' EXIT
    step() { echo "── $*" >&2; }
    # Prints the seconds a command took; callers add `|| exit 1` because set -e does not reach
    # into command substitutions.
    timed() { local start; start=$(date +%s); "$@" >&2 || return 1; echo $(( $(date +%s) - start )); }

    step "1. generate demo-app (license: ${LICENSE:-MIT OR Apache-2.0})"
    if [ -d "$work/demo-app" ]; then
      git -C "$work/demo-app" rev-parse -q --verify HEAD >/dev/null 2>&1 \
        && { echo "$work/demo-app has commits from an earlier run; use a fresh TEMPLATE_CI_DIR" >&2; exit 1; }
      echo "using $work/demo-app from just template-generate" >&2
    else
      TEMPLATE_CI_DIR="$work" just template-generate
    fi
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
    # In CI the Rust cache step has already resolved some lockfiles, so this commit can be empty.
    git add -A && git -c user.name=template-ci -c user.email=template-ci@localhost commit -qm "lockfiles" --allow-empty

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
