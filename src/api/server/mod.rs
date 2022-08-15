//!
//! Logic of `btm daemon ...`
//!

use crate::{
    api::model::{Req, Resp, SERVER_US_ADDR},
    BtmCfg,
};
use ruc::{uau::UauSock, *};

/// Run `btm daemon ...` server
pub fn run_daemon(cfg: BtmCfg) -> Result<()> {
    let s = pnk!(UauSock::new(SERVER_US_ADDR, None));
    loop {
        if let Ok((msg, peer)) = s.recv_128() {
            if let Ok(r) = info!(serde_json::from_slice::<Req>(&msg)) {
                let success = info!(cfg.snapshot(r.idx())).is_ok();
                s.send(&Resp::new(r.idx(), success).to_bytes(), &peer)
                    .c(d!())?;
            }
        }
    }
}
