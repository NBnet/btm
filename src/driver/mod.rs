pub mod btrfs;
#[cfg(target_os = "linux")]
pub mod external;
pub mod zfs;

use crate::{BtmCfg, STEP_CNT, SnapAlgo};
use ruc::{cmd::exec, *};

/// Trait abstracting filesystem-specific snapshot commands.
/// ZFS and Btrfs each implement this trait; the shared orchestration
/// logic lives in the generic functions below.
pub(crate) trait SnapDriver {
    /// Shell command that lists snapshots of the target volume,
    /// one raw entry per line (parsed by `parse_snapshot_line`)
    fn list_snapshots_cmd(cfg: &BtmCfg) -> String;
    /// Extract the snapshot index from one line of `list_snapshots_cmd`
    /// output; `None` for entries that do not belong to btm
    /// (manual snapshots, other subvolumes, etc.)
    fn parse_snapshot_line(cfg: &BtmCfg, line: &str) -> Option<u64>;
    /// Shell command to create a snapshot at the given index
    fn create_snapshot_cmd(volume: &str, idx: u64) -> String;
    /// Shell command to rollback to the given snapshot index
    fn rollback_cmd(volume: &str, idx: u64) -> String;
    /// Shell command to destroy a single snapshot
    fn destroy_cmd(volume: &str, idx: u64) -> String;
    /// Read-only shell command that succeeds iff the volume exists
    /// and is managed by this driver; MUST have no side effects
    fn check_volume_cmd(volume: &str) -> String;

    /// Destroy multiple snapshots. Default implementation destroys one at a time.
    fn destroy_snapshots(volume: &str, indexes: &[u64]) {
        for idx in indexes {
            let cmd = Self::destroy_cmd(volume, *idx);
            info_omit!(exec(&cmd));
        }
    }
}

/// Return `true` if the index is aligned to the snapshot interval.
///
/// The counter is anchored at `u64::MAX` instead of zero so that
/// larger `itv` values keep firing on the same phase regardless of
/// where the index sequence started.
#[inline(always)]
fn idx_aligned(idx: u64, itv: u64) -> bool {
    (u64::MAX - idx).is_multiple_of(itv)
}

pub(crate) fn gen_snapshot<D: SnapDriver>(cfg: &BtmCfg, idx: u64) -> Result<()> {
    // degenerate configs must never reach a shell: itv = 0 would
    // silently skip every index, cap = 0 would destroy every existing
    // snapshot, and an unvalidated volume could smuggle shell syntax
    cfg.validate_params().c(d!())?;

    // cheap arithmetic gate next: skipped indexes must not pay
    // for a shell round-trip
    if !idx_aligned(idx, cfg.itv) {
        return Ok(());
    }

    let snaps = sorted_snapshots::<D>(cfg).c(d!())?;
    if snaps.contains(&idx) {
        return Err(eg!("Snapshot {} already exists!", idx));
    }

    clean_outdated::<D>(cfg, &snaps).c(d!())?;
    let cmd = D::create_snapshot_cmd(&cfg.volume, idx);
    exec(&cmd).c(d!()).map(|_| ())
}

pub(crate) fn sorted_snapshots<D: SnapDriver>(cfg: &BtmCfg) -> Result<Vec<u64>> {
    let cmd = D::list_snapshots_cmd(cfg);
    let output = exec(&cmd).c(d!())?;

    let mut res = output
        .lines()
        .filter_map(|l| D::parse_snapshot_line(cfg, l))
        .collect::<Vec<u64>>();
    res.sort_unstable_by(|a, b| b.cmp(a));
    res.dedup();

    Ok(res)
}

/// Select the snapshot to roll back to.
///
/// - `None` => the latest snapshot
/// - `Some(i)`, exact match => that snapshot
/// - `Some(i)`, no exact match, strict => error
/// - `Some(i)`, no exact match, lax => the newest snapshot older than `i`
pub(crate) fn rollback_target(snaps_asc: &[u64], idx: Option<u64>, strict: bool) -> Result<u64> {
    let last = match snaps_asc.last() {
        Some(l) => *l,
        None => return Err(eg!("no snapshots")),
    };

    let idx = match idx {
        Some(i) => i,
        None => return Ok(last),
    };

    match snaps_asc.binary_search(&idx) {
        Ok(_) => Ok(idx),
        Err(_) if strict => Err(eg!("specified height does not exist")),
        Err(0) => Err(eg!("all snapshots are newer than the requested height")),
        Err(i) => Ok(snaps_asc[i - 1]),
    }
}

pub(crate) fn rollback<D: SnapDriver>(cfg: &BtmCfg, idx: Option<i128>, strict: bool) -> Result<()> {
    // convert to ASC order for `binary_search`
    let mut snaps = sorted_snapshots::<D>(cfg).c(d!())?;
    snaps.reverse();

    let idx = match idx {
        Some(i) => Some(u64::try_from(i).c(d!("snapshot index must be non-negative"))?),
        None => None,
    };

    let target = rollback_target(&snaps, idx, strict).c(d!())?;
    let cmd = D::rollback_cmd(&cfg.volume, target);
    exec(&cmd).c(d!()).map(|_| ())
}

pub(crate) fn check<D: SnapDriver>(volume: &str) -> Result<()> {
    let cmd = D::check_volume_cmd(volume);
    exec(&cmd).c(d!()).map(|_| ())
}

/// Destroy all snapshots except the newest `kept` ones.
pub(crate) fn clean_all<D: SnapDriver>(cfg: &BtmCfg, kept: usize) -> Result<()> {
    let snaps = sorted_snapshots::<D>(cfg).c(d!())?;
    if kept < snaps.len() {
        D::destroy_snapshots(&cfg.volume, &snaps[kept..]);
    }
    Ok(())
}

#[inline(always)]
fn clean_outdated<D: SnapDriver>(cfg: &BtmCfg, snaps_desc: &[u64]) -> Result<()> {
    match cfg.algo {
        SnapAlgo::Fair => {
            let to_del = fair_to_delete(snaps_desc, cfg.get_cap() as usize);
            if !to_del.is_empty() {
                D::destroy_snapshots(&cfg.volume, to_del);
            }
        }
        SnapAlgo::Fade => {
            let to_del = fade_to_delete(snaps_desc, cfg.itv, cfg.get_cap() as usize);
            if !to_del.is_empty() {
                D::destroy_snapshots(&cfg.volume, &to_del);
            }
        }
    }
    Ok(())
}

/// Keep the newest `cap` snapshots, return the rest for deletion.
pub(crate) fn fair_to_delete(snaps_desc: &[u64], cap: usize) -> &[u64] {
    if cap < snaps_desc.len() {
        &snaps_desc[cap..]
    } else {
        &[]
    }
}

/// Fade retention: split the newest snapshots into `STEP_CNT` chunks of
/// `cap / STEP_CNT`; chunk `n` only keeps snapshots aligned to
/// `itv^(1 + n)`, so density decreases exponentially with age.
/// Everything beyond `cap` is deleted unconditionally.
///
/// # Example
///
/// - itv = 10
/// - cap = 100
/// - chunk_size = 100 / STEP_CNT = 10
///
/// index coverage = chunk_size * (itv^1 + itv^2 ... itv^STEP_CNT),
/// so 100 snapshots can cover ~10^11 indexes.
///
/// NOTE: callers must have validated `itv.pow(STEP_CNT)` against
/// overflow (see `BtmCfg::validate_params`). When `cap < STEP_CNT`,
/// `chunk_size` is zero and fade degenerates to the plain cap trim.
pub(crate) fn fade_to_delete(snaps_desc: &[u64], itv: u64, cap: usize) -> Vec<u64> {
    let chunk_size = cap / STEP_CNT;
    if 1 + chunk_size > snaps_desc.len() {
        return vec![];
    }

    let mut to_del = vec![];

    // 1. clean up unaligned snapshots in each chunk
    let mut pair: (&[u64], &[u64]) = (&snaps_desc[..0], snaps_desc);
    for denominator in (0..STEP_CNT as u32).map(|n| itv.pow(1 + n)) {
        pair = if chunk_size < pair.1.len() {
            pair.1.split_at(chunk_size)
        } else {
            (pair.1, &[])
        };

        pair.0.iter().for_each(|n| {
            if !idx_aligned(*n, denominator) {
                to_del.push(*n);
            }
        });
    }

    // 2. clean up snapshots whose positions exceed `cap`
    to_del.extend_from_slice(fair_to_delete(snaps_desc, cap));

    to_del
}

#[cfg(test)]
mod tests {
    use super::btrfs::Btrfs;
    use super::zfs::Zfs;
    use super::*;
    use crate::SnapMode;

    fn cfg_with_volume(volume: &str, mode: SnapMode) -> BtmCfg {
        BtmCfg {
            itv: 1,
            cap: 100,
            cap_clean_kept: 0,
            mode,
            algo: SnapAlgo::Fair,
            volume: volume.to_owned(),
        }
    }

    #[test]
    fn idx_alignment() {
        assert!(idx_aligned(u64::MAX, 10));
        assert!(idx_aligned(u64::MAX - 10, 10));
        assert!(!idx_aligned(u64::MAX - 5, 10));
        // itv = 1 accepts every index
        (0..100u64).for_each(|i| assert!(idx_aligned(i, 1)));
    }

    #[test]
    fn rollback_target_selection() {
        let snaps = [10u64, 20, 30];

        // empty set
        assert!(rollback_target(&[], None, false).is_err());

        // None => latest
        assert_eq!(30, rollback_target(&snaps, None, true).unwrap());

        // exact match
        assert_eq!(20, rollback_target(&snaps, Some(20), true).unwrap());

        // lax fallback: newest snapshot older than the request
        assert_eq!(20, rollback_target(&snaps, Some(25), false).unwrap());

        // lax fallback: request newer than everything => latest
        assert_eq!(30, rollback_target(&snaps, Some(99), false).unwrap());

        // strict miss
        assert!(rollback_target(&snaps, Some(25), true).is_err());

        // request older than everything: nothing to fall back to
        assert!(rollback_target(&snaps, Some(5), false).is_err());
    }

    #[test]
    fn fair_selection() {
        let snaps = [50u64, 40, 30, 20, 10];
        assert!(fair_to_delete(&snaps, 5).is_empty());
        assert!(fair_to_delete(&snaps, 9).is_empty());
        assert_eq!([20u64, 10].as_slice(), fair_to_delete(&snaps, 3));
        assert_eq!(snaps.as_slice(), fair_to_delete(&snaps, 0));
    }

    #[test]
    fn fade_selection() {
        // itv = 2, cap = 20 => chunk_size = 2, denominators 2, 4, 8, ...
        // u64::MAX is odd, so "aligned to 2" means odd indexes,
        // "aligned to 4" means idx % 4 == 3, and so on.
        let snaps: Vec<u64> = (0..=21u64).rev().collect();
        let mut to_del = fade_to_delete(&snaps, 2, 20);
        to_del.sort_unstable();
        let expected: Vec<u64> = (0..=20u64).filter(|n| ![15, 19].contains(n)).collect();
        assert_eq!(expected, to_del);

        // below one chunk of snapshots: nothing to clean
        assert!(fade_to_delete(&[5, 4], 2, 20).is_empty());

        // itv = 1 keeps every in-cap snapshot and trims the overflow
        let mut to_del = fade_to_delete(&snaps, 1, 20);
        to_del.sort_unstable();
        assert_eq!(vec![0u64, 1], to_del);
    }

    #[test]
    fn zfs_parse_snapshot_line() {
        let cfg = cfg_with_volume("tank/igp24", SnapMode::Zfs);
        let parse = |l| Zfs::parse_snapshot_line(&cfg, l);

        assert_eq!(Some(449), parse("tank/igp24@449"));
        assert_eq!(Some(0), parse("tank/igp24@0"));

        // manual snapshots must not become phantom numeric entries
        assert_eq!(None, parse("tank/igp24@20260706-predeploy"));
        assert_eq!(None, parse("tank/igp24@backup"));

        // child datasets are not ours
        assert_eq!(None, parse("tank/igp24/child@449"));

        // no sign prefixes, no overflow, no empty index
        assert_eq!(None, parse("tank/igp24@+449"));
        assert_eq!(None, parse("tank/igp24@99999999999999999999999999"));
        assert_eq!(None, parse("tank/igp24@"));
        assert_eq!(None, parse(""));
    }

    #[test]
    fn btrfs_parse_snapshot_line() {
        let cfg = cfg_with_volume("/btrfs/data", SnapMode::Btrfs);
        let parse = |l| Btrfs::parse_snapshot_line(&cfg, l);

        assert_eq!(Some(123), parse("ID 256 gen 30 top level 5 path data@123"));
        assert_eq!(
            Some(77),
            parse("ID 256 gen 30 top level 5 path nested/data@77")
        );

        // snapshots of sibling subvolumes must not match
        assert_eq!(None, parse("ID 256 gen 30 top level 5 path other@123"));

        // manual snapshots must not become phantom numeric entries
        assert_eq!(
            None,
            parse("ID 256 gen 30 top level 5 path data@2026-predeploy")
        );

        assert_eq!(None, parse("garbage"));
        assert_eq!(None, parse(""));
    }

    #[test]
    fn command_strings() {
        let cfg = cfg_with_volume("tank/data", SnapMode::Zfs);
        assert_eq!(
            "zfs list -H -t snapshot -d 1 -o name tank/data",
            Zfs::list_snapshots_cmd(&cfg)
        );
        assert_eq!(
            "zfs snapshot tank/data@7",
            Zfs::create_snapshot_cmd("tank/data", 7)
        );
        assert_eq!(
            "zfs rollback -r tank/data@7",
            Zfs::rollback_cmd("tank/data", 7)
        );
        assert_eq!("zfs destroy tank/data@7", Zfs::destroy_cmd("tank/data", 7));
        // the volume check must be read-only: no `create` fallback
        assert!(!Zfs::check_volume_cmd("tank/data").contains("create"));
        // batch destroy folds every index into one command
        assert_eq!(
            "zfs destroy tank/data@3,2,1",
            super::zfs::batch_destroy_cmd("tank/data", &[3, 2, 1])
        );

        let cfg = cfg_with_volume("/btrfs/data", SnapMode::Btrfs);
        assert_eq!(
            "btrfs subvolume list -so /btrfs",
            Btrfs::list_snapshots_cmd(&cfg)
        );
        assert!(!Btrfs::check_volume_cmd("/btrfs/data").contains("create"));

        // a bare relative volume name must probe the CWD, not "/"
        let cfg = cfg_with_volume("data", SnapMode::Btrfs);
        assert_eq!(
            "btrfs subvolume list -so .",
            Btrfs::list_snapshots_cmd(&cfg)
        );
    }

    #[test]
    fn degenerate_cfg_rejected_before_exec() {
        // these must all fail during validation, BEFORE any shell
        // command runs, so the test is safe on every platform

        // itv = 0 would silently skip every snapshot forever
        let mut cfg = cfg_with_volume("tank/data", SnapMode::Zfs);
        cfg.itv = 0;
        assert!(gen_snapshot::<Zfs>(&cfg, 42).is_err());

        // cap = 0 would mass-delete all existing snapshots
        let mut cfg = cfg_with_volume("tank/data", SnapMode::Zfs);
        cfg.cap = 0;
        assert!(gen_snapshot::<Zfs>(&cfg, 42).is_err());

        // a struct-literal config with a hostile volume must be
        // stopped even though BtmCfg::new was bypassed
        let cfg = cfg_with_volume("tank/data; rm -rf /", SnapMode::Zfs);
        assert!(gen_snapshot::<Zfs>(&cfg, 42).is_err());
        let cfg = cfg_with_volume("-o exec=evil", SnapMode::Btrfs);
        assert!(gen_snapshot::<Btrfs>(&cfg, 42).is_err());
    }

    #[test]
    fn fade_degenerates_below_step_cnt() {
        // cap < STEP_CNT => chunk_size = 0 => fade cannot thin chunks
        // and degenerates to the plain cap trim
        let snaps: Vec<u64> = (0..10u64).rev().collect();
        let mut to_del = fade_to_delete(&snaps, 2, 4);
        to_del.sort_unstable();
        assert_eq!(vec![0u64, 1, 2, 3, 4, 5], to_del);
    }
}
