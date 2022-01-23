use crate::{
    api::model::{Req, Resp, SERVER_US_ADDR},
    BtmCfg,
};
use once_cell::sync::Lazy;
use ruc::{
    uau::{SockAddr, UauSock},
    *,
};

static SERVER_PEER: Lazy<SockAddr> = Lazy::new(|| pnk!(UauSock::addr_to_sock(SERVER_US_ADDR)));

#[inline(always)]
pub(crate) fn request_snapshot(_cfg: &BtmCfg, idx: u64) -> Result<()> {
    // set receive timeout to 500ms, aka 0.5second
    let cli = UauSock::gen(Some(500)).c(d!())?;
    cli.send(&Req::new(idx).to_bytes(), &SERVER_PEER).c(d!())?;

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
