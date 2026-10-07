---
type: Guide
title: Git hooks
description: What the hk git hooks run on commit and push, the agent Stop hooks, why hooks cannot be skipped, and what to do when one fails.
tags: [workflow, hooks, hk, pre-commit, pre-push, agents]
status: stable
code_refs: [hk.pkl, justfile, .claude/settings.json, .codex/hooks.json]
---

# Git hooks

hk runs the hooks, configured in `hk.pkl`. `just bootstrap` installs them with `hk install --mise`,
so they find mise's tools without an activated shell, and every worktree shares them. `just check`
and `just up` stop with `Git hooks not installed. Run: just bootstrap` when they are missing.

| Hook | Steps |
| --- | --- |
| pre-commit | cargo fmt, taplo, clippy, typos, gitleaks on the staged changes, machete, offline cargo-deny, actionlint, the justfile lint, `just knowledge`, the protected-files warning, the tests of the affected packages, and the panic audit when core changes |
| pre-push | `just ci-fast`: cargo-deny with advisories, dylint, architecture tests, coverage gate, asm snapshots, Gungraun (Linux) |
| check, fix | the formatting and lint steps only (no tests, no panic audit); the agent Stop hooks use these |

A step runs only when a staged file matches its glob; `just knowledge` runs on every commit
because concepts point at code through `code_refs`. pre-commit fixes formatting and restages it,
and stashes unstaged changes so work in progress never lands in a commit. A one-file commit
should take under 30 seconds warm; when it doesn't, move a step to pre-push rather than loosening it.

## Agents

- The Claude Code and Codex Stop hooks run `hk agent stop-hook`: hk's check steps on the files
  the agent changed. The agent cannot finish until they pass.
- Agents never bypass hooks. Claude Code's settings deny `--no-verify`, `HK=0`, `HK_SKIP_*`,
  `hk uninstall` and `git config hk.*`. Those are prefix rules, so `just ci` also runs
  `hk check --all` as the backstop for any commit that skipped the hooks.

## When a hook fails

Each step names the rule and the fix; formatters fix and restage on their own. The
protected-files step only warns: it lists threshold and policy files in the commit so a human
reviews them ([lint policy](/conventions/lints.md)). A pre-push failure writes
`target/harness/<check>.json` like any check ([agent workflow](/conventions/agent-workflow.md)).
If a hook itself is wrong, say so and stop; never work around it. CI runs the same steps:
[CI workflows](/workflows/ci.md).
