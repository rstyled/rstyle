//! Reads the facts files written by `rstyle-driver`.

use rstyle_core::facts::FnFacts;
use std::path::Path;

/// Which facts files count: `<crate>-<kind>.json` for the expected crates, and
/// only lib/bin kinds unless test targets were requested.
pub struct Wanted<'a> {
    pub crates: &'a [String],
    pub all_targets: bool,
}

/// Splits `my_crate-lib.json` into `("my_crate", "lib")`.
pub fn parse_name(file_name: &str) -> Option<(&str, &str)> {
    let stem = file_name.strip_suffix(".json")?;
    stem.rsplit_once('-')
}

impl Wanted<'_> {
    pub fn accepts(&self, file_name: &str) -> bool {
        let Some((krate, kind)) = parse_name(file_name) else {
            return false;
        };
        let kind_ok = self.all_targets || kind == "lib" || kind == "bin";
        kind_ok && self.crates.iter().any(|c| c == krate)
    }
}

/// Facts from all accepted files, read in sorted file-name order.
// rstyle-allow(side_effect_density): IO boundary: reads the driver's facts files
pub fn read_facts(dir: &Path, wanted: &Wanted<'_>) -> Result<Vec<FnFacts>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| wanted.accepts(name))
        .collect();
    names.sort();
    let mut all = Vec::new();
    for name in names {
        let path = dir.join(&name);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{name}: {e}"))?;
        let facts: Vec<FnFacts> =
            serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))?;
        all.extend(facts);
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_crate_and_kind() {
        assert_eq!(parse_name("my_crate-lib.json"), Some(("my_crate", "lib")));
        assert_eq!(parse_name("notes.txt"), None);
    }

    #[test]
    fn filters_by_crate_and_kind() {
        let crates = vec!["app".to_string()];
        let lib_only = Wanted {
            crates: &crates,
            all_targets: false,
        };
        assert!(lib_only.accepts("app-lib.json"));
        assert!(lib_only.accepts("app-bin.json"));
        assert!(!lib_only.accepts("app-test.json"));
        assert!(!lib_only.accepts("other-lib.json"));
        let everything = Wanted {
            crates: &crates,
            all_targets: true,
        };
        assert!(everything.accepts("app-test.json"));
    }
}
