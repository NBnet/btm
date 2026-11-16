# Audit Findings

> Auto-managed by $x-review and $x-fix.

## Open

*(No open findings)*

---

## Won't Fix

### [LOW] driver: two rollbacks of the same volume share one temp name
- **Where**: `Btrfs::rollback_cmd` uses `<volume>.btm-rollback-tmp`
- **What**: Overlapping rollback calls can delete each other's temp subvolume.
- **Reason**: The library does not take a volume lock. Callers serialize snapshot and rollback on a volume. A unique temp name would still race with `mv` onto the live path.

## Resolved — Overhaul (2026-11-16)

### [MEDIUM] api: the snapshot client treated the first datagram as its own
- **Where**: `src/api/client/mod.rs`
- **What**: A response whose index did not match the request returned `snapshot failed` immediately, including a stale datagram still in the socket.
- **Resolution**: A mismatched index is ignored until the matching response arrives or the wait expires. A matching `success: false` is still an error.

Shell validation, exact `<volume>@<digits>` parsing, btrfs copy-before-delete, and Fair/Fade retention were re-checked and left unchanged.

---
