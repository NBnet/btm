use super::SnapDriver;
use crate::BtmCfg;
use ruc::*;

pub(crate) struct Zfs;

/// `zfs destroy` accepts a comma-separated snapshot list after `@`,
/// so a whole batch collapses into a single command.
pub(crate) fn batch_destroy_cmd(volume: &str, indexes: &[u64]) -> String {
    let list = indexes
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("zfs destroy {}@{}", volume, list)
}

impl SnapDriver for Zfs {
    fn list_snapshots_cmd(cfg: &BtmCfg) -> String {
        // `-d 1` limits the scope to the dataset itself, keeping
        // snapshots of child datasets out of the result
        format!("zfs list -H -t snapshot -d 1 -o name {}", &cfg.volume)
    }

    /// Accept only `<volume>@<all-digits>`; anything else (manual
    /// snapshots, child datasets) belongs to someone else.
    fn parse_snapshot_line(cfg: &BtmCfg, line: &str) -> Option<u64> {
        super::parse_exact_snapshot(&cfg.volume, line)
    }

    fn create_snapshot_cmd(volume: &str, idx: u64) -> String {
        format!("zfs snapshot {}@{}", volume, idx)
    }

    fn rollback_cmd(volume: &str, idx: u64) -> String {
        format!("zfs rollback -r {}@{}", volume, idx)
    }

    fn destroy_cmd(volume: &str, idx: u64) -> String {
        format!("zfs destroy {}@{}", volume, idx)
    }

    fn check_volume_cmd(volume: &str) -> String {
        format!("zfs list {}", volume)
    }

    /// Zfs batches snapshot deletions (chunked, with per-item fallback).
    fn destroy_snapshots(volume: &str, indexes: &[u64]) -> Result<()> {
        super::destroy_batched::<Self>(volume, indexes, batch_destroy_cmd)
    }
}
