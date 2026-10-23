# Strata

**The Oxide Pictures Alliance reference platform for pure-Rust VFX and animation libraries.**

Strata is to the Rust VFX ecosystem what the [VFX Reference Platform](https://vfxplatform.com)
is to the C++ one: a yearly list of library versions that studios and tool authors build
against. Strata enforces the list with tooling instead of only publishing it.

Each year pins:

- the **vocabulary crates**: crates whose types cross library boundaries (`half`, `glam`,
  `bytemuck`…). In Rust, `glam 0.33::Vec3` and `glam 0.34::Vec3` are different types, so two
  libraries that disagree on a version cannot pass data to each other;
- the **minerals**: the pure-Rust ports of the VFX libraries (OpenUSD, OpenSubdiv, OpenVDB,
  Ptex, OpenQMC, OpenColorIO), each with the C++ version it is validated against.

The current platform is **Strata 2027** (draft): see [`platform/STRATA-2027.md`](platform/STRATA-2027.md).
The machine-readable spec, [`platform/strata-2027.toml`](platform/strata-2027.toml), is the source
of truth; [`platform/AUDIT.md`](platform/AUDIT.md) records how the pins were chosen.

## Using it

```toml
[dependencies]
strata-platform = { git = "https://github.com/OxidePicturesAlliance/strata", features = ["openusd", "vdb"] }
```

```rust
use strata::half::f16;
use strata::{openusd, vdb};

// openusd's `gf::f16` is `strata::half::f16`, which is what vdb stores.
let mut grid = vdb::Grid::<f16>::new(f16::ZERO);
grid.set_value(&vdb::Coord::new(0, 0, 0), openusd::gf::f16::ONE);
```

- The package is `strata-platform` and the library is `strata`: the `strata` name is taken on
  crates.io (by an unrelated crate from 2019).
- The vocabulary crates are always re-exported: `strata::{bytemuck, glam, half, num_traits, rayon, serde}`.
- Each mineral sits behind a feature of the same name (`openusd`, `opensubdiv`, `vdb`, `ptex`,
  `openqmc`, `ocio`), and `full` enables all of them. `serde` turns on the vocabulary types'
  serde impls.
- Derive macros and mineral features: see the crate docs (`cargo doc -p strata-platform --open`).
  Use `#[bytemuck(crate = "strata::bytemuck")]` / `#[serde(crate = "strata::serde")]`, or add the
  crate as a direct dependency at the spec's range. Cargo unifies it, so there is still one copy.

## ⚠️ Not on crates.io yet

A crate published on crates.io cannot have git dependencies. Strata 2027.0.0-alpha.1 has some,
so the facade is `publish = false` and is used from git for now. These items block publishing:

| Blocker | Item |
|---|---|
| **vdb** | git-only; the crates.io name `vdb` belongs to an unrelated crate. **Publish on crates.io** under a new package name (lib name `vdb`). |
| **openusd** | Crust needs unreleased API, so strata-check patches openusd to a `main` commit. **Release 0.9 on crates.io.** (The facade itself would work with crates.io 0.8.) |
| **Crust** | depends on opensubdiv-rs by git tag, which is a second copy of the crate; strata-check patches it back to crates.io. **Depend on the crates.io release.** |

The exact `Cargo.toml` change for each one is in [`platform/AUDIT.md`](platform/AUDIT.md#5-blocking-items-upstream-tasks).

## Strata 2027 conformance

A mineral is **Strata 2027 conformant** when all of these hold:

1. **It builds against the pins.** It resolves and compiles in one graph with every other
   mineral and the reference consumer (Crust), at the platform toolchain (Rust 1.96), on
   Linux, macOS and Windows (`strata-check`).
2. **There are no duplicate vocabulary crates.** That graph holds exactly one version of each
   vocabulary crate and of each mineral (`cargo deny check bans`).
3. **It passes its own reference tests against the declared C++ version.** Its own test suite
   passes with its vocabulary crates moved to the exact versions Strata resolved
   (`cargo xtask test-mineral`), and the C++ version it validates against is the CY2027 one.

A mineral meeting 1 and 2, whose tests pass against an *older* C++ version, is **lagging**.
One that does not declare its C++ version is **undeclared**. A downstream crate is conformant
when it depends on the platform through the facade (or at the spec's ranges) and its graph
passes 1 and 2.

Every CI run publishes the result as a table (in the run summary, and as the `strata-status`
artifact containing `STATUS.md` and `status.json`, ready for a future dashboard):

Status of 2027.0.0-alpha.1 from a local run on Linux, Rust 1.96, 2026-10-23 (CI regenerates it on every run):

| Mineral | Version | On crates.io | Builds with pins | One copy of each vocabulary crate | Own tests at pins | C++ target (CY2027) | Validated against | Verdict |
|---|---|---|---|---|---|---|---|---|
| `ocio` | 0.2.2 | ✅ | ✅ | ✅ | ✅ | OpenColorIO 2.6.x | 2.5.0+ (ported to 93878f9, targeting 2.6.0) | 🟡 lagging |
| `openqmc` | 0.2.4 | ✅ | ✅ | ✅ | ✅ | OpenQMC (no version pinned) | — | ⚪ undeclared |
| `opensubdiv` | 0.5 | ✅ | ✅ | ✅ | ✅ | OpenSubdiv 3.8.x | — | ⚪ undeclared |
| `openusd` | 0.8 | ✅ | ✅ | ✅ | ✅ | AOUSD Core Specification 1.0.1 | 1.0.1 | ✅ conformant |
| `ptex` | 0.5 | ✅ | ✅ | ✅ | ✅ | Ptex 2.5.x | — | ⚪ undeclared |
| `vdb` | 0.1 | ❌ git only | ✅ | ✅ | ✅ | OpenVDB 14.x | 10.0.1 | 🟡 lagging |

## Repository layout

```
Cargo.toml                 workspace (+ the strata-check-only [patch] entries)
crates/strata/             the facade crate (package strata-platform, lib strata)
check/                     strata-check: every mineral + Crust in one graph, interop tests
xtask/                     cargo xtask check | render | matrix | test-mineral | status
deny.toml                  cargo-deny: duplicate bans scoped to vocabulary + minerals, licenses, advisories
platform/strata-2027.toml  the spec (source of truth)
platform/STRATA-2027.md    the spec for humans (generated: cargo xtask render)
platform/AUDIT.md          the 2027 dependency audit
.github/workflows/ci.yml   CI
```

## Development

```sh
cargo xtask check          # spec vs facade Cargo.toml, workspace, toolchain, deny.toml, STRATA-2027.md
cargo xtask render         # regenerate platform/STRATA-2027.md after editing the spec
cargo test --workspace --all-features
cargo deny check
cargo xtask test-mineral vdb   # one mineral's own tests at Strata's pins
```

To change a pin, edit `platform/strata-2027.toml` first, then make `cargo xtask check` pass.

## License

MIT
