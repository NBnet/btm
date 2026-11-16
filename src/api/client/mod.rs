use crate::api::model::{Req, Resp, SERVER_US_ADDR};
use ruc::{uau::UauSock, *};

#[inline(always)]
pub(crate) fn request_snapshot(idx: u64) -> Result<()> {
    // set receive timeout to 500ms, aka 0.5second
    let cli = UauSock::create(Some(500)).c(d!())?;
    cli.send(
        &Req::new(idx).to_bytes(),
        &pnk!(UauSock::addr_to_sock(SERVER_US_ADDR)),
    )
    .c(d!())?;

    // try at most 20 times, aka 10 seconds
    for _ in 0..20 {
        if let Ok(resp) = cli.recvonly_buf::<128>() {
            let r = serde_json::from_slice::<Resp>(&resp).c(d!())?;
            match classify_snapshot_resp(idx, &r) {
                SnapshotReply::Done => return Ok(()),
                SnapshotReply::Failed => return Err(eg!("snapshot failed")),
                SnapshotReply::Ignore => {}
            }
        }
    }

    Err(eg!("snapshot failed"))
}

enum SnapshotReply {
    Done,
    Failed,
    /// A datagram for a different index. Keep waiting.
    Ignore,
}

fn classify_snapshot_resp(expected: u64, resp: &Resp) -> SnapshotReply {
    if resp.idx() != expected {
        return SnapshotReply::Ignore;
    }
    if resp.success() {
        SnapshotReply::Done
    } else {
        SnapshotReply::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_response_is_not_this_request() {
        let other = Resp::new(1, false);
        assert!(matches!(
            classify_snapshot_resp(2, &other),
            SnapshotReply::Ignore
        ));
        let ok = Resp::new(2, true);
        assert!(matches!(
            classify_snapshot_resp(2, &ok),
            SnapshotReply::Done
        ));
        let failed = Resp::new(2, false);
        assert!(matches!(
            classify_snapshot_resp(2, &failed),
            SnapshotReply::Failed
        ));
    }
}
