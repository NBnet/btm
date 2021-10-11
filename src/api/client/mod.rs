use crate::{
    api::model::{Req, Resp},
    api::SERVER_US,
    BtmCfg,
};
use ruc::{uau::UauSock, *};

#[inline(always)]
pub(crate) fn request_snapshot(_cfg: &BtmCfg, idx: u64) -> Result<()> {
    // set receive timeout to 500ms, aka 0.5second
    let cli = UauSock::gen(Some(500)).c(d!())?;
    cli.send(&Req::new(idx).to_bytes(), SERVER_US.addr())
        .c(d!())?;

    // try at most 10 times
    for _ in 0..10 {
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
