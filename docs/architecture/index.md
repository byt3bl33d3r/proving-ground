# Architecture

Crate layout, layering rules and per-worktree port allocation.

* [Layers](layers.md) - Allowed dependencies between crates and between modules inside core, and how the architecture tests enforce them.
* [Architecture overview](overview.md) - Crate layout (core, runtime, server, cli), the harness directory, and how one request flows through them.
* [Port allocation](port-allocation.md) - How each git worktree gets its own range of local ports for the telemetry stack, derived from a hash of its path.
