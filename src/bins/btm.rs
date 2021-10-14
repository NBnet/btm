//!
//! # btm binary
//!
//! ## Client Mode
//!
//! ```shell
//! btm --snapshot-target <VOLUME> --snapshot-list
//! btm --snapshot-target <VOLUME> --snapshot-rollback
//! btm --snapshot-target <VOLUME> --snapshot-rollback-to <IDX>
//! btm --snapshot-target <VOLUME> --snapshot-rollback-to-exact <IDX>
//! ```
//!
//! ## Server Mode
//!
//! ```shell
//! btm daemon \
//!         --snapshot-target <VOLUME> \
//!         --snapshot-itv <ITV> \
//!         --snapshot-cap <CAP> \
//!         --snapshot-mode <MODE> \
//!         --snapshot-algo <ALGO> \
//! ```
//!

use btm::{run_daemon, BtmCfg, SnapAlgo, SnapMode, STEP_CNT};
use clap::{crate_authors, crate_description, App, Arg, ArgMatches, SubCommand};
use ruc::*;
use std::{env, process::exit};

fn main() {
    pnk!(run_btm(parse_cmdline()).c(d!()))
}

fn run_btm(m: ArgMatches) -> Result<()> {
    let mut res = BtmCfg::new();

    if let Some(sub_m) = m.subcommand_matches("daemon") {
        // this field should be parsed at the top
        res.target = sub_m
            .value_of("snapshot-target")
            .c(d!())
            .map(|t| t.to_owned())
            .or_else(|e| env::var("BTM_SNAPSHOT_TARGET").c(d!(e)))?;

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
            res.mode = res.guess_mode().c(d!())?;
        }

        if let Some(sa) = sub_m.value_of("snapshot-algo") {
            res.algo = SnapAlgo::from_string(sa).c(d!())?;
            res.itv.checked_pow(STEP_CNT as u32).c(d!())?;
        }

        run_daemon(res).c(d!())?;
    } else {
        // this field should be parsed at the top
        res.target = m
            .value_of("snapshot-target")
            .c(d!())
            .map(|t| t.to_owned())
            .or_else(|e| env::var("BTM_SNAPSHOT_TARGET").c(d!(e)))?;

        // the guess should always success in this scene
        res.mode = res.guess_mode().c(d!())?;

        if m.is_present("snapshot-list") {
            list_snapshots(&res).c(d!())?;
        }
        check_rollback(&m, &res).c(d!())?;
    }

    Ok(())
}

fn list_snapshots(cfg: &BtmCfg) -> Result<()> {
    println!("Available snapshots are listed below:");
    cfg.get_sorted_snapshots()
        .c(d!())?
        .into_iter()
        .rev()
        .for_each(|h| {
            println!("    {}", h);
        });
    exit(0);
}

fn check_rollback(m: &ArgMatches, cfg: &BtmCfg) -> Result<()> {
    const HINTS: &str = r#"    NOTE:
            before executing the rollback,
            all related processes must be exited,
            such as findorad, abcid, tendermint, etc.
        "#;

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

fn parse_cmdline() -> ArgMatches<'static> {
    App::new("btm")
        .about(crate_description!())
        .author(crate_authors!())
        .subcommand(
            SubCommand::with_name("daemon")
                .arg_from_usage("-p, --snapshot-target=[TargetPath] 'a data volume containing both ledger data and tendermint data'")
                .arg_from_usage("-i, --snapshot-itv=[Iterval] 'interval between adjacent snapshots, default to 10 blocks'")
                .arg_from_usage("-c, --snapshot-cap=[Capacity] 'the maximum number of snapshots that will be stored, default to 100'")
                .arg_from_usage("-m --snapshot-mode=[Mode] 'zfs/btrfs/external, will try a guess if missing'")
                .arg_from_usage("-a, --snapshot-algo=[Algo] 'fair/fade, default to `fair`'")
        )
        .arg_from_usage("-p, --snapshot-target=[TargetPath] 'a data volume containing both ledger data and tendermint data'")
        .arg_from_usage("-l, --snapshot-list 'list all available snapshots in the form of block height'")
        .arg_from_usage("-x, --snapshot-rollback 'rollback to the last available snapshot'")
        .arg_from_usage("-r, --snapshot-rollback-to=[Height] 'rollback to a custom height, will try the closest smaller height if the target does not exist'")
        .arg_from_usage("-R, --snapshot-rollback-to-exact=[Height] 'rollback to a custom height exactly, an error will be reported if the target does not exist'")
        .arg(Arg::with_name("_a").long("ignored").hidden(true))
        .arg(Arg::with_name("_b").long("nocapture").hidden(true))
        .arg(Arg::with_name("_c").long("test-threads").hidden(true))
        .arg(Arg::with_name("INPUT").multiple(true).hidden(true))
        .get_matches()
}
