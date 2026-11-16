---
name: x-overhaul
description: Full btm codebase overhaul — audit all source files, fix every finding, and commit. Use only when the user explicitly invokes $x-overhaul.
---

# BTM Full Codebase Audit-Fix-Commit Pipeline

Resolve repository paths from the Git root; Markdown links are relative to
this file. Read invocation arguments from the user prompt. This explicit
invocation authorizes the local fix commits produced by the pipeline.

## Phase 1: Full Codebase Review

Execute the full audit protocol of [x-review](../x-review/SKILL.md) with the
`all` scope:

1. Load the skills' required documentation
   (`docs/ai/technical-patterns.md`, `docs/ai/review-core.md`,
   `docs/ai/false-positive-guide.md`).
2. Perform the Full Audit Protocol: one reviewer per subsystem group (read
   all its source files, load the matching pattern guide), deep analysis of
   shell safety, snapshot identity, rollback ordering, retention, and the
   Linux / External split.
3. Aggregate and deduplicate all findings.
4. Manage `docs/audit.md` — prune fixed entries, merge new findings sorted
   by severity.

## Phase 2: Fix & Commit

Execute the full [x-fix](../x-fix/SKILL.md) protocol, including its
self-review and commit phases:

1. Read `docs/audit.md`.
2. Fix, validate, update the registry, self-review the aggregate diff, and
   commit per that skill.
3. Iterate until `docs/audit.md` has zero open entries (or only Won't Fix).

Do not push, tag, amend, or rewrite history.

## Output Format

```
## Full Audit Pipeline Summary

### Review (full codebase)
**Subsystems audited**: <list>
**Total findings**: N (X critical, Y high, Z medium, W low)

### Fix & Commit
**Fixed**: X | **Won't Fix**: Y | **Remaining**: 0
**Commit**: <short hash> <subject line>
```
