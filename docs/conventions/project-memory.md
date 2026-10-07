---
type: Guide
title: Project memory
description: How agents recall and record project knowledge in docs/ with okf: search first, update before create, links and code_refs, plans, decisions and verification.
tags: [agents, okf, memory, knowledge, plans, decisions]
status: stable
code_refs: [docs/index.md, .mcp.json, .codex/config.toml]
---

# Project memory

`docs/` is an OKF v0.2 bundle managed with okf: every file is a concept with frontmatter, each
directory has an `index.md`, and `docs/log.md` records changes by date. Load only what a task
needs: search, then open the hits. Agents use the okf-memory MCP server (`okf_search`, `okf_show`,
`okf_create`, `okf_update`, `okf_relate`, `okf_validate`) or the `okf` CLI shown below. The
commands an agent runs day to day are in the [agent workflow](/conventions/agent-workflow.md).

## Recall

- Search before you change code, add a dependency or ask a question:
  `okf search "<topic>" docs --scope project` (MCP: `okf_search` with `query` and `scope: project`).
- Before editing a file for the first time, find the concepts that govern it:
  `okf search "" docs --scope project --for-path <path>` (MCP: `for_path`).
- Open a hit with `okf show <id> docs`. Don't read the bundle with grep or find.

## Record

- Update the concept that already covers what you learned: `okf update <id> docs --desc "..."`
  (`--body` replaces the whole body). Create one only when nothing covers it:
  `okf create <dir>/<slug> docs --type Guide --title "..." --desc "..." --body "..."`.
- The bundle path goes right after the id: `okf create plans/x --type Plan docs` silently writes
  into the current directory instead.
- Link every new concept from a related one, in the body (a Markdown link to its bundle path,
  such as `/testing/dst.md`) or with `okf relate <from> <to> docs --desc "..."`: an unlinked
  concept fails validation as an orphan.
  Never link to an `index.md` or to files outside docs/.
- List the repo files a concept governs in `code_refs:` (a hand edit); validation fails when one
  disappears. Paths in inline code are not checked.
- `okf create` and `okf update` maintain the parent `index.md` and add a dated `docs/log.md`
  entry; hand edits need both. A PR that changes docs/ without a new log entry fails CI.

## Plans and decisions

- Multi-step work is a Plan in docs/plans/: `okf create plans/<slug> docs --type Plan --status
  draft --tags plan,active --desc "..."`; when done, `okf update plans/<slug> docs --status stable
  --tags plan,completed`. okf accepts only draft, stable and deprecated as a status.
- Decisions are `Decision` concepts in docs/decisions/, such as [dropping workz](/decisions/dropping-workz.md).

## Trust and validation

- Never add a `verified` entry: only humans verify, and okf refuses forged human verifiers.
- `just knowledge` runs `okf validate docs --strict --drift --stale`. Broken links, orphans,
  index descriptions that drift from the concept, missing `code_refs` and expired `stale_after`
  dates all fail it. It also runs `okf agents lint --strict AGENTS.md`, which keeps the memory
  block in AGENTS.md within okf's token budget. It runs in `just check`, on every commit and in CI.
- okf quirks found while building the template: [template notes](/template/notes.md).
