# Plans

Multi-step work: active plans are status draft with tag active; finished ones are stable with tag completed.

Create a plan with `okf create plans/<slug> docs --type Plan --status draft --tags plan,active`
(bundle right after the id) and finish it with `okf update plans/<slug> docs --status stable --tags plan,completed`.

