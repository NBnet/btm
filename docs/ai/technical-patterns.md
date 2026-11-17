# BTM Technical Bug Patterns

Catalog of bug categories in this snapshot manager. Load it before a review
or a fix.

---

## Category 1: Shell Injection

### 1.1 Volume Interpolated Without Validation

**Pattern**: A command string includes `cfg.volume` (or a mountpoint read
back from `zfs get`) before `validate_volume` / `validate_params`.
**Where**: `BtmCfg::snapshot`, `rollback`, `get_sorted_snapshots`,
`clean_snapshots`; `driver::gen_snapshot`; `sync_volume`.
**Impact**: The volume is passed to `sh -c` via `ruc::cmd`. A `;`, `$()`,
quote, or leading `-` becomes another command or a tool flag.
**Check**: Validation runs first on every entry, including struct literals.
The allowed set is ASCII alphanumeric plus `/`, `-`, `_`, `.`.

### 1.2 Mountpoint Used as a Path Without a Check

**Pattern**: `zfs get mountpoint` output is passed to `syncfs_path` or a
shell even when it is `-`, `none`, or not an absolute path.
**Where**: `BtmCfg::sync_volume`.
**Impact**: Sync the wrong filesystem, or interpolate unexpected text.
**Check**: Only a value that starts with `/` is used. Anything else falls
back to the global `sync(2)`.

---

## Category 2: Destructive Rollback

### 2.1 Live Subvolume Deleted Before the Replacement Exists

**Pattern**: Btrfs rollback deletes `<volume>` and only then tries to
snapshot `<volume>@<idx>` into place.
**Where**: `Btrfs::rollback_cmd`.
**Impact**: A missing snapshot, a failed snapshot, or a crash leaves no
live subvolume.
**Check**: Order is: remove a stale `<volume>.btm-rollback-tmp` if present,
snapshot the backup onto that temp name, delete the live subvolume, `mv`
the temp into place. Steps are `&&`-chained. The command must not name the
live volume as the first `btrfs subvolume delete` target.

### 2.2 Wrong Rollback Target

**Pattern**: `None` does not mean "newest", lax mode returns a newer
snapshot, or strict mode accepts a neighbor.
**Where**: `rollback_target`, `rollback`, CLI in `src/bins/btm.rs`.
**Impact**: The volume returns to the wrong height, and ZFS `-r` then
destroys everything newer than that wrong target.
**Check**: Ascending `binary_search` semantics in `rollback_target`. The
CLI converts `snapshot_id < 0` to `None`. The library uses `u64::try_from`
on a caller-supplied `i128`.

---

## Category 3: Snapshot Identity

### 3.1 Parser Accepts a Non-BTM Name

**Pattern**: Listing output is split on `@` and the suffix is parsed
without requiring the prefix to be exactly `volume` and the suffix to be
all digits.
**Where**: `parse_exact_snapshot`, `Zfs::parse_snapshot_line`,
`Btrfs::parse_snapshot_line`.
**Impact**: A child dataset, a sibling subvolume, or a manual snapshot is
cleaned or used as a rollback target.
**Check**: Accept only `<volume>@<all-digits>`. Btrfs lists by globbing
that pattern, not by translating `btrfs subvolume list` paths.

### 3.2 Unaligned Index Still Mutates

**Pattern**: `idx_aligned` fails open, or clean-up runs before the
alignment gate.
**Where**: `idx_aligned`, `gen_snapshot`.
**Impact**: Extra snapshots, or retention deletes on an index that should
have been a no-op.
**Check**: Alignment is `(u64::MAX - idx).is_multiple_of(itv)` and it runs
after validation and before any list or delete. An existing index is an
error, not a silent success.

---

## Category 4: Retention

### 4.1 Fair/Fade Deletes Snapshots Inside the Kept Window

**Pattern**: Descending order is treated as ascending, or Fade's chunk
split uses the wrong power of `itv`.
**Where**: `fair_to_delete`, `fade_to_delete`, `clean_outdated`,
`clean_all`.
**Impact**: The newest backup is destroyed, or old snapshots grow without
bound.
**Check**: Inputs are newest-first. Fair returns `snaps_desc[cap..]`.
Fade deletes in-chunk indexes not aligned to `itv^(1+n)`, then everything
past `cap`. `get_cap` clamps to `CAP_MAX`.

### 4.2 Explicit Clean Reports Success After a Failed Destroy

**Pattern**: `clean_all` uses `info_omit!` the way `clean_outdated` does.
**Where**: `clean_all`, `destroy_batched`.
**Impact**: The caller thinks snapshots are gone; they are not.
**Check**: `clean_outdated` may log and continue. `clean_all` and
`destroy_batched` must return an error when deletes fail. Batch destroy
may fall back per index.

---

## Category 5: Platform and External Mode

### 5.1 Off-Linux Path Shells Out or Pretends a Rollback Worked

**Pattern**: `cfg!(not(target_os = "linux"))` is missing on `snapshot`, so
a non-Linux caller shells out, or `snapshot()` returns `Ok` after a real
snapshot command failed.
**Where**: `BtmCfg::snapshot`. Rollback, list, and clean have no platform
gate.
**Impact**: A macOS or Windows embedder believes a backup exists.
**Check**: Non-Linux `snapshot()` validates, warns once, and returns
`Ok(())` before `sync_volume`. Rollback, list, and clean still run the
ZFS or Btrfs commands. `External` returns `Err` for those three on every
OS. The non-Linux binary does not call them.

### 5.2 Daemon Dies on One Bad Datagram

**Pattern**: `run_daemon` returns on decode failure or a vanished peer.
**Where**: `src/api/server/mod.rs`.
**Impact**: Snapshots stop until someone restarts the unit.
**Check**: The loop logs and continues. A snapshot error becomes
`Resp.success = false`, not a process exit.

---

## Category 6: Durability

### 6.1 Snapshot Captures Unflushed Writes

**Pattern**: `sync_volume` is removed or runs after `create_snapshot_cmd`.
**Where**: `BtmCfg::snapshot`.
**Impact**: The snapshot misses the block the caller just wrote. Rollback
restores a stale volume.
**Check**: Scoped `syncfs(2)` (or the global fallback) finishes before the
driver creates the snapshot. `External` sync is the daemon's
responsibility because the client does not know the volume.
