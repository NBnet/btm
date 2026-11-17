![GitHub top language](https://img.shields.io/github/languages/top/NBnet/btm)
[![Latest Version](https://img.shields.io/crates/v/btm.svg)](https://crates.io/crates/btm)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/btm)
[![Rust](https://github.com/NBnet/btm/actions/workflows/rust.yml/badge.svg)](https://github.com/NBnet/btm/actions/workflows/rust.yml)

# BTM

Blockchain Time Machine.

**NOTE: BTM is designed for Linux.** Off Linux the library still compiles. `BtmCfg::snapshot()` validates, warns once, and returns `Ok(())`. `rollback`, `list_snapshots`, and `clean_snapshots` are not platform-gated: ZFS and Btrfs still shell out, and `External` returns an error for those three. The `btm` binary on a non-Linux host prints a warning and exits 0. It does not run those commands.

BTM is an incremental data backup mechanism that does not require downtime.

## What's new in v3.0.0

- `snapshot()` prefers a scoped `syncfs`. ZFS falls back to a global `sync` when the mountpoint is missing or `syncfs` fails. Btrfs falls back when `syncfs` on the volume path fails. `External` always uses the global `sync` before the daemon snapshots
- snapshot listings are parsed strictly (`<volume>@<all-digits>`): manual snapshots, child datasets, and sibling subvolumes are never counted, cleaned, or rolled back
- `SnapMode::guess()` is read-only — the target volume is no longer created as a side effect of probing
- the crate compiles on non-Linux platforms (`snapshot()` is a no-op there). Rollback, list, and clean still shell out. The daemon/UAU API remains Linux-only
- `clean_snapshots()` goes through the snapshot driver (btrfs deletions are batched again)
- the full config (volume shell-safety + numeric params) is validated at every operation entry point, so struct-literal configs are as safe as `BtmCfg::new` ones; snapshot deletions are batched in groups of at most 64, with a per-snapshot fallback if a batch fails
- v3.0.0 recorded `ruc` 11 and `nix` 0.31. This crate is 3.0.2 and depends on `ruc` 11.0.3 and `nix` 0.32.0. Build with the current stable toolchain.

## Why would you need this?

btm will give you the following abilities or advantages:

- rollback to the state of a desired block height
- hot backup during operation, no downtime is needed
- based on OS-level infrastructure, stable and reliable
- very small resource usage, almost no performance damage
- ...

## Library Usages

```rust
use btm::{BtmCfg, SnapMode, SnapAlgo};

let cfg = BtmCfg {
    // itv = 1 snapshots every index. With itv > 1, only indexes aligned
    // to that interval (anchored at u64::MAX) create a snapshot; the
    // others return Ok(()) and do nothing.
    itv: 1,
    cap: 100,
    cap_clean_kept: 0,
    mode: SnapMode::Zfs,
    algo: SnapAlgo::Fade,
    volume: "zroot/data".to_owned(),
};

cfg.snapshot(0).unwrap();
cfg.snapshot(1).unwrap();
cfg.snapshot(11).unwrap();

// Print all existing snapshots.
cfg.list_snapshots().unwrap();

// Rollback to the state of the last snapshot.
cfg.rollback(None, false).unwrap();

// Rollback to the state of a custom snapshot.
cfg.rollback(Some(11), true).unwrap();
```

## Binary Usages

```
Usage: btm <COMMAND>

Commands:
  list      List all existing snapshots
  rollback  Rollback to the state of an existing snapshot
  clean     Clean all or part of existing snapshots
  daemon    Run btm as a daemon process
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

```
Usage: btm list [OPTIONS]

Options:
  -p, --volume <VOLUME>  The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted
  -h, --help             Print help
```

```
Usage: btm rollback [OPTIONS]

Options:
  -p, --volume <VOLUME>            The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted
  -s, --snapshot-id <SNAPSHOT_ID>  The target snapshot to rollback to, a negative value means the latest snapshot [default: -1]
  -S, --strict                     In this mode, if `snapshot_id` cannot be matched exactly, an error will be returned
  -h, --help                       Print help
```

```
Usage: btm clean [OPTIONS]

Options:
  -p, --volume <VOLUME>  The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted
  -k, --kept <KEPT>      How many snapshots should be kept [default: 0]
  -h, --help             Print help
```

```
Usage: btm daemon [OPTIONS]

Options:
  -p, --volume <VOLUME>  The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted
  -i, --itv <ITV>        The interval between two adjacent snapshots [default: 10]
  -c, --cap <CAP>        The maximum number of snapshots to keep, older snapshots will be cleaned up [default: 100]
  -m, --mode <MODE>      Optional, `zfs` or `btrfs`, case insensitive, will try to automatically identify if not specified
  -a, --algo <ALGO>      fair or fade, case insensitive [default: Fair]
  -h, --help             Print help
```

## Install as a 'systemd service'

**Steps:**

```shell
make
mv btm_package.tar.gz /tmp/
cd /tmp/
tar -xpf btm_package.tar.gz
cd btm_package

su # swith your user account to 'root'

./install.sh \
        --snapshot-itv=4 \
        --snapshot-cap=100 \
        --snapshot-mode=zfs \
        --snapshot-algo=fade \
        --snapshot-volume=zfs/data
```

**Outputs of `systemctl status btm-daemon.service`:**

```
● btm-daemon.service - "btm daemon"
     Loaded: loaded (/lib/systemd/system/btm-daemon.service; enabled; vendor preset: disabled)
     Active: active (running)
   Main PID: 334 (btm)
      Tasks: 1
     CGroup: /system.slice/btm-daemon.service
             └─334 /usr/local/bin/btm daemon -p=zfs/data -i=4 -c=100 -m=zfs -a=fade
```

**Usage of [tools/install.sh](./tools/install.sh):**

```
# tools/install.sh -h

Usage

    install.sh
        --snapshot-itv=<ITV>       optional, default 10
        --snapshot-cap=<CAP>       optional, default 100
        --snapshot-mode=<MODE>     required, zfs or btrfs
        --snapshot-algo=<ALGO>     optional, default fair
        --snapshot-volume=<VOLUME> required

Example

    install.sh \
        --snapshot-itv=4 \
        --snapshot-cap=100 \
        --snapshot-mode=zfs \
        --snapshot-algo=fair \
        --snapshot-volume=zfs/blockchain

Example, short style

    install.sh -i=4 -c=100 -m=zfs -a=fair -p=zfs/blockchain
    install.sh -i=4 -c=100 -m=btrfs -a=fair -p=/data/blockchain
```
