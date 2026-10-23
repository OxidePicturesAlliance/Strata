//! Strata's spec tooling. Run through the `cargo xtask` alias:
//!
//! ```text
//! cargo xtask check                     # spec vs manifests, deny.toml, STRATA-2027.md
//! cargo xtask render                    # regenerate platform/STRATA-2027.md
//! cargo xtask matrix                    # mineral names as JSON, for the CI matrix
//! cargo xtask test-mineral <name> [--out <file.json>]
//! cargo xtask status --results <dir> --build <res> --deny <res> --spec <res>
//!                    [--md <file.md>] [--json <file.json>]
//! ```
#![forbid(unsafe_code)]

mod check;
mod minerals;
mod render;
mod spec;
mod status;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SPEC: &str = "platform/strata-2027.toml";

fn workspace_root() -> PathBuf {
    // xtask/ lives directly under the workspace root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

fn load(root: &Path) -> Result<spec::Spec, String> {
    let text = std::fs::read_to_string(root.join(SPEC)).map_err(|e| format!("{SPEC}: {e}"))?;
    toml::from_str(&text).map_err(|e| format!("{SPEC}: {e}"))
}

/// The value after `--name`, if given.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn real_main() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = workspace_root();
    let spec = load(&root)?;
    match args.first().map(String::as_str) {
        Some("check") => {
            check::run(&root, &spec)?;
            println!(
                "{SPEC}: consistent with the manifests, deny.toml and {}",
                render::OUTPUT
            );
            Ok(true)
        }
        Some("render") => {
            let path = root.join(render::OUTPUT);
            std::fs::write(&path, render::render(&spec)).map_err(|e| format!("{}: {e}", path.display()))?;
            println!("wrote {}", render::OUTPUT);
            Ok(true)
        }
        Some("matrix") => {
            let names: Vec<&String> = spec.minerals.keys().collect();
            println!("{}", serde_json::to_string(&names).map_err(|e| e.to_string())?);
            Ok(true)
        }
        Some("test-mineral") => {
            let name = args
                .get(1)
                .ok_or("usage: cargo xtask test-mineral <name> [--out <file.json>]")?;
            let out = flag(&args, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join("target/strata-status").join(format!("{name}.json")));
            minerals::test(&root, &spec, name, &out)
        }
        Some("status") => {
            let results = flag(&args, "--results").ok_or("--results <dir> is required")?;
            let gates = status::Gates {
                build: flag(&args, "--build").ok_or("--build <result> is required")?,
                deny: flag(&args, "--deny").ok_or("--deny <result> is required")?,
                spec: flag(&args, "--spec").ok_or("--spec <result> is required")?,
            };
            let md = flag(&args, "--md").map_or_else(|| root.join("target/strata-status/STATUS.md"), PathBuf::from);
            let json =
                flag(&args, "--json").map_or_else(|| root.join("target/strata-status/status.json"), PathBuf::from);
            status::run(&spec, Path::new(&results), &gates, &md, &json)?;
            println!("wrote {} and {}", md.display(), json.display());
            Ok(true)
        }
        _ => Err("usage: cargo xtask <check|render|matrix|test-mineral|status>".into()),
    }
}

fn main() -> ExitCode {
    match real_main() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
