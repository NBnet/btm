# Driver and Retention Review Patterns

## Files
- `src/driver/mod.rs` — `SnapDriver`, `gen_snapshot`, `sorted_snapshots`,
  `rollback_target`, `rollback`, `clean_all`, `fair_to_delete`,
  `fade_to_delete`, `destroy_batched`
- `src/driver/zfs.rs` — ZFS command strings, comma-separated batch destroy
- `src/driver/btrfs.rs` — Btrfs command strings, rollback swap, path batch
  destroy
- `src/driver/external.rs` — forwards `gen_snapshot` to the daemon client

## Architecture
- `SnapDriver` builds command strings. The shared module executes them,
  parses listings, and decides which indexes to delete.
- Listings are newest-first after `sorted_snapshots`.
- `rollback` reverses that to ascending order for `binary_search`.

## Critical Invariants

### INV-DR1: Exact Snapshot Names Only
`parse_exact_snapshot` accepts a line only when it is
`<volume>@<all-digits>`.
**Check**: ZFS and Btrfs both call it. Btrfs enumeration is a nullglob
over `<volume>@*`, preceded by `btrfs subvolume show` so a missing volume
still errors.

### INV-DR2: Alignment Gates Mutation
`gen_snapshot` returns `Ok(())` without listing or deleting when
`(u64::MAX - idx)` is not a multiple of `itv`.
**Check**: The gate sits after `validate_params` and before
`sorted_snapshots`. Duplicate indexes error.

### INV-DR3: Btrfs Rollback Keeps the Live Volume Until the Copy Exists
**Check**: `Btrfs::rollback_cmd` snapshots onto `<volume>.btm-rollback-tmp`
before `btrfs subvolume delete <volume>`. The chain is `&&`. ZFS stays
`zfs rollback -r <volume>@<idx>`.

### INV-DR4: Retention Uses Newest-First Input
**Check**: `fair_to_delete(snaps_desc, cap)` drops the tail.
`fade_to_delete` walks `STEP_CNT` chunks of `cap / STEP_CNT` and then
applies the same tail delete. `clean_outdated` may ignore a destroy
error. `clean_all` may not.

### INV-DR5: Batch Destroy Falls Back Per Index
**Check**: `destroy_batched` tries the batch command, then each
`destroy_cmd`, and returns an error if any index still failed. ZFS batches
as `zfs destroy vol@a,b,c`. Btrfs batches as one
`btrfs subvolume delete` with multiple paths.

## Common Bug Patterns

### Live Volume First (technical-patterns.md 2.1)
The first `subvolume delete` in the rollback string names the live volume.

### Parser Too Loose (technical-patterns.md 3.1)
A suffix after the first `@`, or a shared basename, becomes an index.

## Review Checklist
- [ ] Command strings quote nothing they do not need, because validation
      already excluded metacharacters — do not weaken validation to "fix"
      quoting
- [ ] Rollback target tests still cover empty, exact, lax, strict, and
      too-old
- [ ] Fade with `cap < STEP_CNT` does not delete the newest `cap`
- [ ] External driver does not format a zfs or btrfs command
