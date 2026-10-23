//! The typed form of `platform/strata-2027.toml`.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub platform: Platform,
    pub rust: Rust,
    pub vocabulary: BTreeMap<String, Vocabulary>,
    pub watched: BTreeMap<String, String>,
    pub minerals: BTreeMap<String, Mineral>,
    pub consumers: BTreeMap<String, Consumer>,
    #[serde(default)]
    pub patches: Vec<Patch>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub name: String,
    pub year: u16,
    pub version: String,
    pub status: String,
    pub vfx_reference_platform: String,
    pub facade_package: String,
    pub facade_lib: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rust {
    pub toolchain: String,
    pub msrv: String,
    pub edition: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vocabulary {
    pub version: String,
    pub features: Vec<String>,
    pub why: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    #[serde(rename = "crates.io")]
    CratesIo,
    Git,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mineral {
    pub package: String,
    pub lib: String,
    pub version: String,
    pub source: Source,
    pub git: Option<String>,
    pub rev: Option<String>,
    pub extra_packages: Vec<String>,
    pub repository: String,
    pub test_ref: String,
    pub test_args: Vec<String>,
    pub patch: Option<GitPatch>,
    pub cpp_library: String,
    pub cpp_target: String,
    pub cpp_validated: String,
    pub on_vfx_platform: bool,
    pub blocking: Vec<String>,
}

impl Mineral {
    /// Every package the mineral brings: its own plus `extra_packages`.
    pub fn packages(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.package.as_str()).chain(self.extra_packages.iter().map(String::as_str))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitPatch {
    pub git: String,
    pub rev: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub repository: String,
    pub rev: String,
    pub packages: Vec<String>,
    pub blocking: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub source: String,
    pub package: String,
    pub version: String,
    pub why: String,
}

/// How a mineral's validated C++ version compares with the platform target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reference {
    /// Validated against the CY target.
    Matches,
    /// Validated, but against an older version than the target.
    Lagging,
    /// The mineral does not declare what it is validated against.
    Undeclared,
}

impl Mineral {
    pub fn reference(&self) -> Reference {
        if self.cpp_validated.is_empty() || self.cpp_target.is_empty() {
            return Reference::Undeclared;
        }
        // "14.x" matches "14.0.1"; "1.0.1" matches only "1.0.1".
        let target = self.cpp_target.trim_end_matches('x');
        let validated = self.cpp_validated.split_whitespace().next().unwrap_or_default();
        let matches = if target.ends_with('.') {
            validated.starts_with(target)
        } else {
            validated == target
        };
        if matches {
            Reference::Matches
        } else {
            Reference::Lagging
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mineral(target: &str, validated: &str) -> Mineral {
        Mineral {
            package: String::new(),
            lib: String::new(),
            version: String::new(),
            source: Source::CratesIo,
            git: None,
            rev: None,
            extra_packages: Vec::new(),
            repository: String::new(),
            test_ref: String::new(),
            test_args: Vec::new(),
            patch: None,
            cpp_library: String::new(),
            cpp_target: target.into(),
            cpp_validated: validated.into(),
            on_vfx_platform: true,
            blocking: Vec::new(),
        }
    }

    #[test]
    fn reference_comparison() {
        assert_eq!(mineral("14.x", "14.0.1").reference(), Reference::Matches);
        assert_eq!(mineral("14.x", "10.0.1").reference(), Reference::Lagging);
        assert_eq!(mineral("2.6.x", "2.5.0+ (ported)").reference(), Reference::Lagging);
        assert_eq!(mineral("2.6.x", "2.6.0").reference(), Reference::Matches);
        assert_eq!(mineral("1.0.1", "1.0.1").reference(), Reference::Matches);
        assert_eq!(mineral("3.8.x", "").reference(), Reference::Undeclared);
        assert_eq!(mineral("", "").reference(), Reference::Undeclared);
    }
}
