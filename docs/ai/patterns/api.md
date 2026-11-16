# Public API Review Patterns

## Files
- `src/lib.rs` — `BtmCfg`, `SnapMode`, `SnapAlgo`, `snapshot`, `rollback`,
  `get_sorted_snapshots`, `list_snapshots`, `clean_snapshots`, `sync_volume`

## Architecture
- Callers build a `BtmCfg` either with `BtmCfg::new` (validates the volume
  and guesses the mode) or with a struct literal (validation is deferred
  until the operation).
- `snapshot(idx)` flushes, then dispatches to ZFS, Btrfs, or the external
  daemon.
- `rollback`, list, and clean refuse `External`.
- `get_cap` clamps `cap` to `CAP_MAX` (4096). `STEP_CNT` is 10.

## Critical Invariants

### INV-API1: Validate Before Any Shell
`validate_params` runs at the start of `snapshot`, `rollback`,
`get_sorted_snapshots`, and `clean_snapshots`.
**Check**: A new public method that reaches `ruc::cmd` or `syncfs` does
the same. `gen_snapshot` also validates, so a driver-level entry cannot
skip it.

### INV-API2: Non-Linux Snapshot Is a No-Op, Nothing Else Is
**Check**: The `cfg!(not(target_os = "linux"))` branch returns `Ok(())`
only from `snapshot`, and only after validation. Rollback, list, and
clean have no successful non-Linux arm.

### INV-API3: Negative Index Does Not Wrap
**Check**: `rollback`'s `Option<i128>` becomes `u64` only through
`u64::try_from`. A negative value is an error, not `u64::MAX`.

### INV-API4: Flush Happens Before the Snapshot
**Check**: `sync_volume` is called before the driver match. ZFS uses the
dataset mountpoint only when `zfs get` returns a path starting with `/`.
Btrfs syncs `cfg.volume` itself. External mode does not pretend to sync.

## Common Bug Patterns

### Validation Gap (technical-patterns.md 1.1)
A struct-literal `BtmCfg` reaches `format!("zfs snapshot {}@{}", ...)`.

### Platform Lie (technical-patterns.md 5.1)
Off-Linux rollback returns `Ok(())`.

## Review Checklist
- [ ] `validate_params` before every shell or sync
- [ ] Volume charset and leading `-` still rejected
- [ ] `itv >= 1`, `cap >= 1`, `itv.checked_pow(10)` still checked
- [ ] Non-Linux `snapshot()` does not shell out
- [ ] `External` does not run local zfs/btrfs mutations
- [ ] Public docs match the `Ok`/`Err` of each mode
