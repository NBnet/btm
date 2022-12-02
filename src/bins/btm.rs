//!
//! # btm binary
//!
//! ## Client Mode
//!
//! ```shell
//! btm list --volume <VOLUME>
//! btm rollback --volume <VOLUME>
//! btm rollback --volume <VOLUME> --snapshot-id <IDX>
//! btm rollback --volume <VOLUME> --snapshot-id <IDX> --strict
//! btm clean
//! btm clean --kept 1
//! ```
//!
//! ## Server Mode
//!
//! ```shell
//! btm daemon \
//!         --volume <VOLUME> \
//!         --itv <ITV> \
//!         --cap <CAP> \
//!         --mode <MODE> \
//!         --algo <ALGO> \
//! ```
//!

#![deny(warnings)]
#![cfg(feature = "bin")]

fn main() {
    cmd::run();
}

#[cfg(target_os = "linux")]
mod cmd {
    use btm::{run_daemon, BtmCfg, SnapAlgo, SnapMode};
    use clap::{Parser, Subcommand};
    use ruc::*;
    use std::env;

    const ENV_VAR_BTM_VOLUME: &str = "BTM_VOLUME";

    #[derive(Parser)]
    #[clap(about, version, author)]
    struct Cfg {
        #[clap(subcommand)]
        cmds: Cmds,
    }

    #[derive(Debug, Subcommand)]
    enum Cmds {
        #[clap(about = "List all existing snapshots")]
        List {
            #[arg(
                short = 'p',
                long,
                help = "The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted"
            )]
            volume: Option<String>,
        },
        #[clap(about = "Rollback to the state of an existing snapshot")]
        Rollback {
            #[arg(
                short = 'p',
                long,
                help = "The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted"
            )]
            volume: Option<String>,
            #[arg(
                short,
                long,
                default_value_t = -1,
                help = "The target snapshot to rollback to, a negative value means the latest snapshot"
            )]
            snapshot_id: i128,
            #[arg(
                short = 'S',
                long,
                help = "In this mode, if `snapshot_id` cannot be matched exactly, an error will be returned"
            )]
            strict: bool,
        },
        #[clap(about = "Clean all or part of existing snapshots")]
        Clean {
            #[arg(
                short = 'p',
                long,
                help = "The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted"
            )]
            volume: Option<String>,
            #[arg(
                short,
                long,
                default_value_t = 0,
                help = "How many snapshots should be kept"
            )]
            kept: usize,
        },
        #[clap(about = "Run btm as a daemon process")]
        Daemon {
            #[arg(
                short = 'p',
                long,
                help = "The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted"
            )]
            volume: Option<String>,
            #[arg(
                short,
                long,
                default_value_t = 10,
                help = "The interval between two adjacent snapshots"
            )]
            itv: u64,
            #[arg(
                short,
                long,
                default_value_t = 100,
                help = "The maximum number of snapshots to keep, older snapshots will be cleaned up"
            )]
            cap: u64,
            #[arg(
                short,
                long,
                help = "Optional, `zfs` or `btrfs`, case insensitive, will try to automatically identify if not specified"
            )]
            mode: Option<String>,
            #[arg(short, long, default_value_t = String::from("Fair"), help = "fair or fade, case insensitive")]
            algo: String,
        },
    }

    pub(super) fn run() {
        pnk!(run_btm());
    }

    fn run_btm() -> Result<()> {
        let cfg = Cfg::parse();

        match cfg.cmds {
            Cmds::List { volume } => volume
                .c(d!())
                .or_else(|_| env::var(ENV_VAR_BTM_VOLUME).c(d!()))
                .and_then(|v| BtmCfg::new(&v, None).c(d!()))?
                .list_snapshots()
                .c(d!()),
            Cmds::Rollback {
                volume,
                snapshot_id,
                strict,
            } => volume
                .c(d!())
                .or_else(|_| env::var(ENV_VAR_BTM_VOLUME).c(d!()))
                .and_then(|v| BtmCfg::new(&v, None).c(d!()))?
                .rollback(alt!(0 > snapshot_id, None, Some(snapshot_id)), strict)
                .c(d!()),
            Cmds::Clean { volume, kept } => volume
                .c(d!())
                .or_else(|_| env::var(ENV_VAR_BTM_VOLUME).c(d!()))
                .and_then(|v| clean_snapshots(&v, kept).c(d!())),
            Cmds::Daemon {
                volume,
                itv,
                cap,
                mode,
                algo,
            } => {
                let volume = volume
                    .c(d!())
                    .or_else(|_| env::var(ENV_VAR_BTM_VOLUME).c(d!()))?;
                let mode = if let Some(m) = mode {
                    let m = SnapMode::from_string(&m).c(d!())?;
                    if matches!(m, SnapMode::External) {
                        return Err(eg!("`External` mode is not allowed in `btm` binary!"));
                    }
                    m
                } else {
                    SnapMode::guess(&volume).c(d!())?
                };

                let algo = SnapAlgo::from_string(&algo).c(d!())?;

                let btmcfg = BtmCfg {
                    itv,
                    cap,
                    // useless in this scene
                    cap_clean_kept: 0,
                    mode,
                    algo,
                    volume,
                };
                run_daemon(btmcfg).c(d!())
            }
        }
    }

    fn clean_snapshots(volume: &str, kept: usize) -> Result<()> {
        let mut cfg = BtmCfg::new(volume, None).c(d!())?;
        cfg.cap_clean_kept = kept;
        cfg.clean_snapshots().c(d!())
    }
}

#[cfg(not(target_os = "linux"))]
mod cmd {
    pub(super) fn run() {}
}
