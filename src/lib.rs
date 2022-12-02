//!
//! # A Recover Mechanism for Blockchain Scene
//!
//! automatic operations:
//! - create a light-weight(COW) snapshot for each block
//! - clean up expired snapshots
//!

#![cfg(target_os = "linux")]
#![deny(warnings)]
#![deny(missing_docs)]

mod api;
mod driver;

pub use api::server::run_daemon;

use driver::{btrfs, external, zfs};
use ruc::{cmd, *};
use std::{fmt, result::Result as StdResult, str::FromStr};

/// Maximum number of snapshots that can be kept
pub const CAP_MAX: u64 = 4096;

/// `itv.pow(i)`, only useful in `SnapAlgo::Fade` alfo
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
    // Check mistakes
    fn check(&self) -> Result<()> {
        self.itv.checked_pow(STEP_CNT as u32).c(d!()).map(|_| ())
    }

    /// Create a simple instance
    #[inline(always)]
    pub fn new(volume: &str, mode: Option<&str>) -> Result<Self> {
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

    /// Generate a snapshot for the latest state of blockchain
    #[inline(always)]
    pub fn snapshot(&self, idx: u64) -> Result<()> {
        // sync data to disk before snapshoting
        nix::unistd::sync();

        match self.mode {
            SnapMode::Zfs => zfs::gen_snapshot(self, idx).c(d!()),
            SnapMode::Btrfs => btrfs::gen_snapshot(self, idx).c(d!()),
            SnapMode::External => external::gen_snapshot(self, idx).c(d!()),
        }
    }

    /// Rollback the state of blockchain to a specificed height
    #[inline(always)]
    pub fn rollback(&self, idx: Option<i128>, strict: bool) -> Result<()> {
        match self.mode {
            SnapMode::Zfs => zfs::rollback(self, idx, strict).c(d!()),
            SnapMode::Btrfs => btrfs::rollback(self, idx, strict).c(d!()),
            SnapMode::External => Err(eg!("please use the `btm` tool in `External` mode")),
        }
    }

    /// Get snapshot list in 'DESC' order.
    #[inline(always)]
    pub fn get_sorted_snapshots(&self) -> Result<Vec<u64>> {
        match self.mode {
            SnapMode::Zfs => zfs::sorted_snapshots(self).c(d!()),
            SnapMode::Btrfs => btrfs::sorted_snapshots(self).c(d!()),
            SnapMode::External => Err(eg!("please use `btm` tool in `External` mode")),
        }
    }

    #[inline(always)]
    fn get_cap(&self) -> u64 {
        alt!(self.cap > CAP_MAX, CAP_MAX, self.cap)
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

    /// Clean all existing snapshots.
    pub fn clean_snapshots(&self) -> Result<()> {
        self.get_sorted_snapshots().c(d!()).map(|list| {
            list.into_iter()
                .skip(self.cap_clean_kept)
                .rev()
                .for_each(|height| {
                    let cmd = match self.mode {
                        SnapMode::Btrfs => {
                            format!("btrfs subvolume delete {}@{}", &self.volume, height)
                        }
                        SnapMode::Zfs => format!("zfs destroy {}@{}", &self.volume, height),
                        _ => pnk!(Err(eg!("Unsupported deriver"))),
                    };
                    info_omit!(cmd::exec_output(&cmd));
                });
        })
    }
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
/// zfs destroy zfs/data@123456 2>/dev/null
/// zfs snapshot zfs/data@123456
///
/// # btrfs filesystem
/// rm -rf /btrfs/data@123456 2>/dev/null
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
    #[inline(always)]
    #[allow(missing_docs)]
    pub fn from_string(m: &str) -> Result<Self> {
        match m.to_lowercase().as_str() {
            "zfs" => Ok(Self::Zfs),
            "btrfs" => Ok(Self::Btrfs),
            "external" => Ok(Self::External),
            _ => Err(eg!()),
        }
    }

    /// Try to determine which mode can be used on the target volume
    ///
    /// NOTE:
    /// not suitable for the `External` mode.
    pub fn guess(volume: &str) -> Result<Self> {
        zfs::check(volume)
            .c(d!())
            .map(|_| SnapMode::Zfs)
            .or_else(|e| btrfs::check(volume).c(d!(e)).map(|_| SnapMode::Btrfs))
    }
}

impl Default for SnapMode {
    fn default() -> Self {
        Self::External
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
        Self::from_string(s).c(d!()).map_err(|e| e.to_string())
    }
}

/// Snapshot management algorithm
#[derive(Clone, Copy, Debug)]
pub enum SnapAlgo {
    /// snapshots are saved at fixed intervals
    Fair,
    /// snapshots are saved in decreasing density
    Fade,
}

impl SnapAlgo {
    #[inline(always)]
    #[allow(missing_docs)]
    pub fn from_string(m: &str) -> Result<Self> {
        match m.to_lowercase().as_str() {
            "fair" => Ok(Self::Fair),
            "fade" => Ok(Self::Fade),
            _ => Err(eg!()),
        }
    }
}

impl Default for SnapAlgo {
    fn default() -> Self {
        Self::Fair
    }
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
        Self::from_string(s).c(d!()).map_err(|e| e.to_string())
    }
}
