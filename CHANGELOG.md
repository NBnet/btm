# Change logs

#### v3.0.2

- The external snapshot client ignores a datagram for a different index instead of failing the request.
- Dependencies in this tree: `ruc` 11.0.3, `nix` 0.32.0. Build with the current stable toolchain.

#### v3.0.1

- Btrfs snapshot listing uses the configured volume path. Sibling subvolumes are not counted, cleaned, or rolled back.

#### v3.0.0

- `snapshot()` prefers a scoped `syncfs` and falls back to a global `sync` when that flush is skipped or fails
- Snapshot listings accept only `<volume>@<all-digits>`
- `SnapMode::guess()` no longer creates the target volume
- The library compiles off Linux: `snapshot()` is a no-op there. Rollback, list, and clean are not platform-gated
- `clean_snapshots()` goes through the snapshot driver; ZFS and Btrfs deletions are batched
- Volume shell-safety and numeric parameters are validated at every operation entry point
- Dependencies at that release: `ruc` 11.0, `nix` 0.31. The 3.0.2 tree uses `ruc` 11.0.3 and `nix` 0.32.0

#### v2.0.0

- Migrate to the 2024 edition
- Security: add volume name validation to prevent shell command injection
- Security: fix unsafe `i128` to `u64` cast with proper range checking
- Security: fix `clean_snapshots` panic on External mode, now returns `Result`
- Add configuration parameter validation (`itv >= 1`, `cap >= 1`)
- Introduce `SnapDriver` trait to eliminate code duplication between ZFS and Btrfs drivers
- Remove redundant `from_string()` methods, unify on `FromStr` trait
- Remove misleading `Default` impl for `SnapMode`
- Fix various comment typos

#### v0.12.0

- optimize command line expressions
