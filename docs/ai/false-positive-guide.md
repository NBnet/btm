# BTM False Positive Guide

Before reporting any finding, check it against this guide.

---

## FP-1: Rust Ownership Already Prevents It

**Pattern**: Reporting memory unsafety in safe Rust.
**Rule**: This crate has no `unsafe`. Only report memory unsafety if a new
`unsafe` block appears, and then require a `SAFETY` comment.

## FP-2: Non-Linux `snapshot()` Succeeds

**Pattern**: Reporting that `snapshot()` returns `Ok(())` on platforms other
than Linux.
**Rule**: That no-op is intentional so callers can leave the call on the hot
path. Rollback, list, and clean must still fail on those platforms. Report
only if a destructive or query operation pretends to succeed off Linux, or
if `snapshot()` starts shelling out there.

## FP-3: Retention Cleanup Failure Does Not Abort the Snapshot

**Pattern**: Reporting that a failed `destroy` during `clean_outdated` is
logged and ignored.
**Rule**: A failed trim must not block today's snapshot. Report it only if
the new snapshot is skipped, or if the error is swallowed on an explicit
`clean_snapshots()` / rollback path where the caller asked for deletion.

## FP-4: Fade Degenerates When `cap < STEP_CNT`

**Pattern**: Reporting that Fade retention behaves like a plain cap trim
when `chunk_size` is zero.
**Rule**: `fade_to_delete` documents this. `cap` is also clamped by
`CAP_MAX` (4096). Report only if the degeneration deletes snapshots the
plain cap trim would have kept, or if `itv.pow(STEP_CNT)` can overflow
despite `validate_params`.

## FP-5: External Mode Refuses Rollback, List, and Clean

**Pattern**: Reporting that `External` returns an error for rollback, list,
and clean.
**Rule**: External mode only forwards `snapshot()` to the daemon. The CLI
also rejects `External` as a daemon mode. Report only if a path executes a
local zfs/btrfs command while `mode` is `External`.

## FP-6: Global `sync(2)` Fallback

**Pattern**: Reporting that `sync_volume` falls back to `nix::unistd::sync`.
**Rule**: The fallback is intentional when the mountpoint cannot be
resolved. Report only if a resolved mountpoint is ignored, or if the
snapshot proceeds with no flush attempt at all.

## FP-7: Unwrap/Expect on Known-Valid State

**Pattern**: Reporting `unwrap()` as a potential panic.
**Rule**: Check whether the value is guaranteed by prior logic or the code
is a test. Report only with a concrete production input that panics.

## FP-8: Clippy or `deny(warnings)` Would Catch It

**Pattern**: Reporting formatting, unused items, or missing docs.
**Rule**: `make lint` plus `#![deny(warnings)]` and `#![deny(missing_docs)]`
already gate those. Focus on semantic bugs.

## FP-9: "Consider" Without a Concrete Downside

**Pattern**: Advisory suggestions with no trigger and no impact.
**Rule**: Every finding needs a concrete input and a concrete bad outcome
(wrong snapshot destroyed, live volume deleted, shell metacharacter
executed, index skipped or double-created).

## FP-10: Test-Only Code Held to Production Standards

**Pattern**: Reporting unwraps or hardcoded volumes inside `#[cfg(test)]`.
**Rule**: Report a test only when the assertion is wrong or it locks in a
dangerous command string.

## FP-11: Strict Parse Rejects Foreign Snapshots

**Pattern**: Reporting that manual snapshots and sibling subvolumes are
ignored.
**Rule**: Listings accept only `<volume>@<all-digits>`. A foreign object
that already occupies that exact path is the same trust boundary as the
zfs driver. Report only if a non-exact name is parsed, cleaned, or rolled
back.

## FP-12: Rollback Is Destructive

**Pattern**: Reporting that `zfs rollback -r` destroys newer snapshots.
**Rule**: That is the documented contract. Report only if btrfs rollback
deletes the live subvolume before the replacement snapshot exists, or if
the selected target is not the one `rollback_target` specifies.
