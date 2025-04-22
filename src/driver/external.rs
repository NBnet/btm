//!
//! Only useful in client-end
//!

use crate::{BtmCfg, api::client};
use ruc::*;

#[inline(always)]
pub(crate) fn gen_snapshot(_cfg: &BtmCfg, idx: u64) -> Result<()> {
    client::request_snapshot(idx).c(d!())
}
