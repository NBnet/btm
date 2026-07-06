---
description: Deep regression review of BTM changes (latest commit, N commits, hash, range, or full audit)
argument-hint: "[N | all | <hash> | <hash1>..<hash2>] [--fix]"
---

# Deep Regression Analysis for BTM

You are performing a deep code review of changes to BTM (Blockchain Time Machine),
a Rust library for ZFS/BTRFS incremental snapshot management.
This review combines BTM-specific pattern analysis with Claude Code's native
multi-agent review architecture: **dimensional review agents → adversarial
verification → completeness critic → structured report**.

Since BTM is a smaller codebase, the multi-agent structure uses 3 agents
instead of 4 — combining cross-cutting concerns into the style agent.

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

---

## Unified Protocol

All modes follow the same 7-phase structure. Mode-specific adaptations are noted inline.

### Phase 1: Scope & Context

**All modes**:
1. Identify affected subsystems:
   - `src/lib.rs` — public API, BtmCfg, SnapMode, SnapAlgo, validation
   - `src/driver/zfs.rs` — ZFS snapshot/rollback/clean CLI orchestration
   - `src/driver/btrfs.rs` — BTRFS snapshot/rollback/clean CLI orchestration
   - `src/driver/external.rs` — external daemon integration
   - `src/driver/mod.rs` — shared driver logic, snapshot retention algorithms
   - `src/api/` — daemon API (client/server)
   - `src/bins/btm.rs` — CLI binary entry point
2. Classify each change: control flow, shell safety, error handling, platform compatibility, API contract

**Diff modes** (empty, N, hash, range):
3. Read the full diff (`git diff <range>`)

**`all` mode**: all subsystems affected.

### Phase 2: Parallel Multi-Dimensional Review

This is the core of the review. Launch agents that cover distinct review *dimensions* —
different ways of seeing the same code, not just different files.

---

#### A. Diff modes (empty, N, hash, range)

Launch **3 agents in parallel** (smaller codebase — 3 agents covers all dimensions).
Each receives: the full diff and subsystem context.

**Agent 1 — Correctness & Shell Safety** (reads changed files with full context):
- **Shell injection**: every `cmd::exec` or shell-out path must pass through `validate_volume` / `validate_params`. Volume names that break `validate_volume` (starts with `-`, contains `;`, `$`, backticks, etc.) must be rejected before reaching any shell.
- **Rollback safety**: `rollback()` is destructive — verify the target resolution (optional idx, strict mode) is correct.
- **Snapshot retention**: `Fair`/`Fade` algorithms correctly determine which snapshots to keep/delete. No off-by-one in retention counts.
- **Platform gating**: `#[cfg(target_os = "linux")]` / `#[cfg(not(target_os = "linux"))]` correctness — non-Linux must not shell out; `External` mode gated correctly.
- **Validation completeness**: every public operation calls `validate_params()` before reaching a shell.
- Only flag issues with concrete failure scenarios.

**Agent 2 — Diff-Only Bugs** (scans diff surface, no extra context):
- Syntax errors, type errors, missing imports (will not compile)
- Clear logic errors in the diff alone (inverted conditions, off-by-one)
- Unreachable code, dead branches introduced by the change
- Ignore anything that requires surrounding code to validate

**Agent 3 — Code Style & Cross-Cutting** (all changed files):
- `#![deny(warnings)]` and `#![deny(missing_docs)]` — no new suppressions
- Error handling: all fallible operations use `.c(d!())`, never bare `.unwrap()` on `ruc::Result`
- Import grouping (std → external → crate), common prefixes merged
- Doc-code alignment: public API changes must update doc comments and README.md
- Crash safety: sync_volume before destructive ops; rollback atomicity
- API compatibility: observable behavior changes? semver implication?

---

#### B. `all` mode (full audit)

Full audit uses **two layers** — subsystem depth first, then cross-cutting breadth.

**Layer 1 — File-Group Audit (3 agents, parallel)**:

| Agent | Files | Focus |
|-------|-------|-------|
| core | `src/lib.rs`, `src/driver/mod.rs` | Public API, BtmCfg, SnapMode/SnapAlgo, validation, retention algorithms |
| drivers | `src/driver/zfs.rs`, `src/driver/btrfs.rs`, `src/driver/external.rs` | Shell orchestration, injection safety, error handling, platform gating |
| api + bin | `src/api/`, `src/bins/btm.rs` | Daemon API, CLI entry, argument parsing |

Each agent's prompt must be self-contained:
1. Exact file list for the group
2. The relevant concerns from Phase 2
3. High-signal-only rule: flag only confirmed bugs, not style preferences

**Layer 2 — Cross-Cutting Review (1 agent, launched after Layer 1 completes)**:

Once all group agents report, launch 1 agent that reads **ALL source files** with a
global lens:

- Shell injection: every shell-out path across all files verified for validate_volume coverage
- Platform gating: all `#[cfg(target_os)]` pairs correct across the codebase
- Error handling consistency: all fallible ops use `.c(d!())`; no bare unwrap on ruc::Result
- Import grouping consistent across ALL files
- Doc-code alignment: README.md matches current public API

---

**CRITICAL — High-signal gate (applies to ALL agents in ALL modes)**:

Only report findings with **concrete failure scenarios**:
- Code that will definitely fail to compile
- Code that will definitely produce wrong results
- Shell injection or validation bypass
- Concrete crash / data-loss / corruption scenarios

Do NOT flag: style preferences, "consider" suggestions without concrete downside,
or linter-caught issues.

### Phase 3: Adversarial Verification

For each finding from Phase 2, launch **3 verification agents in parallel**. Each agent:
1. Re-reads the reported code location with **full context**
2. Is instructed to **try to REFUTE** the finding — find concrete reasons it is NOT a real bug
3. Returns: `{confirmed: bool, evidence: string}`

**Survival rule**: a finding is CONFIRMED only if **≥2 of 3** verification agents confirm
it as real. Findings with 0–1 confirmations are discarded.

This adversarial pattern prevents plausible-but-wrong findings from surviving.

If zero findings emerged from Phase 2, skip this phase.

### Phase 4: Completeness Critic

Launch **one final review agent** that audits the review itself:
- What subsystems, files, or functions were NOT examined?
- What invariants (shell safety, rollback, retention, platform gating) were NOT verified?
- What edge cases (empty volumes, boundary retention counts, timeout paths) were NOT checked?
- What cross-subsystem interactions were missed?

If gaps are found, loop back to Phase 2 with the specific gap as new scope (launch
targeted agents for the missing coverage only). If no gaps remain, proceed.

If zero findings emerged from Phase 2 and the completeness critic finds no gaps,
skip directly to Phase 6 (no audit.md changes needed).

### Phase 5: Audit Registry

1. Read `docs/audit.md` (create if absent)
2. **Prune**: Remove `## Open` entries that are 100% fixed in current code
3. **Merge**: Add confirmed findings under `## Open`, deduplicating against existing entries.
   Sort by severity: CRITICAL → HIGH → MEDIUM → LOW
4. **Re-evaluate Won't Fix**: For each `## Won't Fix` entry, re-read the code.
   Promote to `## Open` if now fixable; remove if no longer applicable; keep if reason still holds
5. Write updated `docs/audit.md`. **Never include timestamps, dates, or time-based markers.**

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

### Phase 6: Report

Use the **ReportFindings** tool with the confirmed findings (empty array if none).
Then output a terminal summary:

```
## Review Summary

**Scope**: <commits/diff description or "full audit">
**Subsystems**: <list>
**Findings**: N (X critical, Y high, Z medium, W low)

## Findings
(one line per finding: severity, location, one-line summary)
```

If zero findings:
`**Result**: LGTM — no regressions found. Coverage: <subsystems and invariants checked>.`

### Phase 7: Fix (if --fix)

If `--fix` was passed and findings exist:
1. Apply each fix to the working tree
2. Re-report findings via ReportFindings with `outcome` set (`fixed`, `skipped`, `no_change_needed`)
