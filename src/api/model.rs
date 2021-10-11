//!
//! # Data Model of API
//!

use ruc::*;
use serde::{Deserialize, Serialize};

/// SEE: [UAU](ruc::uau::UauSock)
pub(crate) const SERVER_US_ADDR: &[u8] = b"b1ce842e9f6e96d36287c8cfece722d";

#[derive(Deserialize, Serialize)]
pub struct Req {
    idx: u64,
}

impl Req {
    pub fn new(idx: u64) -> Self {
        Self { idx }
    }

    pub fn idx(&self) -> u64 {
        self.idx
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        pnk!(serde_json::to_vec(self))
    }
}

#[derive(Deserialize, Serialize)]
pub struct Resp {
    idx: u64,
    success: bool,
}

impl Resp {
    pub fn new(idx: u64, success: bool) -> Self {
        Self { idx, success }
    }

    pub fn idx(&self) -> u64 {
        self.idx
    }

    pub fn success(&self) -> bool {
        self.success
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        pnk!(serde_json::to_vec(self))
    }
}
