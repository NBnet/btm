use super::SnapDriver;
use crate::BtmCfg;
use ruc::*;

pub(crate) struct Btrfs;

impl SnapDriver for Btrfs {
    fn list_snapshots_cmd(cfg: &BtmCfg) -> String {
        // enumerate snapshots in the same namespace the mutation
        // commands use: they are created as filesystem-path siblings
        // (`<volume>@<idx>`), so glob that exact pattern instead of
        // translating `btrfs subvolume list` output, whose paths are
        // relative to the filesystem toplevel — a mount-dependent
        // namespace that cannot be matched reliably against the
        // configured volume path.
        //
        // the leading volume check keeps the error semantics: the
        // command fails when the volume is missing or not a btrfs
        // subvolume, and succeeds with empty output when there are
        // simply no snapshots yet (nullglob expands an unmatched
        // pattern to nothing); `validate_volume` guarantees the
        // volume contains no glob or shell metacharacters.
        format!(
            "{} >/dev/null && shopt -s nullglob && for s in {}@*; do printf '%s\\n' \"$s\"; done",
            Self::check_volume_cmd(&cfg.volume),
            &cfg.volume
        )
    }

    /// Lines are plain paths; accept only `<volume>@<all-digits>` —
    /// the exact paths `create_snapshot_cmd` produces. Snapshots of
    /// other subvolumes (even ones sharing the volume's basename) and
    /// manual snapshots are ignored.
    ///
    /// A foreign object that happens to live at `<volume>@<digits>`
    /// is indistinguishable from a btm snapshot and will be counted;
    /// this is the same trust boundary as the zfs driver. Rollback
    /// stays safe even then: snapshotting a non-subvolume fails and
    /// aborts the swap chain before the live volume is touched.
    fn parse_snapshot_line(cfg: &BtmCfg, line: &str) -> Option<u64> {
        super::parse_exact_snapshot(&cfg.volume, line)
    }

    fn create_snapshot_cmd(volume: &str, idx: u64) -> String {
        format!("btrfs subvolume snapshot {0} {0}@{1}", volume, idx)
    }

    fn rollback_cmd(volume: &str, idx: u64) -> String {
        // never destroy the live subvolume before its replacement is
        // secured: snapshot the backup to a temp name first, only then
        // swap it in (a vanished source snapshot aborts the chain with
        // the live volume intact); a stale temp from a previous failed
        // attempt is cleaned up front
        format!(
            "[ ! -d {0}.btm-rollback-tmp ] || btrfs subvolume delete {0}.btm-rollback-tmp \
             && btrfs subvolume snapshot {0}@{1} {0}.btm-rollback-tmp \
             && btrfs subvolume delete {0} \
             && mv {0}.btm-rollback-tmp {0}",
            volume, idx
        )
    }

    fn destroy_cmd(volume: &str, idx: u64) -> String {
        format!("btrfs subvolume delete {}@{}", volume, idx)
    }

    fn check_volume_cmd(volume: &str) -> String {
        format!("btrfs subvolume show {}", volume)
    }

    /// Btrfs batches snapshot deletions (chunked, with per-item fallback).
    fn destroy_snapshots(volume: &str, indexes: &[u64]) -> Result<()> {
        super::destroy_batched::<Self>(volume, indexes, batch_destroy_cmd)
    }
}

/// One `btrfs subvolume delete` accepts multiple subvolume paths.
pub(crate) fn batch_destroy_cmd(volume: &str, indexes: &[u64]) -> String {
    let list = indexes
        .iter()
        .map(|i| format!("{}@{}", volume, i))
        .collect::<Vec<_>>()
        .join(" ");
    format!("btrfs subvolume delete {}", list)
}
