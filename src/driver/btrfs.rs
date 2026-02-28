use super::SnapDriver;
use crate::BtmCfg;
use ruc::{cmd::exec_output, *};
use std::path::PathBuf;

pub(crate) struct Btrfs;

impl SnapDriver for Btrfs {
    fn list_snapshots_cmd(cfg: &BtmCfg) -> Result<String> {
        let path = PathBuf::from(&cfg.volume);
        let parent = path.parent().c(d!())?.to_str().c(d!())?;
        Ok(format!(
            r"btrfs subvolume list -so {} | grep -o '@[0-9]\+$' | sed 's/@//'",
            parent
        ))
    }

    fn create_snapshot_cmd(volume: &str, idx: u64) -> String {
        format!(
            "btrfs subvolume delete {0}@{1} 2>/dev/null; btrfs subvolume snapshot {0} {0}@{1}",
            volume, idx
        )
    }

    fn rollback_cmd(volume: &str, idx: u64) -> String {
        format!(
            "btrfs subvolume delete {0} 2>/dev/null; btrfs subvolume snapshot {0}@{1} {0}",
            volume, idx
        )
    }

    fn destroy_cmd(volume: &str, idx: u64) -> String {
        format!("btrfs subvolume delete {}@{}", volume, idx)
    }

    fn check_volume_cmd(volume: &str) -> String {
        format!(
            "btrfs subvolume list {0} || btrfs subvolume create {0}",
            volume
        )
    }

    /// Btrfs batches all snapshot deletions into a single command.
    fn destroy_snapshots(volume: &str, indexes: &[u64]) {
        if indexes.is_empty() {
            return;
        }
        let list: String = indexes
            .iter()
            .map(|i| format!("{}@{}", volume, i))
            .collect::<Vec<_>>()
            .join(" ");
        let cmd = format!("btrfs subvolume delete {}", list);
        info_omit!(exec_output(&cmd));
    }
}

#[inline(always)]
pub(crate) fn gen_snapshot(cfg: &BtmCfg, idx: u64) -> Result<()> {
    super::gen_snapshot::<Btrfs>(cfg, idx)
}

pub(crate) fn sorted_snapshots(cfg: &BtmCfg) -> Result<Vec<u64>> {
    super::sorted_snapshots::<Btrfs>(cfg)
}

pub(crate) fn rollback(cfg: &BtmCfg, idx: Option<i128>, strict: bool) -> Result<()> {
    super::rollback::<Btrfs>(cfg, idx, strict)
}

#[inline(always)]
pub(crate) fn check(volume: &str) -> Result<()> {
    super::check::<Btrfs>(volume)
}
