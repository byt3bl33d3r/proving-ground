@AGENTS.md

## Claude Code specifics
- MCP servers: victoriametrics, victorialogs, victoriatraces (this worktree's stack; start it with
  `just up`; they answer once the stack is up, no restart needed), okf-memory (docs/ search) and hk.
- The Stop hook runs hk's check steps on your changed files. If it blocks, fix what it reports
  before finishing.
- Worktrees from `claude --worktree` get their own ports automatically; removing one runs
  `just down` there first.
- Skill: `.claude/skills/harness/SKILL.md` walks through reproduce, observe, fix, verify.
