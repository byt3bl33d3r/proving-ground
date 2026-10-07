# Conventions

How agents work here (commands, project memory), errors and exit codes, module size, lint policy.

* [Agent workflow](agent-workflow.md) - The commands an agent runs, how to see what the app does, how to read a failing check, and what to do before finishing.
* [Errors and exit codes](errors.md) - Error types per layer, the JSON error body of the API, and the CLI exit codes 0 to 4.
* [Lint policy](lints.md) - The lint policy: denied groups, the expect escape hatch, the exception ledger, and the protected files.
* [Modules and file size](modules.md) - File and function size limits, and how to split code by responsibility when a limit is hit.
* [Project memory](project-memory.md) - How agents recall and record project knowledge in docs/ with okf: search first, update before create, links and code_refs, plans, decisions and verification.
