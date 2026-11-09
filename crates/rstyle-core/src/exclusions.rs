//! Paths a run leaves out. Core and every frontend read these globs one way, which
//! `exclusions/cases.tsv` pins down.
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

/// Compiled exclusions. A glob is matched against a path relative to the run's root, as reports
/// show it, and against each directory above it, so a glob that matches a directory takes
/// everything under it. A leading `./` and trailing `/` are ignored.
pub struct Matcher(GlobSet);

impl Matcher {
    pub fn compile(patterns: &[String]) -> Result<Self, String> {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let glob = Glob::new(normal(pattern))
                .map_err(|error| format!("bad exclude `{pattern}`: {error}"))?;
            builder.add(glob);
        }
        builder.build().map(Self).map_err(|error| error.to_string())
    }

    pub fn is_excluded(&self, path: &Path) -> bool {
        path.ancestors()
            .filter(|path| !path.as_os_str().is_empty())
            .any(|path| self.0.is_match(path))
    }
}

/// A glob as it's matched: `./tests/` names what `tests` does.
fn normal(pattern: &str) -> &str {
    let pattern = pattern.strip_prefix("./").unwrap_or(pattern);
    pattern.trim_end_matches('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_exclusions_match_root_target_contents() {
        let matcher = Matcher::compile(&crate::config::Excludes::default().0).unwrap();
        for path in ["target/generated.rs", "target/debug/build/generated.rs"] {
            assert!(matcher.is_excluded(Path::new(path)), "{path}");
        }
        for path in [
            "src/lib.rs",
            "crates/member/target/generated.rs",
            "targeted/lib.rs",
        ] {
            assert!(!matcher.is_excluded(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn empty_exclusions_keep_all_paths() {
        let matcher = Matcher::compile(&[]).unwrap();
        for path in [
            "target/generated.rs",
            "src/lib.rs",
            "crates/member/src/lib.rs",
            "",
        ] {
            assert!(!matcher.is_excluded(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn explicit_recursive_globs_distinguish_root_and_member_sources() {
        let anchored = Matcher::compile(&["src/**".into()]).unwrap();
        let recursive = Matcher::compile(&["**/src/**".into()]).unwrap();
        assert!(anchored.is_excluded(Path::new("src/nested/lib.rs")));
        assert!(!anchored.is_excluded(Path::new("crates/member/src/lib.rs")));
        for path in ["src/lib.rs", "crates/member/src/lib.rs"] {
            assert!(recursive.is_excluded(Path::new(path)), "{path}");
        }
        assert!(!recursive.is_excluded(Path::new("crates/member/source/lib.rs")));
    }

    #[test]
    fn a_directory_takes_what_is_under_it_however_it_is_written() {
        for written in ["tests/fixtures", "tests/fixtures/", "./tests/fixtures"] {
            let matcher = Matcher::compile(&[written.into()]).unwrap();
            assert!(matcher.is_excluded(Path::new("tests/fixtures/deep/py2.py")));
            assert!(!matcher.is_excluded(Path::new("tests/fixtures.py")));
            assert!(!matcher.is_excluded(Path::new("other/tests/fixtures/a.py")));
        }
    }

    #[test]
    fn invalid_globs_keep_the_existing_deterministic_diagnostic() {
        let patterns = ["src/**".into(), "[".into(), "{".into()];
        let first = Matcher::compile(&patterns).err().unwrap();
        let second = Matcher::compile(&patterns).err().unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with("bad exclude `[`: "), "{first}");
    }

    /// The table a frontend can check its own reading of a glob against.
    #[test]
    fn every_shared_case_reads_as_the_table_says() {
        let table = include_str!("exclusions/cases.tsv");
        let mut checked = 0;
        for line in table.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            let [glob, path, expected] = fields[..] else {
                panic!("not glob, path and yes or no: {line:?}");
            };
            let matcher = Matcher::compile(&[glob.into()]).unwrap();
            let excluded = matcher.is_excluded(Path::new(path));
            assert_eq!(excluded, expected == "yes", "{glob} on {path}");
            checked += 1;
        }
        assert!(checked > 50, "{checked} cases");
    }
}
