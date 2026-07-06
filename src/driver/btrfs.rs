use super::SnapDriver;
use crate::BtmCfg;
use ruc::{cmd::exec, *};
use std::path::Path;

pub(crate) struct Btrfs;

impl SnapDriver for Btrfs {
    fn list_snapshots_cmd(cfg: &BtmCfg) -> String {
        // `btrfs subvolume list` needs any path inside the filesystem;
        // the parent directory of the volume always qualifies
        let parent = Path::new(&cfg.volume)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/");
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
        format!(
            "btrfs subvolume delete {0} 2>/dev/null; btrfs subvolume snapshot {0}@{1} {0}",
            volume, idx
        )
    }

    fn destroy_cmd(volume: &str, idx: u64) -> String {
        format!("btrfs subvolume delete {}@{}", volume, idx)
    }

    fn check_volume_cmd(volume: &str) -> String {
        format!("btrfs subvolume show {}", volume)
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
        info_omit!(exec(&cmd));
    }
}
