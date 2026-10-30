//! Shared configured path exclusions for legacy and captured-source consumers.
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

/// Compiled exclusions using the existing glob syntax and path matching policy.
pub struct Matcher(GlobSet);

impl Matcher {
    pub fn compile(patterns: &[String]) -> Result<Self, String> {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let glob =
                Glob::new(pattern).map_err(|error| format!("bad exclude `{pattern}`: {error}"))?;
            builder.add(glob);
        }
        builder.build().map(Self).map_err(|error| error.to_string())
    }

    pub fn is_excluded(&self, path: &Path) -> bool {
        self.0.is_match(path)
    }
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
    fn invalid_globs_keep_the_existing_deterministic_diagnostic() {
        let patterns = ["src/**".into(), "[".into(), "{".into()];
        let first = Matcher::compile(&patterns).err().unwrap();
        let second = Matcher::compile(&patterns).err().unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with("bad exclude `[`: "), "{first}");
    }
}
