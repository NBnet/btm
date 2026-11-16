# BTM Review Core Methodology

This document defines the systematic review protocol for BTM code changes.

---

## Phase 1: Context Gathering

Before analyzing any change, gather context:

1. **Read the diff** — understand every changed line.
2. **Identify affected subsystems**:
   - `src/lib.rs` → public API, validation, platform split
   - `src/driver/mod.rs` → shared orchestration, Fair/Fade retention
   - `src/driver/zfs.rs` → ZFS command strings
   - `src/driver/btrfs.rs` → Btrfs command strings and rollback ordering
   - `src/driver/external.rs` → daemon client
   - `src/api/` → Unix-socket daemon
   - `src/bins/btm.rs` → CLI
3. **Load subsystem patterns** — read the matching file under
   `docs/ai/patterns/`.
4. **Check call sites** — every new or changed command builder, and every
   caller of `validate_params`, `gen_snapshot`, `rollback`, and
   `clean_snapshots`.
5. **Check tests** — `src/lib.rs` and `src/driver/mod.rs` hold the unit
   tests for validation, command strings, and retention.

## Phase 2: Change Classification

| Category | Description | Risk Level |
|----------|-------------|------------|
| Shell command construction | Volume or index interpolated into a shell string | CRITICAL |
| Rollback ordering | Live volume deleted relative to its replacement | CRITICAL |
| Snapshot identity | Which names are listed, cleaned, or rolled back | CRITICAL |
| Retention selection | Fair/Fade chooses the wrong indexes to destroy | HIGH |
| Durability | `syncfs` / snapshot ordering | HIGH |
| Platform split | Linux vs other OS, `External` vs local drivers | HIGH |
| Index arithmetic | `itv` alignment, overflow, negative CLI ids | HIGH |
| Error handling | `Result`, logged-and-ignored deletes | MEDIUM |
| CLI / daemon protocol | Flags, env vars, UDS request bytes | MEDIUM |
| Documentation | Public docs drifting from behavior | LOW |
| Test changes | New or modified tests | LOW |

## Phase 3: Regression Analysis

For each HIGH or CRITICAL change:

### 3.1 Invariant Check

**Shell safety:**
- `validate_volume` allows only ASCII alphanumeric, `/`, `-`, `_`, `.`,
  rejects empty and a leading `-`.
- `validate_params` also requires `itv >= 1`, `cap >= 1`, and
  `itv.checked_pow(STEP_CNT)`.
- Every path that shells out calls validation first, including
  `gen_snapshot` and struct-literal configs.

**Snapshot identity:**
- Created names are `<volume>@<idx>` with a decimal index.
- Parsers accept only that exact form. Child datasets, sibling subvolumes,
  and manual names are not counted.
- Creating an index that already exists is an error.
- Indexes not aligned to `itv` (anchored at `u64::MAX`) do not create a
  snapshot and do not delete anything.

**Rollback:**
- Target selection follows `rollback_target`: `None` is the newest; strict
  mode requires an exact index; lax mode picks the newest snapshot older
  than the request; a request older than every snapshot errors.
- The CLI maps `snapshot_id < 0` to `None` before the library call. The
  library rejects a negative `i128` via `u64::try_from`.
- ZFS uses `zfs rollback -r`, which destroys newer snapshots. That is
  intentional.
- Btrfs snapshots the backup onto `<volume>.btm-rollback-tmp` before
  `btrfs subvolume delete` of the live volume. A missing source aborts
  with the live volume intact. A stale temp is removed first. The steps
  are chained with `&&`.

**Retention:**
- Fair keeps the newest `min(cap, CAP_MAX)` snapshots.
- Fade thins older chunks by `itv^(1+n)` and still deletes anything past
  `cap`. When `cap < STEP_CNT`, chunk size is zero and Fade degenerates
  to the cap trim.
- A failed trim inside `clean_outdated` does not fail the new snapshot.
  `clean_all` reports deletion failure.

**Platform:**
- Off Linux, `snapshot()` returns `Ok(())` after validation and does not
  shell out. Rollback, list, and clean return errors.
- `External` snapshot goes through the daemon client. Rollback, list, and
  clean error. The `btm` binary refuses to run as an `External` daemon.

### 3.2 Boundary Conditions

- `itv = 1` (every index) and a large `itv` whose power would overflow.
- `cap = 1`, `cap` just below `STEP_CNT`, `cap` above `CAP_MAX`.
- Empty snapshot list; exact hit; lax hit; strict miss; request older than
  every snapshot.
- Duplicate index; index not aligned to `itv`.
- Volume names: empty, leading `-`, `;`, `$()`, unicode, `.` and `/`.
- Btrfs rollback when the temp dir already exists, and when the source
  snapshot is missing.
- Non-Linux build. `External` mode. Daemon datagram that does not decode.

### 3.3 Failure Paths

- A failed shell command must surface on snapshot create, rollback, and
  explicit clean.
- Partial btrfs rollback must not leave the live subvolume deleted.
- Batch destroy falls back per index. Count the failures; do not report
  success when every delete failed.
- The daemon loop logs a bad datagram and keeps serving.

### 3.4 Concurrency

BTM does not take locks around snapshot operations. Two overlapping
`snapshot` / `rollback` / `clean` calls on the same volume can interleave
at the filesystem. Report that only when a change claims mutual exclusion
it does not implement, or when it races its own temp path
(`<volume>.btm-rollback-tmp`).

## Phase 4: Cross-Cutting Concerns

### 4.1 Crash Window

If the change touches rollback or snapshot creation, name the window where
`kill -9` lands: after the temp snapshot exists, after the live subvolume
is deleted, after `syncfs` but before `zfs snapshot`. The btrfs swap must
still prefer a leftover temp over a missing live volume.

### 4.2 Performance

A missed alignment check that shells out on every index, or an unbounded
`zfs list` parse that keeps non-btm snapshots, is a finding. Cold CLI
startup is not.

### 4.3 API Contract

`BtmCfg` fields are public. Changing default `itv` / `cap`, the meaning of
a negative CLI id, or the `Ok` vs `Err` of `External` and non-Linux
`snapshot()` is an API break and must update docs.

### 4.4 Code Style

Violations are LOW:
- No `#[allow(...)]`.
- No new `unsafe`.
- `#![deny(missing_docs)]` on public items.
- Doc-code alignment: `AGENTS.md`, `README.md`, `docs/ai/review-core.md`,
  `.agents/skills/x-review/SKILL.md`, and `docs/ai/patterns/`.

## Phase 5: Reporting

```
[SEVERITY] subsystem: one-line summary

WHERE: file:line_range
WHAT: Description of the issue
WHY: Invariant or pattern from technical-patterns.md
FIX: Suggested fix, or the question that blocks one
```

- **CRITICAL**: shell injection, live volume destroyed, wrong snapshot destroyed
- **HIGH**: wrong retention set, snapshot skipped or duplicated, durability gap
- **MEDIUM**: error swallowed on a path the caller depends on, CLI/index mismatch
- **LOW**: docs, clarity
- **INFO**: observation, not a bug

Only report findings with concrete evidence. Consult
[false-positive-guide.md](false-positive-guide.md) before finalizing.
