---
name: x-fix
description: Resolve every open finding in docs/audit.md for btm, self-review the result, and commit. Use only when the user explicitly invokes $x-fix.
---

# BTM Audit Backlog Fix

Resolve repository paths from the Git root; Markdown links are relative to
this file. Read invocation arguments from the user prompt.

How this differs from [x-commit](../x-commit/SKILL.md): `x-commit` starts
from the uncommitted diff ("review my changes and commit"); `x-fix` starts
from `docs/audit.md` ("work through the backlog — fix, verify, commit").
This explicit invocation authorizes local fix commits of the audit work.

## Setup

Read first:
[technical-patterns.md](../../../docs/ai/technical-patterns.md),
[review-core.md](../../../docs/ai/review-core.md),
[false-positive-guide.md](../../../docs/ai/false-positive-guide.md), and
`docs/audit.md` — the primary work list.

## Phase 1: Fix

### 1. Triage

1. Read `docs/audit.md`. If no `## Open` entries exist, report "nothing to
   fix" and stop.
2. Sort open findings CRITICAL → HIGH → MEDIUM → LOW.
3. For each finding, read the code at the reported location with full
   context (100+ lines, or the whole file when it is shorter).
4. For each affected subsystem, read the matching guide under
   `docs/ai/patterns/`.

### 2. Fix

For each open finding, in severity order:

1. **Understand** the root cause — read the code, trace call sites,
   identify the violated invariant.
2. **Implement** a complete fix that fully resolves the finding (no
   band-aids), introduces no new issues (boundary conditions, error
   paths, crash windows), respects snapshot invariants (shell validation
   before execution, exact `<volume>@<digits>` identity, btrfs rollback
   copies before it deletes, Fair/Fade keep the newest window), and
   follows existing code style.
3. **Verify** by re-reading the modified code and tracing its effects.
4. If a finding **cannot be fixed** (technical limitation, disproportionate
   risk, architectural redesign required), move it to `## Won't Fix` with a
   clear `**Reason**`.

### 3. Validate

1. Re-read every modified file for regressions.
2. Run `make fmt`, then `make lint`.
3. Run `make test` when the fix changes command strings, retention, or
   validation. Tests do not require zfs/btrfs binaries; they cover pure
   logic and command text.

### 4. Update Audit Registry

1. Remove all fixed entries from `## Open`.
2. Add `**Reason**` to entries moved to `## Won't Fix`.
3. Write the updated `docs/audit.md`.

## Phase 2: Self-Review

1. Run `git diff HEAD`. If empty, report "nothing to commit" and stop.
2. Execute the [x-review](../x-review/SKILL.md) execution protocol on the
   diff, including shell-safety, rollback-ordering, and retention checks.
3. Refute every new finding against `false-positive-guide.md`.
4. If new findings emerge: fix them immediately, update `docs/audit.md`,
   and repeat until `## Open` has zero entries (or only Won't Fix).

## Phase 3: Commit

1. Run `make fmt`.
2. Run `git diff HEAD --stat` and `git log -5 --oneline` for scope/style.
3. Stage specific files with `git add` (not `-A`).
4. Bump the package patch version if any `.rs` file changed — see
   [x-commit](../x-commit/SKILL.md) step 4.
5. Draft a commit message summarizing the audit fixes and commit with a
   HEREDOC — **no co-author line**:

```
git commit -m "$(cat <<'EOF'
<commit message here>
EOF
)"
```

6. Run `git status` to verify success. Do not push, tag, amend, or rewrite
   history.

## Output Format

```
## Audit Fix Summary

**Open before**: N findings
**Fixed**: X
**Won't Fix**: Y (moved with reasons)

### Self-Review
**New findings**: N (all resolved)

### Commit
**Commit**: <short hash> <subject line>
```
