// Prints next steps only; runs no commands. The hooks directory is removed from the output.
let name = variable::get("project-name");
print(`
Next steps:
  cd ${name}
  git add -A && git commit -m "Initial commit from rust-project-template"
  mise trust && mise install
  just bootstrap      # lockfile, hk install, tool checks, first build
  just up             # per-worktree telemetry stack + server
  just check          # tier 0
`);
file::delete("template-hooks");
