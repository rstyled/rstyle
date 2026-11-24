//! The machine's own directories, which a capture's documents name by placeholder.
//!
//! A document records the compiler invocation, the selected package and where each file and each
//! dependency's metadata live. Written as they are, those paths tie a document to one checkout on
//! one machine: the same source captured anywhere else gives other bytes, so nothing a run keeps
//! can serve another. A document names them from `$ROOT`, the source root, `$TARGET`, the build
//! directory Cargo wrote to, and `$SYSROOT`, the toolchain's sysroot, and a reader that wants the
//! machine's path back puts its own directories in their place.
use serde_json::Value;
use std::path::Path;

pub const ROOT: &str = "$ROOT";
pub const TARGET: &str = "$TARGET";
pub const SYSROOT: &str = "$SYSROOT";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Places {
    /// Each directory and its placeholder, the longest first, so one inside another keeps its own.
    directories: Vec<(String, &'static str)>,
}

impl Places {
    pub fn new(root: &Path, target: Option<&Path>, sysroot: Option<&Path>) -> Self {
        let mut directories: Vec<(String, &'static str)> =
            [(Some(root), ROOT), (target, TARGET), (sysroot, SYSROOT)]
                .into_iter()
                .filter_map(|(directory, name)| Some((spelled(directory?)?, name)))
                .collect();
        directories.sort_by_key(|(directory, _)| std::cmp::Reverse(directory.len()));
        directories.dedup_by(|later, earlier| later.0 == earlier.0);
        Self { directories }
    }

    /// The text with each of this machine's directories named by its placeholder.
    pub fn portable(&self, text: &str) -> String {
        let mut text = text.to_owned();
        for (directory, name) in &self.directories {
            text = replace_directory(&text, directory, name);
        }
        text
    }

    /// The text with each placeholder replaced by this machine's directory.
    pub fn local(&self, text: &str) -> String {
        let mut text = text.to_owned();
        for (directory, name) in &self.directories {
            text = text.replace(name, directory);
        }
        text
    }
}

/// The value with each of its strings changed, as a document's fields are made portable or local.
pub fn map_strings(value: &Value, change: &impl Fn(&str) -> String) -> Value {
    match value {
        Value::String(text) => Value::String(change(text)),
        Value::Array(items) => items.iter().map(|item| map_strings(item, change)).collect(),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, item)| (key.clone(), map_strings(item, change)))
            .collect(),
        Value::Null | Value::Bool(_) | Value::Number(_) => value.clone(),
    }
}

/// The directory as a path string, absolute and without a trailing separator, or nothing when it
/// isn't absolute or is the filesystem root, which would match every path.
fn spelled(directory: &Path) -> Option<String> {
    let directory = crate::absolute_path(directory).ok()?;
    let text = directory.to_str()?.trim_end_matches('/').to_owned();
    (!text.is_empty()).then_some(text)
}

/// The text with `directory` replaced by `name` wherever it stands as a whole directory: followed
/// by a separator or by anything a path name can't continue with, so `/a/b` leaves `/a/bc` alone.
fn replace_directory(text: &str, directory: &str, name: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(directory) {
        let after = &rest[at + directory.len()..];
        let whole = after.chars().next().is_none_or(|next| {
            !(next.is_alphanumeric() || matches!(next, '_' | '-' | '.' | '+' | '@' | '~'))
        });
        out.push_str(&rest[..at]);
        out.push_str(if whole { name } else { directory });
        rest = after;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn places() -> Places {
        Places::new(
            Path::new("/work/repo"),
            Some(Path::new("/work/repo/target/rstyle/runs/advisory-build")),
            Some(Path::new("/home/u/.rustup/toolchains/nightly")),
        )
    }

    #[test]
    fn each_directory_is_named_from_the_longest_and_back() {
        let places = places();
        for (local, portable) in [
            (
                "/work/repo/crates/a/src/lib.rs",
                "$ROOT/crates/a/src/lib.rs",
            ),
            (
                "libc=/work/repo/target/rstyle/runs/advisory-build/debug/liblibc-1.rmeta",
                "libc=$TARGET/debug/liblibc-1.rmeta",
            ),
            (
                "/home/u/.rustup/toolchains/nightly/lib/rustlib/x/liballoc-2.rmeta",
                "$SYSROOT/lib/rustlib/x/liballoc-2.rmeta",
            ),
            (
                "path+file:///work/repo/crates/a#0.1.0",
                "path+file://$ROOT/crates/a#0.1.0",
            ),
            ("/work/repo", "$ROOT"),
            ("--crate-name a", "--crate-name a"),
        ] {
            assert_eq!(places.portable(local), portable);
            assert_eq!(places.local(portable), local);
        }
    }

    #[test]
    fn a_longer_name_beside_a_directory_is_left_alone() {
        let places = places();
        for text in [
            "/work/repository/src",
            "/work/repo-2/src",
            "/work/repo.old",
            "/work/repos",
        ] {
            assert_eq!(places.portable(text), text);
        }
    }

    #[test]
    fn values_are_changed_in_every_string() {
        let places = places();
        let value = json!({"files": {"file:src/lib.rs": "/work/repo/src/lib.rs"},
                           "invocation": ["rustc", "/work/repo/src/lib.rs"], "count": 3});
        let portable = map_strings(&value, &|text| places.portable(text));
        assert_eq!(
            portable,
            json!({"files": {"file:src/lib.rs": "$ROOT/src/lib.rs"},
                   "invocation": ["rustc", "$ROOT/src/lib.rs"], "count": 3})
        );
        assert_eq!(map_strings(&portable, &|text| places.local(text)), value);
    }

    #[test]
    fn a_relative_or_root_directory_names_nothing() {
        let places = Places::new(Path::new("/"), Some(Path::new("relative")), None);
        assert_eq!(places.portable("/work/repo/src"), "/work/repo/src");
    }
}
