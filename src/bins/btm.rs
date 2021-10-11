//!
//! # btm binary
//!
//! ## Client Mode
//!
//! ```shell
//! btm --snapshot-target <VOLUME> --snapshot-list
//! btm --snapshot-target <VOLUME> --snapshot-rollback
//! btm --snapshot-target <VOLUME> --snapshot-rollback-to <IDX>
//! btm --snapshot-target <VOLUME> --snapshot-rollback-to-exact <IDX>
//! ```
//!
//! ## Server Mode
//!
//! ```shell
//! btm daemon \
//!         --snapshot-target <VOLUME> \
//!         --snapshot-itv <ITV> \
//!         --snapshot-cap <CAP> \
//!         --snapshot-mode <MODE> \
//!         --snapshot-algo <ALGO> \
//! ```
//!

fn main() {}
