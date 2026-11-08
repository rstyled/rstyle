//! Which commit a run read, and what changed between two runs' commits, from git. A run's documents
//! only hold what its frontend read, so the commits are how `risk` finds every other file that
//! changed.

use rstyle_core::merge_risk::{ChangedPath, Commits, Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// The commit a run read, when its source is in a git checkout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Revision {
    /// The commit checked out.
    commit: String,
    /// The run's source, from the repository's root: empty at its root, or ending in `/`.
    prefix: String,
    /// Whether every tracked file under the source was as the commit has it, both when the run
    /// began and when it was saved.
    clean: bool,
}

// rstyle-allow(side_effect_density): process boundary: runs git
fn git(directory: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .output()
        .map_err(|error| format!("git: {error}"))?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {}: {}", args[0], error.trim()));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("git {}: {error}", args[0]))
}

impl Revision {
    /// What `source` has checked out, or nothing when it isn't in a git checkout.
    pub(super) fn of(source: &Path) -> Option<Self> {
        let commit = git(source, &["rev-parse", "--verify", "HEAD^{commit}"]).ok()?;
        let prefix = git(source, &["rev-parse", "--show-prefix"]).ok()?;

        let changes = [
            "status",
            "--porcelain",
            "-z",
            "--untracked-files=no",
            "--",
            ".",
        ];
        let status = git(source, &changes).ok()?;
        Some(Self {
            commit: commit.trim().to_owned(),
            prefix: prefix.trim_end_matches('\n').to_owned(),
            clean: status.is_empty(),
        })
    }

    /// What it says once the run has read `source`: clean only if nothing moved meanwhile.
    pub(super) fn still(self, source: &Path) -> Self {
        let now = Self::of(source);
        Self {
            clean: self.clean && now.as_ref() == Some(&self),
            ..self
        }
    }

    fn is_commit(&self) -> bool {
        matches!(self.commit.len(), 40 | 64) && self.commit.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

/// Git can't read the runs' commits where it was asked to.
fn not_here(directory: &Path, error: String) -> String {
    format!(
        "{error}. Run this from a clone holding both runs' commits, not {}",
        directory.display()
    )
}

/// What git in `directory` says changed between the runs' commits, or why it can't say.
pub(super) fn commits(
    directory: &Path,
    (base, head): (Option<&Revision>, Option<&Revision>),
    (base_run, head_run): (&Snapshot, &Snapshot),
) -> Commits {
    let compared = || -> Result<Commits, String> {
        let (base, head) = (recorded("base", base)?, recorded("head", head)?);
        if base.prefix != head.prefix {
            return Err(format!(
                "the runs read different folders of their repository, `{}` and `{}`",
                base.prefix, head.prefix
            ));
        }
        read_only_what_is_committed(directory, ("base", base), base_run)?;
        read_only_what_is_committed(directory, ("head", head), head_run)?;
        let diff = [
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-renames",
            "--ignore-submodules=none",
            "--raw",
            "-z",
            &base.commit,
            &head.commit,
            "--",
        ];
        let raw = git(directory, &diff).map_err(|error| not_here(directory, error))?;
        let changed = changed_paths(&raw)?;
        Ok(Commits::Compared {
            prefix: base.prefix.clone(),
            changed,
        })
    };
    compared().unwrap_or_else(Commits::Unavailable)
}

/// The run's commit, if it says what the run read.
fn recorded<'a>(which: &str, revision: Option<&'a Revision>) -> Result<&'a Revision, String> {
    let Some(revision) = revision else {
        return Err(format!(
            "the {which} run doesn't say which commit it read. Save it again with this rstyle, \
             from a git checkout"
        ));
    };
    if !revision.is_commit() {
        return Err(format!(
            "the {which} run names `{}` as its commit, which isn't one",
            revision.commit
        ));
    }
    if !revision.clean {
        return Err(format!(
            "the {which} run's checkout had changes its commit doesn't hold, so the commit doesn't \
             say what it read. Commit them, then save the run again"
        ));
    }
    Ok(revision)
}

/// A file a run read that its commit doesn't hold, such as one not yet added, means the commit
/// isn't what the run read, whatever the checkout's status said.
fn read_only_what_is_committed(
    directory: &Path,
    (which, revision): (&str, &Revision),
    run: &Snapshot,
) -> Result<(), String> {
    let listing = ["ls-tree", "-r", "-z", "--name-only", "--full-tree"];
    let listing = [&listing[..], &[revision.commit.as_str()]].concat();
    let listed = git(directory, &listing).map_err(|error| not_here(directory, error))?;
    let tracked: BTreeSet<&str> = listed.split_terminator('\0').collect();

    let untracked: Vec<&str> = run
        .files()
        .filter(|path| !tracked.contains(format!("{}{path}", revision.prefix).as_str()))
        .collect();
    match untracked.first() {
        None => Ok(()),
        Some(first) => Err(format!(
            "the {which} run read {} files its commit doesn't hold, such as {first}. Commit them, \
             then save the run again",
            untracked.len()
        )),
    }
}

/// Each path `git diff --raw -z` lists, with what it is when it isn't a regular file.
fn changed_paths(raw: &str) -> Result<Vec<ChangedPath>, String> {
    let fields: Vec<&str> = raw.split_terminator('\0').collect();
    fields
        .chunks(2)
        .map(|record| match record {
            [header, path] => Ok(ChangedPath {
                path: (*path).to_owned(),
                special: special(header)?,
            }),
            _ => Err("git diff listed a file with no path".to_owned()),
        })
        .collect()
}

/// What a path is when either commit has it as something other than a regular file. Only a regular
/// file's text is all of what it is.
fn special(header: &str) -> Result<Option<&'static str>, String> {
    let fields: Vec<&str> = header.split(' ').collect();
    let [old, new, _, _, _] = fields.as_slice() else {
        return Err(format!(
            "git diff listed `{header}`, which isn't a file's metadata"
        ));
    };

    let modes = [old.trim_start_matches(':'), *new];
    let mut found = None;
    for mode in modes {
        found = found.or(match mode {
            "000000" | "100644" => None,
            "100755" => Some("executable"),
            "120000" => Some("symlink"),
            "160000" => Some("submodule"),
            other => return Err(format!("git diff listed a file of mode {other}")),
        });
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raw_diff_says_each_path_and_what_is_not_a_regular_file() {
        let zeros = "0000000000000000000000000000000000000000";
        let raw = format!(
            ":100644 100644 {zeros} {zeros} M\0src/lib.rs\0\
             :000000 100755 {zeros} {zeros} A\0scripts/test.sh\0\
             :120000 000000 {zeros} {zeros} D\0link\0\
             :160000 160000 {zeros} {zeros} M\0vendor/lib\0"
        );
        let changed = changed_paths(&raw).unwrap();
        let read: Vec<(&str, Option<&str>)> = changed
            .iter()
            .map(|change| (change.path.as_str(), change.special))
            .collect();
        assert_eq!(
            read,
            [
                ("src/lib.rs", None),
                ("scripts/test.sh", Some("executable")),
                ("link", Some("symlink")),
                ("vendor/lib", Some("submodule")),
            ]
        );
        assert!(changed_paths(":100644 100644 a b M\0").is_err());
        assert!(changed_paths(":100644 040000 a b M\0dir\0").is_err());
    }

    #[test]
    fn a_run_without_a_clean_commit_says_why_it_cannot_be_compared() {
        let revision = Revision {
            commit: "0a55f040".repeat(5),
            prefix: String::new(),
            clean: true,
        };
        assert!(recorded("base", Some(&revision)).is_ok());

        let missing = recorded("base", None).unwrap_err();
        assert!(missing.contains("doesn't say which commit"), "{missing}");
        let dirty = Revision {
            clean: false,
            ..revision.clone()
        };
        let error = recorded("head", Some(&dirty)).unwrap_err();
        assert!(
            error.starts_with("the head run's checkout had changes"),
            "{error}"
        );
        let option = Revision {
            commit: "--output=x".into(),
            ..revision
        };
        assert!(recorded("base", Some(&option)).is_err());
    }
}
