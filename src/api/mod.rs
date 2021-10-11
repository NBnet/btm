//!
//! # Logic of External Mode
//!

pub(crate) mod client;
mod model;
pub mod server;

use lazy_static::lazy_static;
use ruc::{uau::UauSock, *};

lazy_static! {
    static ref SERVER_US: UauSock = pnk!(UauSock::new(model::SERVER_US_ADDR, None));
}
