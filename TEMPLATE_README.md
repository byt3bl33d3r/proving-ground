# proving-ground

A [cargo-generate](https://github.com/cargo-generate/cargo-generate) template for a Rust service
built to be worked on by coding agents (Claude Code, Codex): a layered workspace (core, runtime,
axum server, clap CLI), a per-worktree telemetry stack (VictoriaMetrics, VictoriaLogs,
VictoriaTraces over OTLP), deterministic simulation, fuzzing, Kani proofs, sanitizers, strict lint
policy with architecture tests, hk git hooks, and an OKF knowledge bundle in `docs/`. The full
specification is `SPEC.md`; deviations and verified tool versions are in
`docs/template/notes.md`.

This file, `SPEC.md`, `template.just` and `.github/workflows/template-ci.yml` exist only in the
template repository; generation removes them.

## Generate a project

```bash
cargo generate --path /path/to/proving-ground --name my-service
# later, once the repository is published:
cargo generate byt3bl33d3r/proving-ground --tag v1.0.0 --name my-service
```

Four prompts (all have defaults, so `--silent` works):

| Placeholder | Default | Used for |
| --- | --- | --- |
| `description` | `A Rust service` | AGENTS.md, README.md |
| `gh_owner` | `your-org` | repository URLs, CODEOWNERS |
| `license` | `MIT OR Apache-2.0` | `[workspace.package] license` (`Proprietary` becomes `LicenseRef-Proprietary`) |
| `service_name` | the project name | OpenTelemetry `service.name` |

Then follow the printed next steps: commit, `mise trust && mise install`, `just bootstrap`,
`just up`, `just check`. Requirements: mise, Docker, and macOS or Linux.

Git hooks, agent hooks and agent MCP servers all start their tools through `mise x -- ...`, so
only `mise` itself must be on the PATH that git and the agent apps see (Homebrew's
`/opt/homebrew/bin` usually is). For your own shell:

```bash
echo 'eval "$(mise activate zsh)"' >> ~/.zshrc
```

## How the template repository works

- **The repository root is the generated project root.** Names are Liquid placeholders
  (`{{project-name}}`, `{{crate_name}}`, ...), including directory names
  (`crates/{{project-name}}-core`). The tree therefore does not compile here; it is verified by
  generating a project and testing that (`just template-ci`).
- **Copied verbatim** (`exclude` in `cargo-generate.toml`): files that use `{{ }}` natively
  (`justfile`, `hk.pkl`, GitHub workflows), `harness/stack/`, `harness/lints/`, and the fuzz
  corpus. They must not contain project names; recipes read `PROJECT_NAME` from `mise.toml`.
- **`.liquid` twins.** mise renders its own double-brace templates, so the template repository
  cannot keep placeholders in a file mise reads. `mise.toml` holds concrete demo values (so mise
  works while developing the template) and `mise.toml.liquid` holds the placeholders; when both
  exist cargo-generate uses the `.liquid` one. Keep the two identical apart from the placeholders
  and `HK_FILE`, which only the template repository's `mise.toml` sets.
  `README.md.liquid` becomes the project README.
- **Hooks** (`template-hooks/`): `init.rhai` records `template_version`, `pre.rhai` resolves the
  `service_name` default (cargo-generate does not render placeholder defaults), `post.rhai` prints
  next steps. None runs commands.
- **Liquid pitfalls.** In templated files never write `{{`/`}}` escapes (Rust `format!`
  escapes are silently mangled) or `{%`; wrap unavoidable ones in `{% raw %}...{% endraw %}`.
  Binary files must be excluded. `just template-ci` fails on any leftover template syntax.
- **Git hooks here** use `template.hk.pkl` (ignored on generate), selected by `HK_FILE` in this
  repository's `mise.toml`. hk.pkl's Rust steps cannot run on unrendered crate names, so they are
  skipped here; the secrets scan, actionlint, the justfile lint and `just knowledge` still run.
  `just template-ci` exercises hk.pkl in the generated project.
- **Lockfiles** are not in the template (placeholder package names cannot be locked);
  `just bootstrap` generates them and generated projects commit them.

## Developing the template

```bash
just template-ci                       # generate demo-app, bootstrap, ci, up, e2e, down, isolation
KEEP=1 just template-ci                # keep the generated project to inspect it
LICENSE=Proprietary just template-ci   # another license choice
```

Make a change by generating a project (`cargo generate --path . --name demo-app`), making and
testing the change there, then porting it back with names replaced by placeholders.
`template-ci.yml` runs `just template-ci` for every license choice once the repository is on
GitHub.

## Releases and upgrades

cargo-generate cannot update a project after generation, so harness behaviour lives in files a
project can diff and copy: the `justfile`, `hk.pkl`, `mise.toml`, configs and `docs/`.

- **Release:** bump `template_version` in `template-hooks/init.rhai`, run `just template-ci`,
  commit, tag `vX.Y.Z`. Generated projects record the tag in `docs/template/notes.md`.
- **Upgrade a project:** generate the new tag into a temporary directory with the same values
  (name, description, owner, license, service name), diff it against the project (start with the
  `justfile`, `hk.pkl`, `mise.toml`, `.config/`, `.github/`, `harness/`, `docs/`), and port the
  changes you want. Then `just bootstrap` and `just ci`.
