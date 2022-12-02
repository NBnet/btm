![GitHub top language](https://img.shields.io/github/languages/top/ccmlm/btm)
[![Latest Version](https://img.shields.io/crates/v/btm.svg)](https://crates.io/crates/btm)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/btm)
![GitHub Workflow Status](https://img.shields.io/github/workflow/status/ccmlm/btm/Rust)
[![Minimum rustc version](https://img.shields.io/badge/rustc-1.63+-lightgray.svg)](https://github.com/rust-random/rand#rust-version-requirements)

# BTM

Blockchain Time Machine.

BTM is an incremental data backup mechanism that does not require downtime.

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
    itv: 10,
    cap: 100,
    mode: SnapMode::Zfs,
    algo: SnapAlgo::Fade,
    volume: "zroot/data".to_owned(),
};

// Generate snapshots in some threads.
cfg.snapshot(0).unwrap();
cfg.snapshot(1).unwrap();
cfg.snapshot(11).unwrap();

/// Print all existing snapshots.
cfg.list_snapshots();

/// Rollback to the state of the last snapshot.
cfg.rollback(None, false).unwrap();

/// Rollback to the state of a custom snapshot.
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
  -h, --help     Print help information
  -V, --version  Print version information
```

```
Usage: btm list [OPTIONS]

Options:
  -p, --volume <VOLUME>  The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted
  -h, --help             Print help information
```

```
Usage: btm rollback [OPTIONS]

Options:
  -p, --volume <VOLUME>            The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted
  -s, --snapshot-id <SNAPSHOT_ID>  The target snapshot to rollback to, a negative value means the latest snapshot [default: -1]
  -S, --strict                     In this mode, if `snapshot_id` cannot be matched exactly, an error will be returned
  -h, --help                       Print help information
```

```
Usage: btm clean [OPTIONS]

Options:
  -p, --volume <VOLUME>  The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted
  -k, --kept <KEPT>      How many snapshots should be kept [default: 0]
  -h, --help             Print help information
```

```
Usage: btm daemon [OPTIONS]

Options:
  -p, --volume <VOLUME>  The target volume to operate on, if $BTM_VOLUME is specified, this option can be omitted
  -i, --itv <ITV>        The interval between two adjacent snapshots [default: 10]
  -c, --cap <CAP>        The maximum number of snapshots to keep, older snapshots will be cleaned up [default: 100]
  -m, --mode <MODE>      Optional, `zfs` or `btrfs`, case insensitive, will try to automatically identify if not specified
  -a, --algo <ALGO>      fair or fade, case insensitive [default: Fair]
  -h, --help             Print help information
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
     Active: active (running) since Tue 2021-10-12 21:24:16 CST; 2min 27s ago
   Main PID: 334 (btm)
      Tasks: 1 (limit: 37805)
        CPU: 1ms
     CGroup: /system.slice/btm-daemon.service
             └─334 /usr/local/bin/btm daemon -p=/data -i=4 -c=100 -m=btrfs -a=fade
```

**Usage of [tools/install.sh](./tools/install.sh):**

```
# tools/install.sh -h

Usage

    install.sh
        --snapshot-itv=<ITV>
        --snapshot-cap=<CAP>
        --snapshot-mode=<MODE>
        --snapshot-algo=<ALGO>
        --snapshot-volume=<VOLUME>

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
