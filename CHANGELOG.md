# Change logs

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
