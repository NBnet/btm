//!
//! # btm binary
//!
//! ## Client Mode
//!
//! ```shell
//! btm --snapshot-volume <VOLUME> --snapshot-list
//! btm --snapshot-volume <VOLUME> --snapshot-rollback
//! btm --snapshot-volume <VOLUME> --snapshot-rollback-to <IDX>
//! btm --snapshot-volume <VOLUME> --snapshot-rollback-to-exact <IDX>
//! ```
//!
//! ## Server Mode
//!
//! ```shell
//! btm daemon \
//!         --snapshot-volume <VOLUME> \
//!         --snapshot-itv <ITV> \
//!         --snapshot-cap <CAP> \
//!         --snapshot-mode <MODE> \
//!         --snapshot-algo <ALGO> \
//! ```
//!

fn main() {
    cmd::run();
}

#[cfg(target_os = "linux")]
mod cmd {
    use btm::{run_daemon, BtmCfg, SnapAlgo, SnapMode, ENV_VAR_BTM_VOLUME, STEP_CNT};
    use clap::{arg, App, Arg, ArgMatches};
    use ruc::*;
    use std::{env, process::exit};

    pub(super) fn run() {
        pnk!(run_btm(parse_cmdline()).c(d!()))
    }

    fn run_btm(m: ArgMatches) -> Result<()> {
        let mut res = BtmCfg::new();

        if let Some(sub_m) = m.subcommand_matches("daemon") {
            // this field should be parsed at the top
            res.volume = sub_m
                .value_of("snapshot-volume")
                .c(d!())
                .map(|t| t.to_owned())
                .or_else(|e| env::var(ENV_VAR_BTM_VOLUME).c(d!(e)))?;

            res.itv = sub_m
                .value_of("snapshot-itv")
                .unwrap_or("10")
                .parse::<u64>()
                .c(d!())?;
            res.cap = sub_m
                .value_of("snapshot-cap")
                .unwrap_or("100")
                .parse::<u64>()
                .c(d!())?;

            if let Some(sm) = sub_m.value_of("snapshot-mode") {
                res.mode = SnapMode::from_string(sm).c(d!())?;
                if matches!(res.mode, SnapMode::External) {
                    return Err(eg!("Running `External` mode in `btm` is not allowed!"));
                }
            } else {
                res.mode = BtmCfg::guess_mode(&res.volume).c(d!())?;
            }

            if let Some(sa) = sub_m.value_of("snapshot-algo") {
                res.algo = SnapAlgo::from_string(sa).c(d!())?;
                res.itv.checked_pow(STEP_CNT as u32).c(d!())?;
            }

            run_daemon(res).c(d!())?;
        } else {
            // this field should be parsed at the top
            res.volume = m
                .value_of("snapshot-volume")
                .c(d!())
                .map(|t| t.to_owned())
                .or_else(|e| env::var(ENV_VAR_BTM_VOLUME).c(d!(e)))?;

            // the guess should always success in this scene
            res.mode = BtmCfg::guess_mode(&res.volume).c(d!())?;

            if m.is_present("snapshot-list") {
                list_snapshots(&res).c(d!())?;
            } else if m.is_present("snapshot-clean") {
                clean_snapshots(&res).c(d!())?;
            }

            check_rollback(&m, &res).c(d!())?;
        }

        Ok(())
    }

    fn list_snapshots(cfg: &BtmCfg) -> Result<()> {
        cfg.list_snapshots().c(d!())?;
        exit(0);
    }

    fn clean_snapshots(cfg: &BtmCfg) -> Result<()> {
        cfg.clean_snapshots().c(d!())?;
        exit(0);
    }

    fn check_rollback(m: &ArgMatches, cfg: &BtmCfg) -> Result<()> {
        const HINTS: &str = "NOTE: all related processes must be stopped before the rollback!";

        if m.is_present("snapshot-rollback")
            || m.is_present("snapshot-rollback-to")
            || m.is_present("snapshot-rollback-to-exact")
        {
            println!("\x1b[31;01m\n{}\x1b[00m", HINTS);

            let (h, strict) = m
                .value_of("snapshot-rollback-to-exact")
                .map(|h| (Some(h), true))
                .or_else(|| m.value_of("snapshot-rollback-to").map(|h| (Some(h), false)))
                .unwrap_or((None, false));
            let h = if let Some(h) = h {
                Some(h.parse::<u64>().c(d!())?)
            } else {
                None
            };
            cfg.rollback(h, strict).c(d!())?;

            exit(0);
        }

        Ok(())
    }

    fn parse_cmdline() -> ArgMatches {
        App::new("btm")
        .subcommand(
            App::new("daemon")
            .args(&[
                  arg!(-p --"snapshot-volume" [VolumePath] "a data volume containing both ledger data and tendermint data"),
                  arg!(-i --"snapshot-itv" [Iterval] "interval between adjacent snapshots, default to 10 blocks"),
                  arg!(-c --"snapshot-cap" [Capacity] "the maximum number of snapshots that will be stored, default to 100"),
                  arg!(-m --"snapshot-mode" [Mode] "zfs/btrfs/external, will try a guess if missing"),
                  arg!(-a --"snapshot-algo" [Algo] "fair/fade, default to `fair`"),
            ])
        )
        .args(&[
              arg!(-p --"snapshot-volume" [VolumePath] "a data volume containing both ledger data and tendermint data"),
              arg!(-l --"snapshot-list" "list all available snapshots in the form of block height"),
              arg!(-x --"snapshot-rollback" "rollback to the last available snapshot"),
              arg!(-r --"snapshot-rollback-to" [Height] "rollback to a custom height, will try the closest smaller height if the target does not exist"),
              arg!(-R --"snapshot-rollback-to-exact" [Height] "rollback to a custom height exactly, an error will be reported if the target does not exist"),
              arg!(-C --"snapshot-clean" "clean up all existing snapshots"),
        ])
        .arg(Arg::new("_a").long("ignored").hide(true))
        .arg(Arg::new("_b").long("nocapture").hide(true))
        .arg(Arg::new("_c").long("test-threads").hide(true))
        .arg(Arg::new("INPUT").multiple_occurrences(true).hide(true))
        .get_matches()
    }
}

#[cfg(not(target_os = "linux"))]
mod cmd {
    pub(super) fn run() {}
}
