use super::SnapDriver;
use crate::BtmCfg;
use ruc::*;

pub(crate) struct Zfs;

impl SnapDriver for Zfs {
    fn list_snapshots_cmd(cfg: &BtmCfg) -> Result<String> {
        Ok(format!(
            r"zfs list -t snapshot -r {} | grep -o '@[0-9]\+' | sed 's/@//'",
            &cfg.volume
        ))
    }

    fn create_snapshot_cmd(volume: &str, idx: u64) -> String {
        format!(
            "zfs destroy {0}@{1} 2>/dev/null; zfs snapshot {0}@{1}",
            volume, idx
        )
    }

    fn rollback_cmd(volume: &str, idx: u64) -> String {
        format!("zfs rollback -r {}@{}", volume, idx)
    }

    fn destroy_cmd(volume: &str, idx: u64) -> String {
        format!("zfs destroy {}@{}", volume, idx)
    }

    fn check_volume_cmd(volume: &str) -> String {
        format!("zfs list -r {0} || zfs create {0}", volume)
    }
}

#[inline(always)]
pub(crate) fn gen_snapshot(cfg: &BtmCfg, idx: u64) -> Result<()> {
    super::gen_snapshot::<Zfs>(cfg, idx)
}

pub(crate) fn sorted_snapshots(cfg: &BtmCfg) -> Result<Vec<u64>> {
    super::sorted_snapshots::<Zfs>(cfg)
}

pub(crate) fn rollback(cfg: &BtmCfg, idx: Option<i128>, strict: bool) -> Result<()> {
    super::rollback::<Zfs>(cfg, idx, strict)
}

#[inline(always)]
pub(crate) fn check(volume: &str) -> Result<()> {
    super::check::<Zfs>(volume)
}
