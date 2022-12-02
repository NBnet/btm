use crate::api::model::{Req, Resp, SERVER_US_ADDR};
use ruc::{uau::UauSock, *};

#[inline(always)]
pub(crate) fn request_snapshot(idx: u64) -> Result<()> {
    // set receive timeout to 500ms, aka 0.5second
    let cli = UauSock::gen(Some(500)).c(d!())?;
    cli.send(
        &Req::new(idx).to_bytes(),
        &pnk!(UauSock::addr_to_sock(SERVER_US_ADDR)),
    )
    .c(d!())?;

    // try at most 20 times, aka 10 seconds
    for _ in 0..20 {
        if let Ok(resp) = cli.recvonly_128() {
            let r = serde_json::from_slice::<Resp>(&resp).c(d!())?;
            if r.success() && r.idx() == idx {
                return Ok(());
            } else {
                return Err(eg!("snapshot failed"));
            }
        }
    }

    Err(eg!("snapshot failed"))
}
