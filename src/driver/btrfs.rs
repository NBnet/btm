use super::SnapDriver;
use crate::BtmCfg;
use ruc::*;
use std::path::Path;

pub(crate) struct Btrfs;

impl SnapDriver for Btrfs {
    fn list_snapshots_cmd(cfg: &BtmCfg) -> String {
        // `btrfs subvolume list` needs any path inside the filesystem;
        // the parent directory of the volume always qualifies (fall
        // back to the CWD for a bare relative name)
        let parent = Path::new(&cfg.volume)
            .parent()
            .and_then(|p| p.to_str())
            .filter(|p| !p.is_empty())
            .unwrap_or(".");
        format!("btrfs subvolume list -so {}", parent)
    }

    /// Lines look like `ID 256 gen 30 top level 5 path <subvol>@<idx>`.
    /// Accept only entries whose subvolume basename matches the target
    /// volume's basename and whose index is all digits; snapshots of
    /// sibling subvolumes and manual snapshots are ignored.
    fn parse_snapshot_line(cfg: &BtmCfg, line: &str) -> Option<u64> {
        let path = line.rsplit_once(" path ")?.1.trim();
        let (subvol, idx) = path.rsplit_once('@')?;
        let vol_base = Path::new(&cfg.volume).file_name()?.to_str()?;
        let sub_base = subvol.rsplit('/').next().unwrap_or(subvol);
        if sub_base != vol_base {
            return None;
        }
        if idx.is_empty() || !idx.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        idx.parse().ok()
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
