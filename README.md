![](https://tokei.rs/b1/github/FindoraNetwork/btm)
![GitHub top language](https://img.shields.io/github/languages/top/FindoraNetwork/btm)
![GitHub issues](https://img.shields.io/github/issues-raw/FindoraNetwork/btm)
![GitHub pull requests](https://img.shields.io/github/issues-pr-raw/FindoraNetwork/btm)

# btm

Blockchain Time Machine.

## User Instructions

**Usage of `btm ...`:**

```shell
btm
FindoraNetwork
Blockchain Time Machine

USAGE:
    btm [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help                 Prints help information
    -l, --snapshot-list        list all available snapshots in the form of block height
    -x, --snapshot-rollback    rollback to the last available snapshot
    -V, --version              Prints version information

OPTIONS:
    -r, --snapshot-rollback-to <Height>
            rollback to a custom height, will try the closest smaller height if the target does not exist

    -R, --snapshot-rollback-to-exact <Height>
            rollback to a custom height exactly, an error will be reported if the target does not exist

    -p, --snapshot-target <TargetPath>           a data volume containing both ledger data and tendermint data

SUBCOMMANDS:
    daemon
    help      Prints this message or the help of the given subcommand(s)
```

**Usage of `btm daemon ...`:**

```shell
btm-daemon

USAGE:
    btm daemon [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --snapshot-algo <Algo>            fair/fade, default to `fair`
    -c, --snapshot-cap <Capacity>         the maximum number of snapshots that will be stored, default to 100
    -i, --snapshot-itv <Iterval>          interval between adjacent snapshots, default to 10 blocks
    -m, --snapshot-mode <Mode>            zfs/btrfs/external, will try a guess if missing
    -p, --snapshot-target <TargetPath>    a data volume containing both ledger data and tendermint data
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
        --snapshot-target=zfs/data
```

**Outputs of `systemctl status btm-daemon.service`:**

```shell
● btm-daemon.service - "btm daemon"
     Loaded: loaded (/lib/systemd/system/btm-daemon.service; enabled; vendor preset: disabled)
     Active: active (running) since Tue 2021-10-12 21:23:09 CST; 19min ago
   Main PID: 374946 (btm)
      Tasks: 1 (limit: 38448)
        CPU: 781us
     CGroup: /system.slice/btm-daemon.service
             └─374946 /usr/local/bin/btm daemon -p=zfs/findora -i=10 -c=100 -m=zfs -a=fade
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
        --snapshot-target=<TARGET>

Example

    install.sh \
        --snapshot-itv=4 \
        --snapshot-cap=100 \
        --snapshot-mode=zfs \
        --snapshot-algo=fade \
        --snapshot-target=zfs/data

Example, short style

    install.sh -i=4 -c=100 -m=zfs -a=fade -t=zfs/data
```
