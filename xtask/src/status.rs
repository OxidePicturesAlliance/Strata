//! `cargo xtask status`: the conformance table (Markdown and JSON) from the
//! CI job results.

use std::fmt::Write as _;
use std::path::Path;

use serde::Serialize;

use crate::minerals::MineralResult;
use crate::spec::{Reference, Source, Spec};

/// The platform-wide checks, as GitHub Actions job results
/// (`success`, `failure`, `cancelled`, `skipped`).
pub struct Gates {
    pub build: String,
    pub deny: String,
    pub spec: String,
}

#[derive(Serialize)]
struct Row {
    mineral: String,
    package: String,
    version: String,
    publishable: bool,
    builds_with_pins: bool,
    single_vocabulary: bool,
    own_tests: Option<bool>,
    cpp_library: String,
    cpp_target: String,
    cpp_validated: String,
    verdict: &'static str,
}

#[derive(Serialize)]
struct Report<'a> {
    platform: &'a str,
    version: &'a str,
    vfx_reference_platform: &'a str,
    commit: Option<String>,
    spec_consistent: bool,
    minerals: Vec<Row>,
}

fn verdict(row: &Row, reference: Reference) -> &'static str {
    if !(row.builds_with_pins && row.single_vocabulary && row.own_tests == Some(true)) {
        return "failing";
    }
    match reference {
        Reference::Matches => "conformant",
        Reference::Lagging => "lagging",
        Reference::Undeclared => "undeclared",
    }
}

fn mark(ok: Option<bool>) -> &'static str {
    match ok {
        Some(true) => "✅",
        Some(false) => "❌",
        None => "⚪ not run",
    }
}

pub fn run(spec: &Spec, results: &Path, gates: &Gates, md: &Path, json: &Path) -> Result<(), String> {
    let mut rows = Vec::new();
    for (key, m) in &spec.minerals {
        let file = results.join(format!("{key}.json"));
        let result: Option<MineralResult> = std::fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok());
        let mut row = Row {
            mineral: key.clone(),
            package: m.package.clone(),
            version: m.version.clone(),
            publishable: m.source == Source::CratesIo,
            builds_with_pins: gates.build == "success",
            single_vocabulary: gates.deny == "success",
            own_tests: result.as_ref().map(MineralResult::passed),
            cpp_library: m.cpp_library.clone(),
            cpp_target: m.cpp_target.clone(),
            cpp_validated: m.cpp_validated.clone(),
            verdict: "",
        };
        row.verdict = verdict(&row, m.reference());
        rows.push(row);
    }

    let pf = &spec.platform;
    let mut o = String::new();
    let _ = writeln!(o, "# {} {} conformance", pf.name, pf.year);
    let _ = writeln!(o);
    let commit = std::env::var("GITHUB_SHA").ok();
    let _ = writeln!(
        o,
        "Facade `{}` {} · VFX Reference Platform {}{}",
        pf.facade_package,
        pf.version,
        pf.vfx_reference_platform,
        commit
            .as_deref()
            .map(|c| format!(" · commit `{}`", c.get(..7).unwrap_or(c)))
            .unwrap_or_default()
    );
    let _ = writeln!(o);
    let _ = writeln!(o, "Spec consistency: {}", mark(Some(gates.spec == "success")));
    let _ = writeln!(o);
    let _ = writeln!(
        o,
        "| Mineral | Version | On crates.io | Builds with pins | One copy of each vocabulary crate | Own tests at pins | C++ target ({}) | Validated against | Verdict |",
        pf.vfx_reference_platform
    );
    let _ = writeln!(o, "|---|---|---|---|---|---|---|---|---|");
    for r in &rows {
        let verdict = match r.verdict {
            "conformant" => "✅ conformant",
            "lagging" => "🟡 lagging",
            "undeclared" => "⚪ undeclared",
            _ => "❌ failing",
        };
        let dash = |s: &str| if s.is_empty() { "—".to_owned() } else { s.to_owned() };
        let _ = writeln!(
            o,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {verdict} |",
            r.mineral,
            r.version,
            if r.publishable { "✅" } else { "❌ git only" },
            mark(Some(r.builds_with_pins)),
            mark(Some(r.single_vocabulary)),
            mark(r.own_tests),
            if r.cpp_target.is_empty() {
                format!("{} (no version pinned)", r.cpp_library)
            } else {
                format!("{} {}", r.cpp_library, r.cpp_target)
            },
            dash(&r.cpp_validated),
        );
    }
    let _ = writeln!(o);
    let _ = writeln!(
        o,
        "✅ conformant: builds against the pins, one copy of each vocabulary crate, own tests pass, \
         validated against the {} C++ version. 🟡 lagging: the same, but validated against an older C++ \
         version. ⚪ undeclared: passes, but does not state which C++ version it is validated against.",
        pf.vfx_reference_platform
    );

    let report = Report {
        platform: &pf.name,
        version: &pf.version,
        vfx_reference_platform: &pf.vfx_reference_platform,
        commit,
        spec_consistent: gates.spec == "success",
        minerals: rows,
    };
    for path in [md, json] {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
    }
    std::fs::write(md, o).map_err(|e| format!("{}: {e}", md.display()))?;
    let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    std::fs::write(json, text + "\n").map_err(|e| format!("{}: {e}", json.display()))?;
    Ok(())
}
