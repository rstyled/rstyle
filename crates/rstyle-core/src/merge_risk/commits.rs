//! What the runs' commits say changed, which is how a comparison finds the files no document holds:
//! scripts, workflows, build files, lockfiles and configuration. A run only holds what its frontend
//! read, so without the commits a change to one of those would read as no change at all.

use crate::exclusions;
use serde::Serialize;
use std::path::Path;

/// A path that differs between the runs' commits, from the repository's root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedPath {
    pub path: String,
    /// What it is when it isn't a regular file in one of the commits, such as `symlink`. Its text
    /// isn't all of what it does, so no document can stand for it.
    pub special: Option<&'static str>,
}

/// What git says of the runs' commits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Commits {
    Compared {
        /// Where the runs' source is in the repository: empty at its root, or ending in `/`.
        prefix: String,
        changed: Vec<ChangedPath>,
    },
    /// Why they couldn't be compared, which leaves every file no document holds unseen.
    Unavailable(String),
}

/// A changed file nothing here reads or rates, from the repository's root.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Unread {
    pub path: String,
    /// `test` for a file a frontend marks as tests, or what it is when it isn't a regular file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special: Option<&'static str>,
}

impl Unread {
    /// Why nothing here can say what changing it does.
    pub(super) fn why(&self) -> String {
        match self.special {
            None => "no document holds it".to_owned(),
            Some("test") => "it's a test file, which isn't rated".to_owned(),
            Some(special) => format!("nothing here reads it ({special})"),
        }
    }

    pub(super) fn line(&self) -> String {
        format!("{}: changed, and {}", self.path, self.why())
    }
}

/// How the runs' documents hold a path from their source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Held {
    No,
    /// As a written file, which a comparison reads.
    Written,
    /// As a test file, whose functions are counted and not rated.
    Test,
}

/// The changed files the runs' documents don't hold, by what becomes of them.
pub(super) struct Outside {
    /// What nothing here reads, or why the commits can't say.
    pub unread: Result<Vec<Unread>, String>,
    /// What the base's policy excludes, from the repository's root.
    pub excluded: Vec<String>,
}

impl Outside {
    /// Sorts each changed path: a written file a document holds is compared already, one the policy
    /// excludes is one the project said not to look at, and documentation for people is read by
    /// people. Anything else, tests included, is what this can't say the effect of.
    pub(super) fn of(
        commits: &Commits,
        held: impl Fn(&str) -> Held,
        excluded: &exclusions::Matcher,
    ) -> Self {
        let (prefix, changed) = match commits {
            Commits::Compared { prefix, changed } => (prefix, changed),
            Commits::Unavailable(why) => {
                return Self {
                    unread: Err(why.clone()),
                    excluded: Vec::new(),
                };
            }
        };

        let mut unread = Vec::new();
        let mut skipped = Vec::new();
        for change in changed {
            let in_source = change.path.strip_prefix(prefix.as_str());
            let held_as = in_source.map_or(Held::No, &held);
            let excluded = in_source.is_some_and(|path| excluded.is_excluded(Path::new(path)));
            let special = match (change.special, held_as) {
                (Some(special), _) => Some(special),
                (None, Held::Written) => continue,
                (None, _) if excluded => {
                    skipped.push(change.path.clone());
                    continue;
                }
                (None, Held::Test) => Some("test"),
                (None, Held::No) if documentation(&change.path) => continue,
                (None, Held::No) => None,
            };
            unread.push(Unread {
                path: change.path.clone(),
                special,
            });
        }
        Self {
            unread: Ok(unread),
            excluded: skipped,
        }
    }
}

/// Files with these names, or under folders with them, are read by agents, not people.
const AGENT_FILES: &[&str] = &[
    "agent",
    "agents",
    "skill",
    "skills",
    "agent.md",
    "agents.md",
    "skill.md",
    "skills.md",
    "claude.md",
    "codex.md",
    "gemini.md",
];

/// The repository's `README.md` and Markdown under its `docs/`, as gstyle reads them: prose for
/// people. A dotted folder, or a file an agent reads, is read by a program, so it isn't.
fn documentation(path: &str) -> bool {
    if path == "README.md" {
        return true;
    }
    if !path.starts_with("docs/") || !path.ends_with(".md") {
        return false;
    }
    path.to_lowercase()
        .split('/')
        .all(|part| !part.starts_with('.') && !AGENT_FILES.contains(&part))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changed(paths: &[&str]) -> Commits {
        Commits::Compared {
            prefix: String::new(),
            changed: paths
                .iter()
                .map(|path| ChangedPath {
                    path: (*path).to_owned(),
                    special: None,
                })
                .collect(),
        }
    }

    fn unread(outside: &Outside) -> Vec<&str> {
        let files = outside.unread.as_ref().unwrap();
        files.iter().map(|file| file.path.as_str()).collect()
    }

    fn nothing_excluded() -> exclusions::Matcher {
        exclusions::Matcher::compile(&[]).unwrap()
    }

    fn written(path: &'static str) -> impl Fn(&str) -> Held {
        move |held: &str| match held == path {
            true => Held::Written,
            false => Held::No,
        }
    }

    /// gstyle passes a new test file only once it has parsed it as plain tests, which a document
    /// can't say, so here any test file that changed needs a person.
    #[test]
    fn a_changed_test_file_is_unread_and_says_it_is_a_test() {
        let commits = changed(&["state_test.go"]);
        let outside = Outside::of(&commits, |_| Held::Test, &nothing_excluded());
        let files = outside.unread.unwrap();
        assert_eq!(files[0].special, Some("test"));
        assert_eq!(
            files[0].line(),
            "state_test.go: changed, and it's a test file, which isn't rated"
        );
    }

    #[test]
    fn a_changed_file_no_document_holds_is_unread() {
        let commits = changed(&[".github/workflows/ci.yml", "Cargo.lock", "src/lib.rs"]);
        let outside = Outside::of(&commits, written("src/lib.rs"), &nothing_excluded());
        assert_eq!(unread(&outside), [".github/workflows/ci.yml", "Cargo.lock"]);
    }

    #[test]
    fn what_the_policy_excludes_is_set_apart_and_not_unread() {
        let commits = changed(&["src/generated/api.rs", "scripts/gen.sh"]);
        let excluded = exclusions::Matcher::compile(&["src/generated/**".into()]).unwrap();
        let outside = Outside::of(&commits, |_| Held::No, &excluded);
        assert_eq!(unread(&outside), ["scripts/gen.sh"]);
        assert_eq!(outside.excluded, ["src/generated/api.rs"]);
    }

    #[test]
    fn only_prose_for_people_reads_as_documentation() {
        for path in ["README.md", "docs/merge-risk.md", "docs/guide/start.md"] {
            assert!(documentation(path), "{path}");
        }
        for path in [
            "crates/core/README.md",
            "docs/notes.txt",
            "docs/AGENTS.md",
            "docs/skills/review.md",
            "docs/.hidden/page.md",
            "SKILL.md",
            "CHANGELOG.md",
        ] {
            assert!(!documentation(path), "{path}");
        }
    }

    #[test]
    fn a_symlink_or_executable_is_unread_even_where_a_document_holds_its_path() {
        let commits = Commits::Compared {
            prefix: String::new(),
            changed: vec![ChangedPath {
                path: "src/lib.rs".into(),
                special: Some("executable"),
            }],
        };
        let outside = Outside::of(&commits, |_| Held::Written, &nothing_excluded());
        let files = outside.unread.unwrap();
        assert_eq!(files[0].special, Some("executable"));
        assert_eq!(
            files[0].line(),
            "src/lib.rs: changed, and nothing here reads it (executable)"
        );
    }

    #[test]
    fn a_source_below_the_repository_root_holds_only_what_is_under_it() {
        let commits = Commits::Compared {
            prefix: "go/".into(),
            changed: ["go/main.go", "main.go", "Makefile"]
                .iter()
                .map(|path| ChangedPath {
                    path: (*path).to_owned(),
                    special: None,
                })
                .collect(),
        };
        let outside = Outside::of(&commits, written("main.go"), &nothing_excluded());
        assert_eq!(unread(&outside), ["main.go", "Makefile"]);
    }

    #[test]
    fn commits_that_could_not_be_compared_say_why() {
        let commits = Commits::Unavailable("no commit".into());
        let outside = Outside::of(&commits, |_| Held::Written, &nothing_excluded());
        assert_eq!(outside.unread, Err("no commit".into()));
    }
}
