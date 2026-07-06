//!
//! Logic of `btm daemon ...`
//!

use crate::{
    BtmCfg,
    api::model::{Req, Resp, SERVER_US_ADDR},
};
use ruc::{uau::UauSock, *};

/// Run `btm daemon ...` server
pub fn run_daemon(cfg: BtmCfg) -> Result<()> {
    let s = pnk!(UauSock::new(SERVER_US_ADDR, None));
    loop {
        if let Ok((msg, peer)) = s.recv_buf::<128>()
            && let Ok(r) = info!(serde_json::from_slice::<Req>(&msg))
        {
            let success = info!(cfg.snapshot(r.idx())).is_ok();
            // a vanished client must not kill the daemon loop
            info_omit!(s.send(&Resp::new(r.idx(), success).to_bytes(), &peer));
        }
    }
}
