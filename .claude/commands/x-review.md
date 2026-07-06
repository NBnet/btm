---
description: Deep regression review of BTM changes (latest commit, N commits, hash, range, or full audit)
argument-hint: "[N | all | <hash> | <hash1>..<hash2>] [--fix]"
---

# Deep Regression Analysis for BTM

You are performing a deep code review of changes to BTM (Blockchain Time Machine),
a Rust library for ZFS/BTRFS incremental snapshot management.
This review uses Claude Code's multi-agent review architecture adapted to BTM's small codebase.

## Input

Arguments: `$ARGUMENTS`

Parse to determine scope; `--fix` flag means apply verified fixes after review.
Use the session's current effort level (no explicit override — review depth scales with it naturally).

| Input | Scope |
|-------|-------|
| *(empty)* | Latest commit |
| `N` (integer) | Last N commits |
| `all` | Full codebase audit |
| `<commit hash>` | Specific commit |
| `<hash1>..<hash2>` | Commit range |

## Execution Protocol (diff-based reviews)

### Phase 1: Context & Classification

1. Read the full diff (`git diff <range>`)
2. Identify affected subsystems:
   - `src/lib.rs` — public API, BtmCfg, SnapMode, SnapAlgo, validation
   - `src/driver/zfs.rs` — ZFS snapshot/rollback/clean CLI orchestration
   - `src/driver/btrfs.rs` — BTRFS snapshot/rollback/clean CLI orchestration
   - `src/driver/external.rs` — external daemon integration
   - `src/driver/mod.rs` — shared driver logic, snapshot retention algorithms
   - `src/api/` — daemon API (client/server)
   - `src/bins/btm.rs` — CLI binary entry point
3. Classify each change: control flow, shell safety, error handling, platform compatibility, API contract

### Phase 2: Parallel Multi-Agent Review

Launch **3 review agents in parallel** (smaller codebase — 3 agents covers all dimensions).
Each agent receives: the full diff and the summary context.

**Agent 1 — Correctness & Shell Safety** (deep context read):
Scan for bugs that require understanding surrounding code. Focus on:
- **Shell injection**: every `cmd::exec` or shell-out path must pass through `validate_volume` / `validate_params`. Volume names that break `validate_volume` (starts with `-`, contains `;`, `$`, backticks, etc.) must be rejected before reaching any shell.
- **Rollback safety**: `rollback()` is destructive — verify the target resolution (optional idx, strict mode) is correct.
- **Snapshot retention**: `Fair`/`Fade` algorithms correctly determine which snapshots to keep/delete. No off-by-one in retention counts.
- **Platform gating**: `#[cfg(target_os = "linux")]` / `#[cfg(not(target_os = "linux"))]` correctness — non-Linux must not shell out; `External` mode gated correctly.
- **Validation completeness**: every public operation calls `validate_params()` before reaching a shell.
- Only flag issues with concrete failure scenarios.

**Agent 2 — Diff-Only Bugs** (diff surface scan):
Scan ONLY the diff lines without reading extra context. Flag:
- Syntax errors, type errors, missing imports (will not compile)
- Clear logic errors visible in the diff alone (inverted conditions, off-by-one)
- Unreachable code, dead branches introduced by the change
- Ignore anything that requires surrounding code to validate

**Agent 3 — Code Style & Conventions** (project rules + cross-cutting):
Check changed files against:
- `#![deny(warnings)]` and `#![deny(missing_docs)]` — no new suppressions
- Error handling: all fallible operations use `.c(d!())`, never bare `.unwrap()` on `ruc::Result`
- Import grouping (std → external → crate), common prefixes merged
- Doc-code alignment: public API changes must update doc comments and README.md
- Crash safety: sync_volume before destructive ops; rollback atomicity

**CRITICAL: Only report HIGH SIGNAL issues.** Flag only:
- Code that will definitely fail to compile
- Code that will definitely produce wrong results
- Shell injection or validation bypass
- Concrete crash/data-loss/corruption scenarios

Do NOT flag: style preferences, "consider" suggestions without concrete downside, issues a linter catches.

### Phase 3: Verification

For each finding from Phase 2 agents, launch a **verification agent** that:
1. Re-reads the reported code location with full context
2. Attempts to CONFIRM or REFUTE the finding against actual code
3. Returns only CONFIRMED findings with concrete evidence

Filter out any finding not confirmed by its verification agent.

### Phase 4: Audit Registry

1. Read `docs/audit.md` (create if absent)
2. **Prune**: Remove `## Open` entries that are 100% fixed in current code
3. **Merge**: Add confirmed findings under `## Open`, deduplicating against existing entries. Sort by severity (CRITICAL → HIGH → MEDIUM → LOW)
4. **Re-evaluate Won't Fix**: For each `## Won't Fix` entry, re-read the code. Promote to `## Open` if now fixable; remove if no longer applicable; keep if reason still holds
5. Write updated `docs/audit.md`. **Never include timestamps, dates, or time-based markers.**

Format:

```markdown
# Audit Findings

> Auto-managed by /x-review and /x-fix.

## Open

### [SEVERITY] subsystem: one-line summary
- **Where**: file:line_range
- **What**: description
- **Why**: invariant/pattern violated
- **Suggested fix**: how to fix

---

## Won't Fix

### [SEVERITY] subsystem: one-line summary
- **Where**: file:line_range
- **What**: description
- **Reason**: why this cannot or should not be fixed
```

### Phase 5: Report

Use the **ReportFindings** tool with the confirmed findings. Then output a terminal summary:

```
## Review Summary

**Scope**: <commits/diff description>
**Subsystems**: <list>
**Findings**: N (X critical, Y high, Z medium, W low)

## Findings
(one line per finding with severity and location)
```

If zero findings: `**Result**: LGTM — no regressions found. Coverage: <subsystems and invariants checked>.`

### Phase 6: Fix (if --fix)

If `--fix` was passed and findings exist:
1. Apply each fix to the working tree
2. Re-report findings via ReportFindings with `outcome` set (`fixed`, `skipped`, `no_change_needed`)

---

## Full Audit Protocol (for `all` mode)

### Strategy: Parallel File-Group Audit

Launch **3 agents in parallel** covering the full codebase:

| Agent | Files | Focus |
|-------|-------|-------|
| core | `src/lib.rs`, `src/driver/mod.rs` | Public API, BtmCfg, SnapMode/SnapAlgo, validation, retention algorithms |
| drivers | `src/driver/zfs.rs`, `src/driver/btrfs.rs`, `src/driver/external.rs` | Shell orchestration, injection safety, error handling, platform gating |
| api + bin | `src/api/`, `src/bins/btm.rs` | Daemon API, CLI entry, argument parsing |

Each agent's prompt must be self-contained and include:
1. Exact file list for the group
2. The relevant concerns from Phase 2
3. High-signal-only rule

### Aggregation

After all agents complete:
1. Collect all findings
2. Launch verification agents for each finding (Phase 3)
3. Deduplicate findings
4. Update audit registry (Phase 4)
5. Report with ReportFindings + terminal summary (Phase 5)
6. Fix if --fix (Phase 6)
