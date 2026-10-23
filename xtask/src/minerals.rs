//! `cargo xtask test-mineral <name>`: run a mineral's own test suite with
//! the vocabulary crates moved to the exact versions Strata's `Cargo.lock`
//! resolved.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use crate::spec::Spec;

/// One mineral's outcome, written as JSON for `cargo xtask status`.
#[derive(Debug, Serialize, Deserialize)]
pub struct MineralResult {
    pub mineral: String,
    pub test_ref: String,
    pub toolchain: String,
    /// Vocabulary crate -> the version the mineral was tested with.
    pub vocabulary: BTreeMap<String, String>,
    /// Empty when the mineral's graph accepted every Strata version.
    pub alignment_errors: Vec<String>,
    pub tests_passed: bool,
}

impl MineralResult {
    pub fn passed(&self) -> bool {
        self.alignment_errors.is_empty() && self.tests_passed
    }
}

/// Package name -> every version of it in a `Cargo.lock`.
fn lock_versions(path: &Path) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let lock: Table = text.parse().map_err(|e| format!("{}: {e}", path.display()))?;
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for pkg in lock.get("package").and_then(Value::as_array).into_iter().flatten() {
        let (Some(name), Some(version)) = (
            pkg.get("name").and_then(Value::as_str),
            pkg.get("version").and_then(Value::as_str),
        ) else {
            continue;
        };
        map.entry(name.to_owned()).or_default().insert(version.to_owned());
    }
    Ok(map)
}

/// The one version of each vocabulary crate in Strata's own lock.
pub fn strata_vocabulary(root: &Path, spec: &Spec) -> Result<BTreeMap<String, String>, String> {
    let lock = lock_versions(&root.join("Cargo.lock"))?;
    let mut out = BTreeMap::new();
    for name in spec.vocabulary.keys() {
        let versions = lock.get(name).cloned().unwrap_or_default();
        match versions.len() {
            1 => {
                out.insert(name.clone(), versions.into_iter().next().unwrap_or_default());
            }
            0 => return Err(format!("Cargo.lock has no `{name}`; run `cargo generate-lockfile`")),
            _ => {
                return Err(format!(
                    "Cargo.lock has several `{name}`: {versions:?}; see `cargo deny check bans`"
                ));
            }
        }
    }
    Ok(out)
}

fn run(cmd: &mut Command) -> Result<bool, String> {
    eprintln!("$ {cmd:?}");
    cmd.status().map(|s| s.success()).map_err(|e| format!("{cmd:?}: {e}"))
}

fn must(cmd: &mut Command) -> Result<(), String> {
    if run(cmd)? {
        Ok(())
    } else {
        Err(format!("failed: {cmd:?}"))
    }
}

pub fn test(root: &Path, spec: &Spec, name: &str, out: &Path) -> Result<bool, String> {
    let mineral = spec
        .minerals
        .get(name)
        .ok_or_else(|| format!("no mineral `{name}` in the spec (have: {:?})", spec.minerals.keys()))?;
    let pins = strata_vocabulary(root, spec)?;
    let dir: PathBuf = root.join("target").join("minerals").join(name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    let git = || {
        let mut c = Command::new("git");
        c.arg("-C").arg(&dir);
        c
    };
    if !dir.join(".git").exists() {
        must(git().args(["init", "--quiet"]))?;
        must(git().args(["remote", "add", "origin", &mineral.repository]))?;
    }
    must(git().args(["fetch", "--quiet", "--depth", "1", "origin", &mineral.test_ref]))?;
    must(git().args(["checkout", "--quiet", "--force", "FETCH_HEAD"]))?;
    // Test fixtures often live in submodules (openusd: vendor/usd-wg-assets).
    if dir.join(".gitmodules").exists() {
        must(git().args([
            "submodule",
            "update",
            "--init",
            "--recursive",
            "--depth",
            "1",
            "--quiet",
        ]))?;
    }
    // A lock the repository does not track is a leftover from a previous run.
    let tracked = git()
        .args(["ls-files", "--error-unmatch", "Cargo.lock"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !tracked {
        let _ = std::fs::remove_file(dir.join("Cargo.lock"));
    }

    // The mineral's own rust-toolchain.toml must not win over the platform's.
    let cargo = || {
        let mut c = Command::new("cargo");
        c.current_dir(&dir).env("RUSTUP_TOOLCHAIN", &spec.rust.toolchain);
        c
    };
    if !dir.join("Cargo.lock").exists() {
        must(cargo().arg("generate-lockfile"))?;
    }

    let mut alignment_errors = Vec::new();
    let mut vocabulary = BTreeMap::new();
    let theirs = lock_versions(&dir.join("Cargo.lock"))?;
    for (krate, want) in &pins {
        let Some(have) = theirs.get(krate) else { continue };
        for old in have.iter().filter(|v| *v != want) {
            let spec_id = format!("{krate}@{old}");
            if !run(cargo().args(["update", "--quiet", "-p", &spec_id, "--precise", want]))? {
                alignment_errors.push(format!("`{krate}` {old} cannot move to {want}"));
            }
        }
    }
    let after = lock_versions(&dir.join("Cargo.lock"))?;
    for krate in pins.keys() {
        if let Some(versions) = after.get(krate) {
            if versions.len() > 1 {
                alignment_errors.push(format!("`{krate}` still has several versions: {versions:?}"));
            }
            vocabulary.insert(krate.clone(), versions.iter().cloned().collect::<Vec<_>>().join(", "));
        }
    }

    let tests_passed = alignment_errors.is_empty() && run(cargo().args(["test", "--locked"]).args(&mineral.test_args))?;
    let result = MineralResult {
        mineral: name.to_owned(),
        test_ref: mineral.test_ref.clone(),
        toolchain: spec.rust.toolchain.clone(),
        vocabulary,
        alignment_errors,
        tests_passed,
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
    std::fs::write(out, json + "\n").map_err(|e| format!("{}: {e}", out.display()))?;
    for e in &result.alignment_errors {
        eprintln!("alignment: {e}");
    }
    Ok(result.passed())
}
