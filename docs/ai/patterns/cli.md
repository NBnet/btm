# CLI and Daemon Review Patterns

## Files
- `src/bins/btm.rs` — `list`, `rollback`, `clean`, `daemon`
- `src/api/model.rs` — `Req` / `Resp` bytes, `SERVER_US_ADDR`
- `src/api/client/mod.rs` — `request_snapshot`
- `src/api/server/mod.rs` — `run_daemon`
- `tools/btm-daemon.service`, `tools/install.sh`

## Architecture
- The binary is optional (`feature = "bin"`). Volume comes from `--volume`
  or `$BTM_VOLUME`.
- `rollback --snapshot-id` defaults to `-1`, and any negative value is
  passed to the library as `None` (latest snapshot).
- `clean --kept` sets `cap_clean_kept` and calls `clean_snapshots`.
- `daemon` builds a `BtmCfg` from flags, rejects `SnapMode::External`,
  calls `validate_params`, and serves a Unix datagram socket.
- The client sends a JSON `Req` and the daemon replies with `Resp`.

## Critical Invariants

### INV-CLI1: Negative Id Means Latest, Not a Wrapped Index
**Check**: The comparison is `snapshot_id < 0`, and that arm passes
`None`. Zero and positive ids pass through as `Some`.

### INV-CLI2: Daemon Mode Cannot Be External
**Check**: `SnapMode::External` returns an error before `run_daemon`.
Guessed mode is still validated.

### INV-CLI3: One Bad Request Does Not Kill the Daemon
**Check**: `recv_buf`, JSON decode, `snapshot`, and `send` failures are
logged. The loop continues. `Resp.success` reflects `cfg.snapshot`.

### INV-CLI4: Protocol Bytes Stay in Lockstep
**Check**: `Req::to_bytes` / decode and `Resp` fields agree between
`client` and `server`. A change to `SERVER_US_ADDR` updates both.

## Common Bug Patterns

### Daemon Exit (technical-patterns.md 5.2)
`?` on `recv_buf` or `from_slice` breaks the loop.

### CLI / Library Index Split (technical-patterns.md 2.2)
The CLI starts passing negative `i128` straight into `rollback`, where
`try_from` rejects it, so `--snapshot-id -1` no longer means "latest".

## Review Checklist
- [ ] `$BTM_VOLUME` still fills in a missing `--volume`
- [ ] `clean --kept` reaches `cap_clean_kept`, not `cap`
- [ ] Daemon validates before it listens
- [ ] Install unit and socket path still match `SERVER_US_ADDR`
