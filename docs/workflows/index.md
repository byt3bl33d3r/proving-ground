# Workflows

How work moves through the project: setup and worktrees, git hooks, CI, and upgrading from a newer template release.

Other workflows live with their topic: the day-to-day agent loop in conventions/agent-workflow,
recalling and recording knowledge in conventions/project-memory, DST failures in testing/dst,
fuzz crashes in hardening/fuzzing, panic-audit findings in hardening/panic-audit, assembly
snapshot review in performance/hot-paths, lint exceptions in conventions/lints, and UI evidence
in testing/evidence.

* [CI workflows](ci.md) - The GitHub Actions workflows (ci, perf, hardening, gardening), what each job runs, how failures are reported, and how to run workflows locally.
* [Git hooks](git-hooks.md) - What the hk git hooks run on commit and push, the agent Stop hooks, why hooks cannot be skipped, and what to do when one fails.
* [Setup and worktrees](setup.md) - First-time setup (mise, bootstrap), the per-worktree stack and server lifecycle, adding and removing worktrees, and the shared build cache.
* [Upgrading from the template](upgrading.md) - How to bring a newer template release into this project by generating the new tag with the same values and porting the diff.
