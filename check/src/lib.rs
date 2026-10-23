//! `strata-check`: the Strata 2027 guarantee, as a crate.
//!
//! It depends on the facade with every mineral enabled and on Crust, so the
//! whole platform resolves in one graph. `cargo deny check` then proves the
//! graph holds one version of each vocabulary crate and each mineral, and the
//! tests in `tests/` prove values really cross mineral boundaries.
#![forbid(unsafe_code)]

pub use crust_assets;
pub use crust_core;
pub use strata;
