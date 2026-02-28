pub mod btrfs;
pub mod external;
pub mod zfs;

use crate::{BtmCfg, STEP_CNT, SnapAlgo};
use ruc::{cmd::exec_output, *};

/// Trait abstracting filesystem-specific snapshot commands.
/// ZFS and Btrfs each implement this trait; the shared orchestration
/// logic lives in the generic functions below.
pub(crate) trait SnapDriver {
    /// Shell command that lists snapshot identifiers (one per line)
    fn list_snapshots_cmd(cfg: &BtmCfg) -> Result<String>;
    /// Shell command to create a snapshot at the given index
    fn create_snapshot_cmd(volume: &str, idx: u64) -> String;
    /// Shell command to rollback to the given snapshot index
    fn rollback_cmd(volume: &str, idx: u64) -> String;
    /// Shell command to destroy a single snapshot
    fn destroy_cmd(volume: &str, idx: u64) -> String;
    /// Shell command to check/create a volume
    fn check_volume_cmd(volume: &str) -> String;

    /// Destroy multiple snapshots. Default implementation destroys one at a time.
    fn destroy_snapshots(volume: &str, indexes: &[u64]) {
        for idx in indexes {
            let cmd = Self::destroy_cmd(volume, *idx);
            info_omit!(exec_output(&cmd));
        }
    }
}

#[inline(always)]
pub(crate) fn gen_snapshot<D: SnapDriver>(cfg: &BtmCfg, idx: u64) -> Result<()> {
    if sorted_snapshots::<D>(cfg).c(d!())?.contains(&idx) {
        return Err(eg!("Snapshot {} already exists!", idx));
    }

    alt!(0 != (u64::MAX - idx) % cfg.itv, return Ok(()));
    clean_outdated::<D>(cfg).c(d!())?;
    let cmd = D::create_snapshot_cmd(&cfg.volume, idx);
    exec_output(&cmd).c(d!()).map(|_| ())
}

pub(crate) fn sorted_snapshots<D: SnapDriver>(cfg: &BtmCfg) -> Result<Vec<u64>> {
    let cmd = D::list_snapshots_cmd(cfg).c(d!())?;
    let output = exec_output(&cmd).c(d!())?;

    let mut res = output
        .lines()
        .map(|l| l.parse::<u64>().c(d!()))
        .collect::<Result<Vec<u64>>>()?;
    res.sort_unstable_by(|a, b| b.cmp(a));

    Ok(res)
}

pub(crate) fn rollback<D: SnapDriver>(cfg: &BtmCfg, idx: Option<i128>, strict: bool) -> Result<()> {
    // convert to ASC order for `binary_search`
    let mut snaps = sorted_snapshots::<D>(cfg).c(d!())?;
    snaps.reverse();
    alt!(snaps.is_empty(), return Err(eg!("no snapshots")));

    let idx = match idx {
        Some(i) => u64::try_from(i).c(d!("snapshot index must be non-negative"))?,
        None => snaps[snaps.len() - 1],
    };

    let cmd = match snaps.binary_search(&idx) {
        Ok(_) => D::rollback_cmd(&cfg.volume, idx),
        Err(i) => {
            if strict {
                return Err(eg!("specified height does not exist"));
            }
            let effective_idx = if 1 + i > snaps.len() {
                snaps[snaps.len() - 1]
            } else {
                *(0..i)
                    .rev()
                    .find_map(|i| snaps.get(i))
                    .c(d!("no snapshots found"))?
            };
            D::rollback_cmd(&cfg.volume, effective_idx)
        }
    };

    exec_output(&cmd).c(d!()).map(|_| ())
}

pub(crate) fn check<D: SnapDriver>(volume: &str) -> Result<()> {
    let cmd = D::check_volume_cmd(volume);
    exec_output(&cmd).c(d!()).map(|_| ())
}

#[inline(always)]
fn clean_outdated<D: SnapDriver>(cfg: &BtmCfg) -> Result<()> {
    match cfg.algo {
        SnapAlgo::Fair => clean_outdated_fair::<D>(cfg).c(d!()),
        SnapAlgo::Fade => clean_outdated_fade::<D>(cfg).c(d!()),
    }
}

fn clean_outdated_fair<D: SnapDriver>(cfg: &BtmCfg) -> Result<()> {
    let snaps = sorted_snapshots::<D>(cfg).c(d!())?;
    let cap = cfg.get_cap() as usize;

    if 1 + cap > snaps.len() {
        return Ok(());
    }

    D::destroy_snapshots(&cfg.volume, &snaps[cap..]);

    Ok(())
}

// Logical steps:
//
// 1. clean up outdated snapshot in each chunks
// > # Example
// > - itv = 10
// > - cap = 100
// > - step_cnt = 5
// > - chunk_size = 100 / 5 = 20
// >
// > blocks cover = chunk_size * (itv^1 + itv^2 ... itv^step_cnt)
// >              = 55_5500
// >
// > this means we can use 100 snapshots to cover 55_5500 blocks
//
// 2. clean up snapshot whose indexes exceed `cap`
fn clean_outdated_fade<D: SnapDriver>(cfg: &BtmCfg) -> Result<()> {
    let snaps = sorted_snapshots::<D>(cfg).c(d!())?;
    let cap = cfg.get_cap() as usize;

    cfg.validate_params().c(d!())?;
    let chunk_size = cap / STEP_CNT;
    let chunk_denominators = (0..STEP_CNT as u32).map(|n| cfg.itv.pow(1 + n));

    if 1 + chunk_size > snaps.len() {
        return Ok(());
    }

    let mut to_del = vec![];

    // 1.
    let mut pair = (&snaps[..0], &snaps[..]);
    for denominator in chunk_denominators {
        pair = if chunk_size < pair.1.len() {
            pair.1.split_at(chunk_size)
        } else {
            (pair.1, &[])
        };

        pair.0.iter().for_each(|n| {
            if 0 != (u64::MAX - n) % denominator {
                to_del.push(*n);
            }
        });
    }

    // 2.
    if cap < snaps.len() {
        to_del.extend_from_slice(&snaps[cap..]);
    }

    if !to_del.is_empty() {
        D::destroy_snapshots(&cfg.volume, &to_del);
    }

    Ok(())
}
