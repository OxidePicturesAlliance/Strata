# Strata 2027 — dependency audit

Audit date: 2026-10-23. Toolchains used: Rust 1.97.0 (local stable) and
Rust 1.96.1 (the highest MSRV declared by a participant). Every repository
was read at its default-branch `HEAD` on that date:

| Repository | Commit |
|---|---|
| mxpv/openusd | `f899419` (Crust locks `bb9058e`, see §3) |
| doubleailes/OpenSubdiv-rs | `da94679` |
| OxidePicturesAlliance/vdb | `6dbb632` |
| doubleailes/ptex-rs | `194ce5c` |
| doubleailes/openqmc-rs | `f695f95` |
| doubleailes/ocio-rs | `f5e2c83` |
| doubleailes/crust-render | `cd4f4e0` (first audited at `f958a80`, see §3) |

## 1. Minerals and Crust

| Mineral | Package | Lib (`use` name) | Latest (repo) | crates.io | Edition | MSRV (`rust-version`) | License |
|---|---|---|---|---|---|---|---|
| OpenUSD | `openusd` (+ `openusd-schemas`) | `openusd` | 0.8.0 (unreleased commits on `main`) | **0.8.0** | 2024 | 1.96 | MIT |
| OpenSubdiv | `opensubdiv-rs` | `opensubdiv_rs` | 0.5.0 | **0.5.0** | 2021 | 1.85 | MIT |
| OpenVDB | `vdb` | `vdb` | 0.1.0 | **no — name taken** (see below) | 2021 | *not declared* | MIT |
| Ptex | `ptex-rust` | `ptex` | 0.5.0 | **0.5.0** | 2021 | 1.85 | MIT |
| OpenQMC | `openqmc-rs` | `openqmc` | 0.2.4 | **0.2.4** | 2024 | 1.85 | Apache-2.0 |
| OpenColorIO (WIP) | `ocio` | `ocio` | 0.2.2 | **0.2.2** | 2021 | *not declared* | MIT AND BSD-3-Clause |
| Crust (consumer) | `crust-core`, `crust-assets`, … (workspace) | `crust_core`, … | 0.8.0 | no (`publish = false`, by design) | 2024 | 1.96 (cranelift) | MIT |

Notes:

- **`vdb` is git-only, and the crates.io name `vdb` belongs to an unrelated
  crate** (rvputten/vdb, v0.4.0, published 2019). The mineral cannot be
  published under its current package name. Either rename the package (e.g.
  `openvdb-rs`, keeping `[lib] name = "vdb"` so `use vdb::…` is unchanged) or
  obtain a name transfer — and then publish ≥ 0.5.0, since 0.1.0–0.4.0 are
  already taken by the old crate.
- `openusd` 0.8.0 on crates.io depends on `zip ^8.2`; `main` has moved to
  `zip 9`, and Crust depends on API only on `main` (§3). A 0.9 release is
  needed before Strata can use crates.io only.
- Package ≠ lib name for `opensubdiv-rs` (`opensubdiv_rs`), `ptex-rust`
  (`ptex`) and `openqmc-rs` (`openqmc`). The facade must name features after
  the mineral (`opensubdiv`, `ptex`, `openqmc`…) and rename dependencies with
  `package = "…"`.
- Mixed editions (2021/2024) are fine for interop; editions are per crate.
- `vdb` and `ocio` declare no `rust-version`; both build at 1.96, their real
  floor is unknown.

## 2. Shared crates in public APIs

Only crates whose **types appear in a mineral's public API** are vocabulary;
the rest are private implementation details and may duplicate freely.

| Crate | Who declares it (range) | In public API? | Evidence |
|---|---|---|---|
| `half` | openusd `2.3.1` (features `bytemuck`, `num-traits`, opt. `serde`) · vdb `2` · ocio `2` · Crust `2.7` | **yes** — openusd, vdb, ocio | `openusd::gf::f16` is `pub use half::f16`; `sdf::Value::Half(f16)`/`HalfVec(Vec<f16>)`. `vdb::Vec3h = Vec3<half::f16>`, `impl vdb::Value for half::f16`. `ocio::ops::lut1d::halfs_differ(f16, f16, …)` |
| `bytemuck` | openusd `1.14.0` (derive); transitively via `half` | **yes** — openusd | `openusd::gf` quaternion/vector types `derive(bytemuck::Pod, Zeroable)` |
| `num-traits` | openusd `0.2.17` · vdb `0.2` | **yes** — vdb (bounds) | `vdb::math::is_zero<T: num_traits::Zero>` and friends |
| `serde` | openusd `1` (optional `serde` feature) · Crust `1.0.229` | **yes** — openusd with `serde` | `sdf::Value` serialises; Crust's MCP session relies on it |
| `rayon` | vdb `1` · Crust `1.10.0` | **shared global pool** | not in signatures, but vdb tools and Crust's renderer run on the same global pool; two majors would mean two pools |
| `glam` | Crust `0.34.1` only | **no mineral uses it** | every mineral has its own math types (`openusd::gf::Vec3f`, `vdb::Vec3f`, …) |
| `flate2` | vdb `1.1.10` · ptex `1` · Crust `1.1` | no (private) | used internally for (de)compression only |
| `zip` | openusd 0.8.0 `^8.2` (main: `9.0.1`) · ocio `9` | no (private) | archive reading only |
| `lz4_flex` | openusd `0.14.0` · vdb `0.14.0` | no | |
| `thiserror` | openusd `2` · vdb `1` | no (derive only; error *types* are each crate's own) | |
| `exr`, `image`, `tiff` | Crust only | no mineral | |
| `wgpu` | nobody | — | |

## 3. Combined build

A scratch crate depending on every mineral plus `crust-core` and
`crust-assets` (manifest in the appendix).

### 3.1 Naïve combination (crates.io latest + Crust `main`) — **fails**

`cargo tree -d -e normal` duplicates:

| Duplicate | Cause | Vocabulary? |
|---|---|---|
| `ocio 0.1.1` + `ocio 0.2.2` | Crust at `f958a80` declared `ocio = "0.1.0"` | **mineral conflict** — fixed in Crust `cd4f4e0` the same day (`ocio = "0.2.0"`) |
| `opensubdiv-rs 0.5.0` (crates.io) + `opensubdiv-rs 0.5.0` (git tag `0.5.0`) | Crust depends on the git tag, not crates.io | **mineral conflict** — same version, different source ⇒ different types |
| `zip 2.6.1` + `8.6.0` + `9.0.5` | ocio 0.1 / openusd 0.8.0 (crates.io) / ocio 0.2 | private |
| `thiserror 1` + `2` | vdb | private |

and `cargo check` **fails to compile `crust-core`**: Crust uses openusd API
that only exists on `main` (`gf::Mat4d`, `Attribute::metadata`,
`CollectionAPI::membership_query`, …). Crust itself gets it through
`[patch.crates-io] openusd = { git, branch = "main" }`, which does not apply
to Crust's dependents. Following openusd `main` HEAD (`f899419`) also fails:
upstream has since changed `MaterialBindingAPI` (`collection_binding`,
`binding_strength`), so Crust only builds against the commit its lock records,
`bb9058e`.

### 3.2 With the proposed pins — **builds** (1.96.1 and 1.97.0)

With openusd patched to `bb9058e`, Crust pinned to `cd4f4e0`, and the
`opensubdiv-rs` git source patched back to crates.io 0.5.0, the remaining
duplicates are all private:

```
hashbrown 0.15.5 / 0.17.1    yaml-rust2 (ocio) vs indexmap (zip)
miniz_oxide 0.8.9 / 0.9.1    exr, png (Crust) vs flate2
syn 2 / 3                    proc-macros
thiserror 1 / 2              vdb vs openusd
```

One copy each of `half 2.7.1`, `bytemuck 1.25.2`, `num-traits 0.2.19`,
`serde 1.0.229`, `rayon 1.12.0`, `glam 0.34.1`, `flate2 1.1.10`, `zip 9.0.5`.
`cargo +1.96 check` and `cargo +1.97 check` both finish clean.

### 3.3 Update 2026-10-24 — openusd `68a867a`, Crust `5e5721e`

Crust `main` moved onto openusd's new `MaterialBindingAPI` (issue #2) and its
lock now records openusd `main` at `68a867a`; it no longer builds against
`bb9058e`. Strata moves both revs together: openusd patch and `test_ref` →
`68a867a`, Crust → `5e5721e` (Crust `main`). B3 is still open: Crust's
`strata-2027` branch (crates.io `opensubdiv-rs = "0.5.0"`, `ocio = "0.2.2"`)
is not merged into `main` yet, so the opensubdiv-rs patch stays.

## 4. Proposed pins (for approval)

Ranges are caret ranges whose floor is the highest floor any participant
declares (the lowest version known to satisfy everyone).

### Toolchain

| Item | Proposal | Why |
|---|---|---|
| MSRV (`rust-version`) | **1.96** | openusd and Crust (cranelift) both require it |
| CI toolchain | **1.96** (MSRV job) + `stable` | |
| Edition (Strata's own crates) | **2024** | |

### Vocabulary crates (`multiple-versions = "deny"`)

| Crate | Pin | Notes |
|---|---|---|
| `half` | `"2.7"` | features `bytemuck`, `num-traits` (openusd needs them; enabling in the facade is harmless) |
| `bytemuck` | `"1.14"` | |
| `num-traits` | `"0.2.19"` | |
| `serde` | `"1.0.229"` | Crust's floor; optional `serde` feature on the facade |
| `rayon` | `"1.10"` | one global thread pool |
| `glam` | `"0.34"` | **Crust-only today**; pinned so downstream renderers agree. Upstream suggestion (non-blocking): optional `glam` conversions in openusd/vdb |

### Watched, not denied (private today; listed in the spec, duplicates allowed)

`flate2 = "1.1"`, `zip = "9"`, `lz4_flex = "0.14"`, `exr = "1.74"`,
`image = "0.25"`. If a mineral ever exposes one of their types, it moves to
the vocabulary list.

### Minerals

| Mineral | Pin in the facade | Source for 2027.0.0-alpha.1 |
|---|---|---|
| openusd + openusd-schemas | `"0.8"` | **git `68a867a`** (was `bb9058e`, see §3.3) via `[patch.crates-io]` in the workspace (needed by Crust); the facade itself depends on crates.io `0.8` |
| opensubdiv-rs | `"0.5"` | crates.io |
| vdb | — | **git `6dbb632`** (`rev =`) — git-only |
| ptex-rust | `"0.5"` | crates.io |
| openqmc-rs | `"0.2.4"` | crates.io |
| ocio | `"0.2.2"` | crates.io (WIP) |
| Crust (check only) | git `5e5721e` (was `cd4f4e0`, see §3.3) | not published by design |

### C++ reference versions vs. VFX Reference Platform CY2027

CY2027 values were taken from vfxplatform.com on 2026-10-23 (screenshots
supplied by the maintainer; the site was unreachable from the audit sandbox).
Several CY2027 entries are still marked "release expected by" a date.

| Mineral | CY2027 target | Validated against (stated by the mineral) | Status |
|---|---|---|---|
| ocio | OpenColorIO **2.6.x** (release expected by Oct 1st) | between v2.5.0 and v2.6.0: ported to `93878f9`, targeting v2.6.0 (`UPSTREAM.md`) | close; finish the port to the v2.6.0 tag |
| opensubdiv-rs | OpenSubdiv **3.8.x** (release expected by Oct 1st) | *not stated* (ports the 3.x Sdc/Vtr/Far layers) | unknown; upstream must declare its version |
| vdb | OpenVDB **14.x** (release expected by Nov 1st) | OpenVDB **10.0.1** (test fixtures, benchmarks) | **lagging by 4 majors** |
| ptex-rust | Ptex **2.5.x** | *not stated* (wdas/ptex) | unknown; upstream must declare its version |
| openusd | *USD is not on the VFX Reference Platform list* | AOUSD Core Specification 1.0.1 (`docs/aousd_core_spec_1.0.1.pdf`) | reference = AOUSD Core Spec 1.0.1 |
| openqmc-rs | *OpenQMC is not on the VFX Reference Platform list* | *not stated* (bit-for-bit with upstream `main`) | reference = an upstream tag/commit, to be declared |

Related CY2027 rows that matter to Strata, though no mineral ports them yet:
OpenEXR 3.5.x (Crust uses the pure-Rust `exr` crate), Imath 3.2.x, ACES 2.0,
C++20, gcc 14.2 / glibc 2.34, macOS 14.0 minimum deployment target, Windows
SDK 10.0.22621 / VS build tools v14.44. The OS minimums inform the CI matrix
(Rust's own targets already meet them).

Proposal for the spec: each mineral carries **two** fields, `cpp_target` (the
CY2027 version) and `cpp_validated` (what its reference tests actually run
against). A mineral is *conformant* when the two match, and *lagging* when it
builds and passes tests against an older C++ version. For 2027.0.0-alpha.1
that makes vdb lagging, ocio near-conformant, and opensubdiv-rs, ptex-rust
and openqmc-rs undeclared.

## 5. Blocking items (upstream tasks)

Strata does not modify minerals; each item is the exact change to request.

### B0. Strata itself — the `strata` crate name is taken (resolved by renaming)

Found while bootstrapping (after the pins were approved): `strata` on
crates.io is an unrelated crate (shepmaster/strata 0.1.1, 2019). The facade is
published as **`strata-platform`** with `[lib] name = "strata"`, so code still
writes `use strata::…` and `#[serde(crate = "strata::serde")]`. If the name is
ever transferred, the package can be renamed without touching dependents'
code.

### B1. vdb — publish on crates.io (**resolved**)

Resolved 2026-10-24: vdb is published as `OpenVDB-rs` 0.1.0 (lib `OpenVDB_rs`,
from `88639ae`); the facade depends on it as `vdb = { package = "OpenVDB-rs",
version = "0.1" }`, so `strata::vdb` is unchanged.

**B1b (open).** The rename (`88639ae`, the published commit) also renamed the
lib to `OpenVDB_rs`, but vdb's own tests, benches and examples still write
`use vdb::…`, so its test suite does not compile at the published commit.
Until it is fixed, `test_ref` stays at `6dbb632` (identical source, old
package name). The fix, then publish 0.1.1:

```toml
# vdb/Cargo.toml
[lib]
name = "vdb"                 # keeps `use vdb::…` working, as originally requested
```

The original request:

The facade cannot be published while it has a git dependency, so
`crates/strata/Cargo.toml` is `publish = false` until this is done. Do not
work around it by adding `version = "0.1"` to the git dependency: `cargo
publish` would then resolve the unrelated crates.io `vdb` in its place.

```toml
# vdb/Cargo.toml
[package]
name = "openvdb-rs"          # `vdb` is taken on crates.io (rvputten/vdb 0.4.0)
version = "0.5.0"            # > 0.4.0 in case of a later name transfer
rust-version = "1.85"        # or whatever `cargo msrv` finds; must be <= 1.96
[lib]
name = "vdb"                 # keeps `use vdb::…` working
```

### B2. openusd — release 0.9 (**blocking** for Crust / strata-check without git patches)

Release `main` (≥ `68a867a`, with `zip = "9"`) as 0.9.0. Then Strata moves to
`openusd = "0.9"`, drops the `[patch]`, and Crust drops its own patch.

### B3. Crust — depend on opensubdiv-rs from crates.io (**blocking** for "no duplicate minerals")

```toml
# crust-render/crates/crust-core/Cargo.toml
-opensubdiv-rs = { git = "https://github.com/doubleailes/OpenSubdiv-rs", tag = "0.5.0" }
+opensubdiv-rs = "0.5.0"
```

Until then strata-check patches the git source back to crates.io.

### Non-blocking

- **vdb**: `thiserror = "1"` → `thiserror = "2"` (removes a duplicate; no API change for users).
- **vdb**, **ocio**: declare `rust-version` (≤ 1.96).
- **opensubdiv-rs**, **ptex-rust**, **openqmc-rs**: state the C++ release/commit
  they are validated against (README + a `package.metadata.strata.reference`
  key would let Strata check it automatically).
- **vdb**: validate against OpenVDB 14.x (CY2027) rather than 10.0.1.
- **opensubdiv-rs**: validate against OpenSubdiv 3.8.x; **ptex-rust**: against Ptex 2.5.x; **ocio**: finish the port to the OCIO v2.6.0 tag.
- **openusd**, **vdb**: optional `glam` feature with `From` conversions, so the
  platform's `glam` pin becomes a real interop point instead of Crust-only.

## Appendix — scratch manifest (§3.2)

```toml
[dependencies]
openusd = { version = "0.8", features = ["mmap", "serde"] }
openusd-schemas = "0.8"
opensubdiv-rs = "0.5"
vdb = { git = "https://github.com/OxidePicturesAlliance/vdb" }
ptex = { package = "ptex-rust", version = "0.5" }
openqmc = { package = "openqmc-rs", version = "0.2.4" }
ocio = "0.2.2"
crust-core = { git = "https://github.com/doubleailes/crust-render", rev = "cd4f4e0" }
crust-assets = { git = "https://github.com/doubleailes/crust-render", rev = "cd4f4e0" }

[patch.crates-io]
openusd = { git = "https://github.com/mxpv/openusd", rev = "bb9058e8d47d44f983caabf1e663401f59e3c7f0" }
openusd-schemas = { git = "https://github.com/mxpv/openusd", rev = "bb9058e8d47d44f983caabf1e663401f59e3c7f0" }

[patch."https://github.com/doubleailes/OpenSubdiv-rs"]
opensubdiv-rs = "0.5.0"
```
