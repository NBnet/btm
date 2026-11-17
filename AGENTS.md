# Working on btm

BTM (Blockchain Time Machine) is a Linux snapshot library and `btm` CLI for
ZFS and Btrfs. It takes hot backups of a data volume at a monotonic index
(block height or any dense counter), lists them, rolls back, and cleans
them. There is no consensus engine here: the safety properties are shell
safety, snapshot identity, destructive-rollback ordering, and retention.

## AI workflows

This file is the shared agent entrypoint. Reusable workflows live in
`.agents/skills/` and require explicit invocation; ordinary requests do
not implicitly authorize commits:

- `$x-review`: deep regression review of a diff, N commits, a hash/range, or
  `all`. Updates [docs/audit.md](docs/audit.md); does not commit.
- `$x-fix`: resolve the `docs/audit.md` backlog, self-review, and commit.
- `$x-commit`: self-review uncommitted changes, fix, format, bump the patch
  version, and commit.
- `$x-overhaul`: full codebase audit → fix → commit pipeline.

Shared guides live under [docs/ai/](docs/ai/):
`technical-patterns.md` (cataloged snapshot-management bug patterns),
`review-core.md` (systematic review methodology),
`false-positive-guide.md` (rules for filtering spurious findings), and
`patterns/` (per-subsystem review guides for the public API, drivers, and
the daemon/CLI).

## Layout

```
src/lib.rs            # BtmCfg, SnapMode, SnapAlgo, public snapshot API
src/driver/mod.rs     # SnapDriver, retention (Fair/Fade), rollback target
src/driver/zfs.rs     # ZFS command strings and batched destroy
src/driver/btrfs.rs   # Btrfs command strings; rollback swap ordering
src/driver/external.rs
src/api/              # Unix-socket daemon protocol (Linux only)
src/bins/btm.rs       # CLI: list, rollback, clean, daemon
```

About 1.4K lines of Rust, including unit tests.

## Build & test

```bash
make fmt          # cargo fmt
make lint         # cargo fmt && cargo clippy
make test         # cargo test -- --test-threads=1
```

The crate root is `#![deny(warnings)]` and `#![deny(missing_docs)]`.
Snapshot orchestration needs Linux zfs/btrfs tooling. On other platforms
the library still compiles: `snapshot()` validates, warns once, and
returns `Ok(())`. Rollback, list, and clean are not platform-gated.
The non-Linux binary prints a warning and exits 0.

## Conventions

- No `#[allow(...)]`. Fix warnings at the source.
- No `unsafe`. This crate has none.
- Volume names reach a shell. `BtmCfg::validate_volume` /
  `validate_params` must run before any command string is built or
  executed, including paths that start from a struct literal.
- Public API changes must update the crate docs, `README.md`, and the
  matching guide under `docs/ai/patterns/`.
- Keep repository documentation, instructions, and commit messages in
  English. Respond to the user in their language.
