//!
//! # BTM
//!
//! Blockchain Time Machine.
//!
//! BTM is an incremental data backup mechanism that does not require downtime.
//!
//! - rollback to the state of a desired block height
//! - hot backup during operation, no downtime is needed
//! - based on OS-level infrastructure, stable and reliable
//! - very small resource usage, almost no performance damage
//!
//! ## Platform support
//!
//! Snapshot orchestration requires Linux (zfs/btrfs tooling). On other
//! platforms the crate still compiles so that cross-platform callers can
//! embed it unconditionally: [`BtmCfg::snapshot`] degrades to a no-op,
//! while destructive/query operations (rollback, list, clean) fail at
//! runtime instead of pretending to succeed.
//!
//! ## Index semantics for non-blockchain callers
//!
//! `idx` is just a monotonically increasing `u64` — block heights for
//! blockchains, but any dense counter works (e.g. minutes since an
//! epoch). Note that the `itv` alignment gate applies at snapshot
//! *creation* time: with `itv > 1`, only indexes aligned to the interval
//! (anchored at `u64::MAX`) produce a snapshot, so sparse counters like
//! raw unix timestamps should use `itv = 1` and control cadence at the
//! call site.
//!
//! ## Rollback semantics
//!
//! Rollback is destructive: the zfs driver uses `zfs rollback -r`, which
//! destroys every snapshot newer than the rollback target. To inspect a
//! snapshot without destroying history, clone it manually instead
//! (`zfs clone <volume>@<idx> <target>`).
//!

#![deny(warnings)]
#![deny(missing_docs)]

#[cfg(target_os = "linux")]
mod api;
mod driver;

#[cfg(target_os = "linux")]
pub use api::server::run_daemon;

#[cfg(target_os = "linux")]
use driver::external;
use driver::{btrfs::Btrfs, zfs::Zfs};
use ruc::*;
use std::{fmt, result::Result as StdResult, str::FromStr};

/// Maximum number of snapshots that can be kept
pub const CAP_MAX: u64 = 4096;

/// `itv.pow(i)`,
/// only useful within the `SnapAlgo::Fade` algo
pub const STEP_CNT: usize = 10;

/// Configures of snapshot mgmt
#[derive(Clone, Debug)]
pub struct BtmCfg {
    /// The interval between adjacent snapshots, default to 10 blocks
    pub itv: u64,
    /// The maximum number of snapshots that will be stored, default to 100
    pub cap: u64,
    /// How many snapshots should be kept after a `clean_snapshots`, default to 0
    pub cap_clean_kept: usize,
    /// Zfs or Btrfs or External, should try a guess if missing
    pub mode: SnapMode,
    /// Fair or Fade, default to 'Fair'
    pub algo: SnapAlgo,
    /// A data volume containing all blockchain data
    pub volume: String,
}

impl BtmCfg {
    /// Create a simple instance
    #[inline(always)]
    pub fn new(volume: &str, mode: Option<&str>) -> Result<Self> {
        Self::validate_volume(volume).c(d!())?;
        let mode = if let Some(m) = mode {
            SnapMode::from_str(m).map_err(|e| eg!(e))?
        } else {
            SnapMode::guess(volume).c(d!())?
        };
        Ok(Self {
            itv: 10,
            cap: 100,
            cap_clean_kept: 0,
            mode,
            algo: SnapAlgo::Fair,
            volume: volume.to_owned(),
        })
    }

    /// Validate volume name to prevent shell command injection.
    /// Only allows alphanumeric characters, `/`, `-`, `_`, and `.`;
    /// a leading `-` is rejected so the name can never be parsed as a
    /// command-line flag by the zfs/btrfs tools.
    fn validate_volume(volume: &str) -> Result<()> {
        if volume.is_empty() {
            return Err(eg!("volume name cannot be empty"));
        }
        if volume.starts_with('-') {
            return Err(eg!("volume name must not start with '-'"));
        }
        if !volume
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
        {
            return Err(eg!(
                "invalid volume name: only alphanumeric, '/', '-', '_', '.' are allowed"
            ));
        }
        Ok(())
    }

    /// Validate the whole configuration: the volume name (shell-safety)
    /// and the numeric parameters.
    ///
    /// Called automatically by every operation that may reach a shell,
    /// so a `BtmCfg` built via struct literal gets the same protection
    /// as one built via [`BtmCfg::new`].
    pub fn validate_params(&self) -> Result<()> {
        Self::validate_volume(&self.volume).c(d!())?;
        if self.itv < 1 {
            return Err(eg!("itv must be >= 1"));
        }
        if self.cap < 1 {
            return Err(eg!("cap must be >= 1"));
        }
        self.itv
            .checked_pow(STEP_CNT as u32)
            .c(d!("itv is too large, causes overflow"))?;
        Ok(())
    }

    /// Generate a snapshot for the latest state of the data volume.
    ///
    /// On non-Linux platforms this is a no-op: production deployments
    /// are Linux-only, and cross-platform callers must be able to keep
    /// this call in their hot path unconditionally.
    pub fn snapshot(&self, idx: u64) -> Result<()> {
        // the config must be proven shell-safe and non-degenerate
        // before anything else runs (sync_volume already shells out)
        self.validate_params().c(d!())?;

        if cfg!(not(target_os = "linux")) {
            static WARN_ONCE: std::sync::Once = std::sync::Once::new();
            WARN_ONCE.call_once(|| {
                eprintln!(
                    "btm: snapshots are only supported on Linux, `snapshot()` is a no-op on this platform"
                );
            });
            return Ok(());
        }

        // flush OS caches before snapshotting, so that everything
        // already written by the caller reaches the on-disk state
        // captured by the snapshot
        self.sync_volume();

        match self.mode {
            SnapMode::Zfs => driver::gen_snapshot::<Zfs>(self, idx).c(d!()),
            SnapMode::Btrfs => driver::gen_snapshot::<Btrfs>(self, idx).c(d!()),
            #[cfg(target_os = "linux")]
            SnapMode::External => external::gen_snapshot(self, idx).c(d!()),
            #[cfg(not(target_os = "linux"))]
            SnapMode::External => Err(eg!("`External` mode requires Linux")),
        }
    }

    /// Flush pending writes of the target volume to disk.
    ///
    /// Prefer `syncfs(2)` scoped to the volume's filesystem — a global
    /// `sync(2)` on a busy host stalls on every other filesystem's dirty
    /// pages. Fall back to the global sync when the volume's mountpoint
    /// cannot be resolved.
    fn sync_volume(&self) {
        #[cfg(target_os = "linux")]
        {
            let synced = match self.mode {
                SnapMode::Zfs => {
                    cmd::exec(&format!("zfs get -H -o value mountpoint {}", &self.volume))
                        .ok()
                        .map(|mp| mp.trim().to_owned())
                        .filter(|mp| mp.starts_with('/'))
                        .is_some_and(|mp| syncfs_path(&mp).is_ok())
                }
                SnapMode::Btrfs => syncfs_path(&self.volume).is_ok(),
                // the target volume is only known to the external daemon
                SnapMode::External => false,
            };
            if !synced {
                nix::unistd::sync();
            }
        }
    }

    /// Rollback the state of the data volume to a specified height.
    ///
    /// NOTE: destructive — snapshots newer than the target are destroyed
    /// (`zfs rollback -r` semantics).
    #[inline(always)]
    pub fn rollback(&self, idx: Option<i128>, strict: bool) -> Result<()> {
        self.validate_params().c(d!())?;
        match self.mode {
            SnapMode::Zfs => driver::rollback::<Zfs>(self, idx, strict).c(d!()),
            SnapMode::Btrfs => driver::rollback::<Btrfs>(self, idx, strict).c(d!()),
            SnapMode::External => Err(eg!("please use the `btm` tool in `External` mode")),
        }
    }

    /// Get snapshot list in 'DESC' order.
    #[inline(always)]
    pub fn get_sorted_snapshots(&self) -> Result<Vec<u64>> {
        self.validate_params().c(d!())?;
        match self.mode {
            SnapMode::Zfs => driver::sorted_snapshots::<Zfs>(self).c(d!()),
            SnapMode::Btrfs => driver::sorted_snapshots::<Btrfs>(self).c(d!()),
            SnapMode::External => Err(eg!("please use `btm` tool in `External` mode")),
        }
    }

    #[inline(always)]
    fn get_cap(&self) -> u64 {
        if self.cap > CAP_MAX {
            CAP_MAX
        } else {
            self.cap
        }
    }

    /// List all existing snapshots.
    pub fn list_snapshots(&self) -> Result<()> {
        println!("Available snapshots are listed below:");
        self.get_sorted_snapshots().c(d!()).map(|list| {
            list.into_iter().rev().for_each(|h| {
                println!("    {}", h);
            })
        })
    }

    /// Clean all existing snapshots except the newest
    /// `cap_clean_kept` ones.
    pub fn clean_snapshots(&self) -> Result<()> {
        self.validate_params().c(d!())?;
        match self.mode {
            SnapMode::Zfs => driver::clean_all::<Zfs>(self, self.cap_clean_kept).c(d!()),
            SnapMode::Btrfs => driver::clean_all::<Btrfs>(self, self.cap_clean_kept).c(d!()),
            SnapMode::External => Err(eg!(
                "Unsupported driver: External mode does not support clean_snapshots"
            )),
        }
    }
}

/// Flush the filesystem containing `path` via `syncfs(2)`.
#[cfg(target_os = "linux")]
fn syncfs_path(path: &str) -> Result<()> {
    let f = std::fs::File::open(path).c(d!())?;
    nix::unistd::syncfs(&f).c(d!())
}

/// # Inner Operations
///
/// assume:
/// - root volume of zfs is `zfs`
/// - root volume of btrfs is `/btrfs`
/// - business data is stored in `<root volume>/data`
/// - target block height to recover is 123456
///
/// ## snapshot
///
/// ```shell
/// # zfs filesystem
/// zfs snapshot zfs/data@123456
///
/// # btrfs filesystem
/// btrfs subvolume snapshot /btrfs/data /btrfs/data@123456
/// ```
///
/// ## rollback
///
/// ```shell
/// # zfs filesystem
/// zfs rollback -r zfs/data@123456
///
/// # btrfs filesystem
/// rm -rf /btrfs/data || exit 1
/// btrfs subvolume snapshot /btrfs/data@123456 /btrfs/data
/// ```
#[derive(Clone, Copy, Debug)]
pub enum SnapMode {
    /// Available on some Linux distributions and FreeBSD
    /// - Ubuntu Linux
    /// - Gentoo Linux
    /// - FreeBSD
    /// - ...
    Zfs,
    /// Available on most Linux distributions,
    /// but its user experience is worse than zfs
    Btrfs,
    /// Rely on an external independent process
    External,
}

impl SnapMode {
    /// Try to determine which mode can be used on the target volume.
    ///
    /// The probe is read-only: the volume must already exist, it will
    /// never be created as a side effect of guessing.
    ///
    /// NOTE:
    /// not suitable for the `External` mode.
    pub fn guess(volume: &str) -> Result<Self> {
        BtmCfg::validate_volume(volume).c(d!())?;
        driver::check::<Zfs>(volume)
            .c(d!())
            .map(|_| SnapMode::Zfs)
            .or_else(|e| {
                driver::check::<Btrfs>(volume)
                    .c(d!(e))
                    .map(|_| SnapMode::Btrfs)
            })
    }
}

impl fmt::Display for SnapMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = match self {
            Self::Zfs => "Zfs",
            Self::Btrfs => "Btrfs",
            Self::External => "External",
        };
        write!(f, "{}", contents)
    }
}

impl FromStr for SnapMode {
    type Err = String;
    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zfs" => Ok(Self::Zfs),
            "btrfs" => Ok(Self::Btrfs),
            "external" => Ok(Self::External),
            _ => Err(format!("unknown snap mode: '{}'", s)),
        }
    }
}

/// Snapshot management algorithm
#[derive(Clone, Copy, Debug, Default)]
pub enum SnapAlgo {
    /// snapshots are saved at fixed intervals
    #[default]
    Fair,
    /// snapshots are saved in decreasing density
    Fade,
}

impl fmt::Display for SnapAlgo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contents = match self {
            Self::Fair => "Fair",
            Self::Fade => "Fade",
        };
        write!(f, "{}", contents)
    }
}

impl FromStr for SnapAlgo {
    type Err = String;
    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fair" => Ok(Self::Fair),
            "fade" => Ok(Self::Fade),
            _ => Err(format!("unknown snap algo: '{}'", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_validation() {
        assert!(BtmCfg::validate_volume("tank/igp24-v1_data.0").is_ok());
        assert!(BtmCfg::validate_volume("/btrfs/data").is_ok());

        assert!(BtmCfg::validate_volume("").is_err());
        assert!(BtmCfg::validate_volume("-o").is_err());
        assert!(BtmCfg::validate_volume("-tank/data").is_err());
        assert!(BtmCfg::validate_volume("tank/data; rm -rf /").is_err());
        assert!(BtmCfg::validate_volume("tank/data$(reboot)").is_err());
        assert!(BtmCfg::validate_volume("tank/data\u{4e2d}").is_err());
    }

    #[test]
    fn params_validation() {
        let mut cfg = BtmCfg {
            itv: 1,
            cap: 100,
            cap_clean_kept: 0,
            mode: SnapMode::Zfs,
            algo: SnapAlgo::Fade,
            volume: "tank/data".to_owned(),
        };
        assert!(cfg.validate_params().is_ok());

        cfg.itv = 0;
        assert!(cfg.validate_params().is_err());

        // itv^STEP_CNT must not overflow u64
        cfg.itv = 90;
        assert!(cfg.validate_params().is_err());
        cfg.itv = 80;
        assert!(cfg.validate_params().is_ok());

        cfg.cap = 0;
        assert!(cfg.validate_params().is_err());
    }

    #[test]
    fn enums_from_str() {
        assert!(matches!("zfs".parse(), Ok(SnapMode::Zfs)));
        assert!(matches!("BTRFS".parse(), Ok(SnapMode::Btrfs)));
        assert!(matches!("External".parse(), Ok(SnapMode::External)));
        assert!("xfs".parse::<SnapMode>().is_err());

        assert!(matches!("fair".parse(), Ok(SnapAlgo::Fair)));
        assert!(matches!("Fade".parse(), Ok(SnapAlgo::Fade)));
        assert!("fibonacci".parse::<SnapAlgo>().is_err());
    }
}
