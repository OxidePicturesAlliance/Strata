# CLAUDE.md — Strata

Strata is the Oxide Pictures Alliance's yearly reference platform for pure-Rust VFX
libraries, in the spirit of vfxplatform.com but enforced by tooling. Read `README.md` for the
user-facing picture and `platform/AUDIT.md` for why each pin is what it is.

## Source of truth

`platform/strata-2027.toml` is the spec. Everything else follows it:

| File | Derived how | Checked by |
|---|---|---|
| `crates/strata/Cargo.toml` | by hand | `cargo xtask check` |
| `Cargo.toml` (`workspace.package`, `[patch]`) | by hand | `cargo xtask check` |
| `rust-toolchain.toml` | by hand | `cargo xtask check` |
| `check/Cargo.toml` (Crust rev) | by hand | `cargo xtask check` |
| `deny.toml` (`bans.deny`, `sources.allow-git`) | by hand | `cargo xtask check` |
| `platform/STRATA-2027.md` | **generated**: `cargo xtask render`, never edit | `cargo xtask check` |

To change a pin: edit the spec, run `cargo xtask render`, update the manifests until
`cargo xtask check` passes, then `cargo update -p <crate>` and `cargo deny check`.

## Layout

- `crates/strata`: the facade. Package `strata-platform`, lib `strata`.
- `check`: `strata-check`. Depends on the facade with `full` and on Crust. `tests/interop.rs`
  passes values between minerals.
- `xtask`: `check` (consistency), `render`, `matrix` (CI mineral list),
  `test-mineral <name>` (clones the mineral at `test_ref`, aligns its vocabulary crates to
  our `Cargo.lock` with `cargo update --precise`, runs its tests, writes JSON), `status`
  (conformance table: `STATUS.md` + `status.json`).

## Decisions (do not undo without a reason)

- **Package name `strata-platform`, lib `strata`.** `strata` is taken on crates.io
  (shepmaster/strata, 2019). Code still says `use strata::…`.
- **Version `YYYY.minor.patch[-pre]`**: the year is the semver major (2027.0.0-alpha.1).
- **Caret ranges only** in every manifest; `cargo xtask check` rejects `=` pins.
- **Vocabulary = crates whose types are in a mineral's public API**: half, bytemuck,
  num-traits, serde, rayon (shared global pool), glam (Crust only, for downstream). Private
  shared crates (flate2, zip, lz4_flex, exr, image) are "watched": duplicates allowed.
- **deny.toml** uses `multiple-versions = "allow"` plus `deny-multiple-versions = true` per
  vocabulary crate *and per mineral package*. That catches the same version from two sources
  (git + crates.io), which is two incompatible crates.
- **Facade is `publish = false`** until vdb is on crates.io. Never add `version = …` to the vdb
  git dependency: crates.io would resolve the unrelated `vdb` crate instead.
- **Workspace `[patch]`** (openusd → `main` commit `bb9058e`; Crust's opensubdiv-rs git tag →
  crates.io) exists only so strata-check can build Crust. It does not affect facade users.
  Remove each patch when the matching upstream blocker is resolved (AUDIT.md §5).
- **openusd tracks Crust's lock**, not openusd `main` HEAD: newer `main` breaks Crust's API use.
  Bump both revs together.
- **Each mineral records `cpp_target` (CY2027) and `cpp_validated`.** The verdict is
  conformant / lagging / undeclared / failing. The CY2027 values came from vfxplatform.com
  screenshots on 2026-10-23 (the site is not reachable from the sandbox).
- **The minerals are never modified here.** A needed change becomes an upstream task in
  `platform/AUDIT.md` with the exact `Cargo.toml` diff.
- MSRV and CI toolchain 1.96: openusd and Crust (cranelift) require it.

## Conventions

- MIT, edition 2024, `cargo fmt` (max_width 120), `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `unsafe_code = "forbid"`.
- Small, reviewable commits.
- Before pushing: `cargo xtask check && cargo fmt --all --check && cargo clippy … && cargo test
  --workspace --all-features && cargo deny check`.

## Gotchas

- Mineral repos ship their own `rust-toolchain.toml` (Crust pins 1.99). `test-mineral` sets
  `RUSTUP_TOOLCHAIN` to the spec's toolchain so the platform's toolchain wins.
- The mineral CI jobs set `rustflags: ""`: setup-rust-toolchain defaults to `-D warnings`,
  which would fail minerals on their own lints.
- vfxplatform.com is blocked by the sandbox proxy; ask the maintainer for values.
