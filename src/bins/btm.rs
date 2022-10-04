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
    use clap::{arg, Arg, ArgMatches, Command};
    use ruc::*;
    use std::{env, process::exit};

    pub(super) fn run() {
        pnk!(run_btm(parse_cmdline()).c(d!()))
    }

    fn run_btm(m: ArgMatches) -> Result<()> {
        let mut cfg = BtmCfg::new();

        if let Some(sub_m) = m.subcommand_matches("daemon") {
            // this field should be parsed first
            cfg.volume = sub_m
                .value_of("snapshot-volume")
                .c(d!("'--snapshot-volume' missing"))
                .map(|t| t.to_owned())
                .or_else(|e| env::var(ENV_VAR_BTM_VOLUME).c(d!("{}: {}", ENV_VAR_BTM_VOLUME, e)))?;

            cfg.itv = sub_m
                .value_of("snapshot-itv")
                .unwrap_or("10")
                .parse::<u64>()
                .c(d!())?;
            cfg.cap = sub_m
                .value_of("snapshot-cap")
                .unwrap_or("100")
                .parse::<u64>()
                .c(d!())?;

            if let Some(sm) = sub_m.value_of("snapshot-mode") {
                cfg.mode = SnapMode::from_string(sm).c(d!())?;
                if matches!(cfg.mode, SnapMode::External) {
                    return Err(eg!("Running `External` mode in `btm` is not allowed!"));
                }
            } else {
                cfg.mode = BtmCfg::guess_mode(&cfg.volume).c(d!())?;
            }

            if let Some(sa) = sub_m.value_of("snapshot-algo") {
                cfg.algo = SnapAlgo::from_string(sa).c(d!())?;
                cfg.itv.checked_pow(STEP_CNT as u32).c(d!())?;
            }

            run_daemon(cfg).c(d!())?;
        } else {
            // this field should be parsed first
            cfg.volume = m
                .value_of("snapshot-volume")
                .c(d!("'--snapshot-volume' missing"))
                .map(|t| t.to_owned())
                .or_else(|e| env::var(ENV_VAR_BTM_VOLUME).c(d!("{}: {}", ENV_VAR_BTM_VOLUME, e)))?;

            // the guess should always success in this scene
            cfg.mode = BtmCfg::guess_mode(&cfg.volume).c(d!())?;

            if m.is_present("snapshot-rollback")
                || m.is_present("snapshot-rollback-to")
                || m.is_present("snapshot-rollback-to-exact")
            {
                println!("\x1b[31;01m\nNOTE: all related processes must be stopped before the rollback!\x1b[00m");

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
            } else if m.is_present("snapshot-clean") {
                clean_snapshots(&cfg).c(d!())?;
            } else if let Some(n) = m.value_of("snapshot-clean-kept") {
                cfg.cap_clean_kept = n.parse::<usize>().c(d!())?;
                clean_snapshots(&cfg).c(d!())?;
            } else {
                // - if m.is_present("snapshot-list") {}
                // - default behavior
                list_snapshots(&cfg).c(d!())?;
            }
        }

        Ok(())
    }

    fn list_snapshots(cfg: &BtmCfg) -> Result<()> {
        cfg.list_snapshots().c(d!())?;
        exit(0);
    }

    fn clean_snapshots(cfg: &BtmCfg) -> Result<()> {
        cfg.clean_snapshots(cfg.cap_clean_kept).c(d!())?;
        exit(0);
    }

    fn parse_cmdline() -> ArgMatches {
        Command::new("btm")
        .subcommand(
            Command::new("daemon")
            .args(&[
                  arg!(-p --"snapshot-volume" [VolumePath] "a data volume containing your blockchain data"),
                  arg!(-i --"snapshot-itv" [Iterval] "interval between adjacent snapshots, default to 10 blocks"),
                  arg!(-c --"snapshot-cap" [Capacity] "the maximum number of snapshots that will be stored, default to 100"),
                  arg!(-m --"snapshot-mode" [Mode] "zfs/btrfs/external, will try a guess if missing"),
                  arg!(-a --"snapshot-algo" [Algo] "fair/fade, default to `fair`"),
            ])
        )
        .args(&[
              arg!(-p --"snapshot-volume" [VolumePath] "a data volume containing your blockchain data"),
              arg!(-l --"snapshot-list" "list all available snapshots in the form of block height"),
              arg!(-x --"snapshot-rollback" "rollback to the last available snapshot"),
              arg!(-r --"snapshot-rollback-to" [Height] "rollback to a custom height, will try the closest smaller height if the target does not exist"),
              arg!(-R --"snapshot-rollback-to-exact" [Height] "rollback to a custom height exactly, an error will be reported if the target does not exist"),
              arg!(-C --"snapshot-clean" "clean up all existing snapshots"),
              arg!(-K --"snapshot-clean-kept" [KeptNum] "clean up old snapshots out of kept capacity"),
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
