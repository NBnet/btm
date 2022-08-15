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
    enable: true,
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

**Usage of `btm ...`:**

```shell
btm

USAGE:
    btm [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -C, --snapshot-clean
            clean up all existing snapshots

    -h, --help
            Print help information

    -l, --snapshot-list
            list all available snapshots in the form of block height

    -p, --snapshot-volume [<VolumePath>...]
            a data volume containing your blockchain data

    -r, --snapshot-rollback-to [<Height>...]
            rollback to a custom height, will try the closest smaller height if the target does not
            exist

    -R, --snapshot-rollback-to-exact [<Height>...]
            rollback to a custom height exactly, an error will be reported if the target does not
            exist

    -x, --snapshot-rollback
            rollback to the last available snapshot
```

**Usage of `btm daemon ...`:**

```shell
btm-daemon

USAGE:
    btm daemon [OPTIONS]

OPTIONS:
    -a, --snapshot-algo [<Algo>...]
            fair/fade, default to `fair`

    -c, --snapshot-cap [<Capacity>...]
            the maximum number of snapshots that will be stored, default to 100

    -h, --help
            Print help information

    -i, --snapshot-itv [<Iterval>...]
            interval between adjacent snapshots, default to 10 blocks

    -m, --snapshot-mode [<Mode>...]
            zfs/btrfs/external, will try a guess if missing

    -p, --snapshot-volume [<VolumePath>...]
            a data volume containing your blockchain data
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

```shell
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

```shell
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
