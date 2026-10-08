## 2026-10-07
* **Creation**: Initialized the bundle with architecture, observability, testing, hardening, performance, conventions, decisions, plans and template concepts.
* **Update**: Added testing/evidence (planned UI evidence how-to), linked it and the Adding a UI decision from the test strategy, cross-linked the conventions, and listed the repo paths each concept names in its code_refs.
* **Update**: template/notes records the justfile lint, the knowledge hook on every commit, agents starting tools through mise x, and the template repository's own hook file; hardening/tiers lists the justfile lint in tier 0.
* **Update**: Added conventions/agent-workflow and conventions/project-memory, moving the commands, failure handling and okf how-to out of AGENTS.md; the seams, failing-test-first, Gungraun-numbers and no-unwrap rules now live in architecture/overview, testing/strategy, performance/benchmarks and conventions/lints.
* **Update**: template/notes records the AGENTS.md map decision and the okf AGENTS.md lint in `just knowledge`; conventions/project-memory mentions the lint.
* **Update**: template/notes: AGENTS.md keeps every rule as a one-liner linking to its concept (human decision).
* **Update**: agent-workflow names `just --show <recipe>` as the way to read a private recipe.
* **Update**: Grouped the justfile menu and made its plumbing private; `logs`, `query`, `fuzz`, `fuzz-crash`, `kani [full]`, `deny [offline]` and `fmt [--check]` replace the one-flag variants and `status` carries the UI URLs; agent-workflow, errors, queries, telemetry, setup, fuzzing, kani and the template notes name the new recipes; a test in harness/checks verifies that every `just <recipe>` in docs/ exists.
* **Creation**: Added the workflows section (setup and worktrees, git hooks, CI workflows, upgrading from the template) and hardening/miri; tiers documents mutation testing and the release tier; template/notes records the act gaps.
* **Update**: testing/strategy: harness/dst has its own nextest config and needs every profile CI selects; harness/checks/tests/nextest.rs enforces it (template-ci failed on `profile ci not found`).
