---
type: Guide
title: Upgrading from the template
description: How to bring a newer template release into this project by generating the new tag with the same values and porting the diff.
tags: [workflow, template, upgrade, cargo-generate]
status: stable
code_refs: [justfile, hk.pkl, mise.toml]
---

# Upgrading from the template

cargo-generate cannot update a project after generation. The [template notes](/template/notes.md)
record which template release this project came from, and the harness lives in files you can
diff: `justfile`, `hk.pkl`, `mise.toml`, `.config/`, `.github/`, `harness/`, `docs/` and the agent
configs. Crate code is yours; template releases change the harness.

1. Generate the new release into a temporary directory with this project's values (name,
   description, owner, license, service name), for example
   `cargo generate --git <template repo> --tag <new tag> --name <project> --destination "$(mktemp -d)" --define description="..." --define gh_owner=... --define license="..." --define service_name=... --silent`
   (or `--path` with the template checked out at that tag).
2. Diff it against this project, starting with the files above, and port the changes you want.
3. `mise install`, `just bootstrap`, `just ci`. Update the release recorded in the template
   notes, and add a `docs/log.md` entry.

How setup works after an upgrade: [setup and worktrees](/workflows/setup.md).
