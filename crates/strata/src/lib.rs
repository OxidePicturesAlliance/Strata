//! # Strata 2027
//!
//! The Oxide Pictures Alliance reference platform: one crate that pins the
//! shared "vocabulary" crates and the ported VFX libraries ("minerals") of a
//! given year, so that everything built on the same Strata year
//! interoperates.
//!
//! In Rust, `glam 0.33::Vec3` and `glam 0.34::Vec3` are different types, and
//! so are two copies of one mineral. Depending on Strata instead of on each
//! crate separately means your graph holds the versions the platform was
//! validated with, and a value produced by one mineral can be handed to
//! another.
//!
//! The platform spec lives in `platform/strata-2027.toml` in the repository,
//! rendered for humans in `platform/STRATA-2027.md`.
//!
//! ## Usage
//!
//! The package is `strata-platform` (the `strata` name is taken on
//! crates.io); the library is `strata`:
//!
//! ```toml
//! [dependencies]
//! strata-platform = { version = "2027.0.0-alpha.2", features = ["openusd", "vdb"] }
//! ```
//!
//! ```
//! # #[cfg(all(feature = "openusd", feature = "vdb"))]
//! # fn main() {
//! use strata::half::f16;
//! use strata::{openusd, vdb};
//!
//! // openusd's `gf::f16` *is* `strata::half::f16`, and so is the value type
//! // of a `vdb::Grid<f16>`: one type, no conversion.
//! let mut grid = vdb::Grid::<f16>::new(f16::ZERO);
//! grid.set_value(&vdb::Coord::new(0, 0, 0), openusd::gf::f16::ONE);
//! # }
//! # #[cfg(not(all(feature = "openusd", feature = "vdb")))]
//! # fn main() {}
//! ```
//!
//! ## What is re-exported
//!
//! Always, the vocabulary crates: [`bytemuck`], [`glam`], [`half`],
//! [`num_traits`], [`rayon`] and [`serde`].
//!
//! Behind a feature of the same name, each mineral (`full` enables all):
//!
//! | Feature | Re-export | Package |
//! |---|---|---|
//! | `openusd` | `strata::openusd`, `strata::openusd_schemas` (every schema family) | `openusd`, `openusd-schemas` |
//! | `opensubdiv` | `strata::opensubdiv` | `opensubdiv-rs` |
//! | `vdb` | `strata::vdb` | `OpenVDB-rs` |
//! | `ptex` | `strata::ptex` | `ptex-rust` |
//! | `openqmc` | `strata::openqmc` | `openqmc-rs` |
//! | `ocio` | `strata::ocio` | `ocio` |
//!
//! The `serde` feature turns on the serde impls of `half`, `glam` and (with
//! `openusd`) `openusd::sdf::Value`.
//!
//! ## Mineral features and derive macros
//!
//! Some things cannot go through a re-export. In both cases the fix is the
//! same: add the crate as a **direct** dependency at the version range in the
//! spec. Cargo unifies it with the one Strata pulls in, so there is still a
//! single copy (`cargo tree -d` stays empty).
//!
//! * **A mineral's own features**, e.g. openusd's `mmap`:
//!
//!   ```toml
//!   openusd = { version = "0.8", features = ["mmap"] }
//!   ```
//!
//! * **Derive macros that hard-code `::crate` paths.** Prefer the macro's
//!   crate-path attribute when it has one:
//!
//!   ```
//!   #[derive(Clone, Copy, strata::bytemuck::Pod, strata::bytemuck::Zeroable)]
//!   #[bytemuck(crate = "strata::bytemuck")]
//!   #[repr(C)]
//!   struct Vertex {
//!       position: [f32; 3],
//!   }
//!
//!   #[derive(strata::serde::Serialize, strata::serde::Deserialize)]
//!   #[serde(crate = "strata::serde")]
//!   struct Shot {
//!       name: String,
//!   }
//!   ```
//!
//!   Otherwise add the direct dependency (`serde = "1.0.229"`,
//!   `bytemuck = "1.14"`, …).
#![forbid(unsafe_code)]

pub use bytemuck;
pub use glam;
pub use half;
pub use num_traits;
pub use rayon;
pub use serde;

#[cfg(feature = "ocio")]
pub use ocio;
#[cfg(feature = "openqmc")]
pub use openqmc;
#[cfg(feature = "opensubdiv")]
pub use opensubdiv;
#[cfg(feature = "openusd")]
pub use openusd;
#[cfg(feature = "openusd")]
pub use openusd_schemas;
#[cfg(feature = "ptex")]
pub use ptex;
#[cfg(feature = "vdb")]
pub use vdb;

/// The Strata platform year this crate pins.
pub const YEAR: u16 = 2027;

#[cfg(test)]
mod tests {
    #[test]
    fn year_is_the_semver_major() {
        assert_eq!(env!("CARGO_PKG_VERSION_MAJOR"), super::YEAR.to_string());
    }

    // The derive paths the crate docs recommend must actually compile.
    #[derive(Clone, Copy, crate::bytemuck::Pod, crate::bytemuck::Zeroable)]
    #[bytemuck(crate = "crate::bytemuck")]
    #[repr(C)]
    struct Vertex {
        position: [f32; 3],
    }

    #[derive(crate::serde::Serialize, crate::serde::Deserialize)]
    #[serde(crate = "crate::serde")]
    struct Shot {
        name: String,
    }

    #[test]
    fn derives_through_the_reexports() {
        let v = Vertex {
            position: [1.0, 2.0, 3.0],
        };
        assert_eq!(crate::bytemuck::bytes_of(&v).len(), 12);
        let _ = Shot {
            name: String::from("sq010_sh0010"),
        }
        .name;
    }
}
