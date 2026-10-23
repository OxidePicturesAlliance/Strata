//! `cargo xtask check`: the spec, the manifests, `deny.toml` and
//! `STRATA-2027.md` must agree.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use toml::{Table, Value};

use crate::render;
use crate::spec::{Source, Spec};

/// Collects every disagreement, so one run reports them all.
#[derive(Default)]
pub struct Problems(Vec<String>);

impl Problems {
    fn push(&mut self, file: &str, msg: impl std::fmt::Display) {
        self.0.push(format!("{file}: {msg}"));
    }

    fn expect_eq(&mut self, file: &str, what: &str, found: Option<&str>, expected: &str) {
        if found != Some(expected) {
            self.push(file, format!("{what} is {}, the spec says {expected:?}", show(found)));
        }
    }

    pub fn into_result(self) -> Result<(), String> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} disagreement(s) with platform/strata-2027.toml:\n  - {}",
                self.0.len(),
                self.0.join("\n  - ")
            ))
        }
    }
}

fn show(v: Option<&str>) -> String {
    v.map_or_else(|| "missing".to_owned(), |s| format!("{s:?}"))
}

fn read_table(root: &Path, rel: &str) -> Result<Table, String> {
    let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    text.parse::<Table>().map_err(|e| format!("{rel}: {e}"))
}

fn get<'a>(table: &'a Table, path: &[&str]) -> Option<&'a Value> {
    let (last, init) = path.split_last()?;
    let mut t = table;
    for key in init {
        t = t.get(*key)?.as_table()?;
    }
    t.get(*last)
}

fn get_str<'a>(table: &'a Table, path: &[&str]) -> Option<&'a str> {
    get(table, path)?.as_str()
}

fn str_set(v: Option<&Value>) -> BTreeSet<String> {
    v.and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default()
}

/// A dependency entry normalised from either `"1.0"` or `{ version = … }`.
struct Dep<'a> {
    version: Option<&'a str>,
    package: Option<&'a str>,
    git: Option<&'a str>,
    rev: Option<&'a str>,
    optional: bool,
    features: BTreeSet<String>,
}

fn dep(v: &Value) -> Dep<'_> {
    match v {
        Value::String(s) => Dep {
            version: Some(s),
            package: None,
            git: None,
            rev: None,
            optional: false,
            features: BTreeSet::new(),
        },
        Value::Table(t) => Dep {
            version: get_str(t, &["version"]),
            package: get_str(t, &["package"]),
            git: get_str(t, &["git"]),
            rev: get_str(t, &["rev"]),
            optional: get(t, &["optional"]).and_then(Value::as_bool).unwrap_or(false),
            features: str_set(t.get("features")),
        },
        _ => Dep {
            version: None,
            package: None,
            git: None,
            rev: None,
            optional: false,
            features: BTreeSet::new(),
        },
    }
}

pub fn run(root: &Path, spec: &Spec) -> Result<(), String> {
    let mut p = Problems::default();
    facade(root, spec, &mut p)?;
    workspace(root, spec, &mut p)?;
    toolchain(root, spec, &mut p)?;
    check_crate(root, spec, &mut p)?;
    deny(root, spec, &mut p)?;
    no_exact_pins(root, &mut p)?;
    rendered(root, spec, &mut p)?;
    p.into_result()
}

fn facade(root: &Path, spec: &Spec, p: &mut Problems) -> Result<(), String> {
    const F: &str = "crates/strata/Cargo.toml";
    let m = read_table(root, F)?;
    p.expect_eq(
        F,
        "package.name",
        get_str(&m, &["package", "name"]),
        &spec.platform.facade_package,
    );
    p.expect_eq(F, "lib.name", get_str(&m, &["lib", "name"]), &spec.platform.facade_lib);
    p.expect_eq(
        F,
        "package.version",
        get_str(&m, &["package", "version"]),
        &spec.platform.version,
    );
    p.expect_eq(
        F,
        "package.edition",
        get_str(&m, &["package", "edition"]),
        &spec.rust.edition,
    );
    p.expect_eq(
        F,
        "package.rust-version",
        get_str(&m, &["package", "rust-version"]),
        &spec.rust.msrv,
    );
    let major = spec.platform.version.split('.').next();
    if major != Some(spec.platform.year.to_string().as_str()) {
        p.push(
            F,
            format!(
                "version {} does not have the year {} as its major",
                spec.platform.version, spec.platform.year
            ),
        );
    }

    let deps = get(&m, &["dependencies"])
        .and_then(Value::as_table)
        .cloned()
        .unwrap_or_default();
    let mut expected: BTreeSet<String> = spec.vocabulary.keys().cloned().collect();

    for (name, v) in &spec.vocabulary {
        let Some(d) = deps.get(name).map(dep) else {
            p.push(F, format!("vocabulary crate `{name}` is not a dependency"));
            continue;
        };
        p.expect_eq(F, &format!("`{name}` version"), d.version, &v.version);
        if d.optional {
            p.push(F, format!("vocabulary crate `{name}` must not be optional"));
        }
        let want: BTreeSet<String> = v.features.iter().cloned().collect();
        if d.features != want {
            p.push(
                F,
                format!("`{name}` features are {:?}, the spec says {want:?}", d.features),
            );
        }
    }

    let features = get(&m, &["features"])
        .and_then(Value::as_table)
        .cloned()
        .unwrap_or_default();
    for (key, mineral) in &spec.minerals {
        expected.insert(key.clone());
        let Some(d) = deps.get(key).map(dep) else {
            p.push(
                F,
                format!("mineral `{key}` is not a dependency (key must be the feature name)"),
            );
            continue;
        };
        let package = d.package.unwrap_or(key);
        if package != mineral.package {
            p.push(
                F,
                format!("`{key}` is package `{package}`, the spec says `{}`", mineral.package),
            );
        }
        if !d.optional {
            p.push(F, format!("mineral `{key}` must be optional (behind feature `{key}`)"));
        }
        match mineral.source {
            Source::CratesIo => {
                p.expect_eq(F, &format!("`{key}` version"), d.version, &mineral.version);
                if d.git.is_some() {
                    p.push(F, format!("`{key}` is a crates.io mineral but has a git source"));
                }
            }
            Source::Git => {
                p.expect_eq(
                    F,
                    &format!("`{key}` git"),
                    d.git,
                    mineral.git.as_deref().unwrap_or_default(),
                );
                p.expect_eq(
                    F,
                    &format!("`{key}` rev"),
                    d.rev,
                    mineral.rev.as_deref().unwrap_or_default(),
                );
            }
        }
        let mut want_feature = vec![format!("dep:{key}")];
        for extra in &mineral.extra_packages {
            expected.insert(extra.clone());
            match deps.get(extra).map(dep) {
                None => p.push(F, format!("`{extra}` (part of mineral `{key}`) is not a dependency")),
                Some(d) => {
                    p.expect_eq(F, &format!("`{extra}` version"), d.version, &mineral.version);
                    if !d.optional {
                        p.push(F, format!("`{extra}` must be optional"));
                    }
                }
            }
            want_feature.push(format!("dep:{extra}"));
        }
        let have = str_set(features.get(key));
        for w in want_feature {
            if !have.contains(&w) {
                p.push(F, format!("feature `{key}` does not enable `{w}`"));
            }
        }
    }

    let full = str_set(features.get("full"));
    let all: BTreeSet<String> = spec.minerals.keys().cloned().collect();
    if full != all {
        p.push(
            F,
            format!("feature `full` is {full:?}, it must be exactly the minerals {all:?}"),
        );
    }
    let have: BTreeSet<String> = deps.keys().cloned().collect();
    for extra in have.difference(&expected) {
        p.push(
            F,
            format!("dependency `{extra}` is neither a vocabulary crate nor a mineral"),
        );
    }
    Ok(())
}

fn workspace(root: &Path, spec: &Spec, p: &mut Problems) -> Result<(), String> {
    const F: &str = "Cargo.toml";
    let m = read_table(root, F)?;
    p.expect_eq(
        F,
        "workspace.package.edition",
        get_str(&m, &["workspace", "package", "edition"]),
        &spec.rust.edition,
    );
    p.expect_eq(
        F,
        "workspace.package.rust-version",
        get_str(&m, &["workspace", "package", "rust-version"]),
        &spec.rust.msrv,
    );

    // Every patch, keyed by (source, package), against what the spec lists.
    let mut want: BTreeMap<(String, String), String> = BTreeMap::new();
    for m in spec.minerals.values() {
        if let Some(patch) = &m.patch {
            for pkg in m.packages() {
                want.insert(
                    ("crates-io".into(), pkg.to_owned()),
                    format!("git {} rev {}", patch.git, patch.rev),
                );
            }
        }
    }
    for patch in &spec.patches {
        want.insert(
            (patch.source.clone(), patch.package.clone()),
            format!("version {}", patch.version),
        );
    }

    let mut have: BTreeMap<(String, String), String> = BTreeMap::new();
    if let Some(patches) = get(&m, &["patch"]).and_then(Value::as_table) {
        for (source, entries) in patches {
            for (pkg, v) in entries.as_table().into_iter().flatten() {
                let d = dep(v);
                let desc = match (d.git, d.rev, d.version) {
                    (Some(g), Some(r), _) => format!("git {g} rev {r}"),
                    (None, None, Some(v)) => format!("version {v}"),
                    _ => "an unsupported patch form (use git+rev or a version)".into(),
                };
                have.insert((source.clone(), pkg.clone()), desc);
            }
        }
    }
    for (key, w) in &want {
        match have.get(key) {
            Some(h) if h == w => {}
            Some(h) => p.push(F, format!("[patch.{}] {} is {h}, the spec says {w}", key.0, key.1)),
            None => p.push(F, format!("[patch.{}] {} is missing ({w})", key.0, key.1)),
        }
    }
    for key in have.keys().filter(|k| !want.contains_key(*k)) {
        p.push(F, format!("[patch.{}] {} is not in the spec", key.0, key.1));
    }
    Ok(())
}

fn toolchain(root: &Path, spec: &Spec, p: &mut Problems) -> Result<(), String> {
    const F: &str = "rust-toolchain.toml";
    let m = read_table(root, F)?;
    p.expect_eq(
        F,
        "toolchain.channel",
        get_str(&m, &["toolchain", "channel"]),
        &spec.rust.toolchain,
    );
    Ok(())
}

fn check_crate(root: &Path, spec: &Spec, p: &mut Problems) -> Result<(), String> {
    const F: &str = "check/Cargo.toml";
    let m = read_table(root, F)?;
    let deps = get(&m, &["dependencies"])
        .and_then(Value::as_table)
        .cloned()
        .unwrap_or_default();
    let facade = deps.get(&spec.platform.facade_lib).map(dep);
    match facade {
        None => p.push(
            F,
            format!("does not depend on the facade as `{}`", spec.platform.facade_lib),
        ),
        Some(d) => {
            if !d.features.contains("full") {
                p.push(F, "the facade dependency must enable `full`");
            }
        }
    }
    for (name, consumer) in &spec.consumers {
        for pkg in &consumer.packages {
            let Some(d) = deps.get(pkg).map(dep) else {
                p.push(F, format!("consumer `{name}`: `{pkg}` is not a dependency"));
                continue;
            };
            p.expect_eq(F, &format!("`{pkg}` git"), d.git, &consumer.repository);
            p.expect_eq(F, &format!("`{pkg}` rev"), d.rev, &consumer.rev);
        }
    }
    Ok(())
}

/// The package names whose duplication deny.toml must reject.
pub fn denied_duplicates(spec: &Spec) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = spec.vocabulary.keys().cloned().collect();
    for m in spec.minerals.values() {
        set.extend(m.packages().map(str::to_owned));
    }
    set
}

/// The git repositories the graph may pull from.
pub fn allowed_git(spec: &Spec) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for m in spec.minerals.values() {
        set.extend(m.git.clone());
        set.extend(m.patch.as_ref().map(|p| p.git.clone()));
    }
    set.extend(spec.consumers.values().map(|c| c.repository.clone()));
    set
}

fn deny(root: &Path, spec: &Spec, p: &mut Problems) -> Result<(), String> {
    const F: &str = "deny.toml";
    let m = read_table(root, F)?;
    p.expect_eq(
        F,
        "bans.multiple-versions",
        get_str(&m, &["bans", "multiple-versions"]),
        "allow",
    );

    let mut have = BTreeSet::new();
    for entry in get(&m, &["bans", "deny"])
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(t) = entry.as_table() else { continue };
        let multi = get(t, &["deny-multiple-versions"]).and_then(Value::as_bool) == Some(true);
        if let (Some(name), true) = (get_str(t, &["crate"]), multi) {
            have.insert(name.to_owned());
        }
    }
    let want = denied_duplicates(spec);
    for missing in want.difference(&have) {
        p.push(
            F,
            format!("bans.deny has no `deny-multiple-versions` entry for `{missing}`"),
        );
    }
    for extra in have.difference(&want) {
        p.push(
            F,
            format!("bans.deny bans duplicates of `{extra}`, which is neither vocabulary nor mineral"),
        );
    }

    let git = str_set(get(&m, &["sources", "allow-git"]));
    let want_git = allowed_git(spec);
    if git != want_git {
        p.push(
            F,
            format!("sources.allow-git is {git:?}, the spec implies {want_git:?}"),
        );
    }
    Ok(())
}

/// Exact `=` pins in a library break downstream resolution; ban them in every
/// workspace manifest.
fn no_exact_pins(root: &Path, p: &mut Problems) -> Result<(), String> {
    for f in ["crates/strata/Cargo.toml", "check/Cargo.toml", "xtask/Cargo.toml"] {
        let m = read_table(root, f)?;
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            for (name, v) in get(&m, &[section]).and_then(Value::as_table).into_iter().flatten() {
                if dep(v).version.is_some_and(|v| v.trim_start().starts_with('=')) {
                    p.push(f, format!("`{name}` uses an exact `=` pin; use a caret range"));
                }
            }
        }
    }
    Ok(())
}

fn rendered(root: &Path, spec: &Spec, p: &mut Problems) -> Result<(), String> {
    let f = render::OUTPUT;
    let have = std::fs::read_to_string(root.join(f)).unwrap_or_default();
    if have.replace("\r\n", "\n") != render::render(spec) {
        p.push(f, "is out of date; run `cargo xtask render`");
    }
    Ok(())
}
