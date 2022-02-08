use crate::{BtmCfg, SnapAlgo, STEP_CNT};
use ruc::{cmd::exec_output, *};

#[inline(always)]
pub(crate) fn gen_snapshot(cfg: &BtmCfg, idx: u64) -> Result<()> {
    alt!(0 != (u64::MAX - idx) % cfg.itv as u64, return Ok(()));
    clean_outdated(cfg).c(d!())?;
    let cmd = format!(
        "
            zfs destroy {0}@{1} 2>/dev/null;
            zfs snapshot {0}@{1}
            ",
        &cfg.volume, idx
    );
    exec_output(&cmd).c(d!()).map(|_| ())
}

pub(crate) fn sorted_snapshots(cfg: &BtmCfg) -> Result<Vec<u64>> {
    let cmd = format!(
        r"zfs list -t snapshot {} | grep -o '@[0-9]\+' | sed 's/@//'",
        &cfg.volume
    );
    let output = exec_output(&cmd).c(d!())?;

    let mut res = output
        .lines()
        .map(|l| l.parse::<u64>().c(d!()))
        .collect::<Result<Vec<u64>>>()?;
    res.sort_unstable_by(|a, b| b.cmp(a));

    Ok(res)
}

pub(crate) fn rollback(cfg: &BtmCfg, idx: Option<u64>, strict: bool) -> Result<()> {
    // convert to AESC order for `binary_search`
    let mut snaps = sorted_snapshots(cfg).c(d!())?;
    // convert to AESC order for `binary_search`
    snaps.reverse();
    alt!(snaps.is_empty(), return Err(eg!("no snapshots")));

    let idx = idx.unwrap_or_else(|| snaps[snaps.len() - 1]);

    let cmd = match snaps.binary_search(&idx) {
        Ok(_) => {
            format!("zfs rollback -r {}@{}", &cfg.volume, idx)
        }
        Err(i) => {
            if strict {
                return Err(eg!("specified height does not exist"));
            } else {
                let effective_idx = if 1 + i > snaps.len() {
                    snaps[snaps.len() - 1]
                } else {
                    *(0..i)
                        .rev()
                        .find_map(|i| snaps.get(i))
                        .c(d!("no snapshots found"))?
                };
                format!("zfs rollback -r {}@{}", &cfg.volume, effective_idx)
            }
        }
    };

    exec_output(&cmd).c(d!()).map(|_| ())
}

#[inline(always)]
pub(crate) fn check(volume: &str) -> Result<()> {
    let cmd = format!("zfs list {0} || zfs create {0}", volume);
    exec_output(&cmd).c(d!()).map(|_| ())
}

#[inline(always)]
fn clean_outdated(cfg: &BtmCfg) -> Result<()> {
    match cfg.algo {
        SnapAlgo::Fair => clean_outdated_fair(cfg).c(d!()),
        SnapAlgo::Fade => clean_outdated_fade(cfg).c(d!()),
    }
}

fn clean_outdated_fair(cfg: &BtmCfg) -> Result<()> {
    let snaps = sorted_snapshots(cfg).c(d!())?;
    let cap = cfg.get_cap() as usize;

    if 1 + cap > snaps.len() {
        return Ok(());
    }

    snaps[cap..].iter().for_each(|i| {
        let cmd = format!("zfs destroy {}@{}", &cfg.volume, i);
        info_omit!(exec_output(&cmd));
    });

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
// 2. clean up snapshot whose indexs exceed `cap`
fn clean_outdated_fade(cfg: &BtmCfg) -> Result<()> {
    let snaps = sorted_snapshots(cfg).c(d!())?;
    let cap = cfg.get_cap() as usize;

    let chunk_size = cap / STEP_CNT;
    let chunk_denominators = (0..STEP_CNT as u32).map(|n| cfg.itv.pow(1 + n));

    if 1 + chunk_size > snaps.len() {
        return Ok(());
    }

    // 1.
    let mut pair = (&snaps[..0], &snaps[..]);
    for denominator in chunk_denominators {
        pair = if chunk_size < pair.1.len() {
            pair.1.split_at(chunk_size)
        } else {
            (pair.1, &[])
        };

        pair.0.iter().for_each(|n| {
            if 0 != (u64::MAX - n) % denominator as u64 {
                let cmd = format!("zfs destroy {}@{}", &cfg.volume, n);
                info_omit!(exec_output(&cmd));
            }
        });
    }

    // 2.
    if cap < snaps.len() {
        snaps[cap..].iter().for_each(|i| {
            let cmd = format!("zfs destroy {}@{}", &cfg.volume, i);
            info_omit!(exec_output(&cmd));
        });
    }

    Ok(())
}
