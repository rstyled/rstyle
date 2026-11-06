//! What compiler reports share about whether requested work completed, and the changed-file
//! selection that scopes them.
pub(super) mod changed_scope;

use super::Args;
use serde::Serialize;
use std::path::Path;

/// Whether requested work has enough completed coverage to permit a clean exit.
/// Implementations retain their consumer-specific execution and scope rules.
pub(in crate::compiler) trait Completion {
    fn permits_success(&self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    Complete,
    Incomplete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Execution {
    NotRequested,
    Completed,
}

pub(super) struct Config {
    changed_files: changed_scope::Selection,
}

impl Config {
    pub(in crate::compiler) fn changed_files(&self) -> Option<&changed_scope::Selection> {
        Some(&self.changed_files)
    }
}

pub(super) fn configuration(args: &Args) -> Result<Option<Config>, String> {
    args.changed_files
        .as_deref()
        .map(changed_scope::Selection::load)
        .transpose()
        .map(|selection| selection.map(|changed_files| Config { changed_files }))
}

pub(super) fn source_file(function: &super::Function) -> Option<&str> {
    function.facts.view().source_file
}

pub(in crate::compiler) fn checked_path<'a>(original: &'a str, root: &Path) -> Option<&'a str> {
    let root_text = root.to_str()?;
    if original == "/" || !valid_absolute(original) || !valid_absolute(root_text) {
        return None;
    }
    match Path::new(original).strip_prefix(root) {
        Ok(relative) => relative.to_str().filter(|path| !path.is_empty()),
        // A checked absolute path cannot match a relative manifest entry.
        Err(_) => Some(original),
    }
}

fn valid_absolute(path: &str) -> bool {
    let Some(relative) = path.strip_prefix('/') else {
        return false;
    };
    !path.contains(['\\', ':'])
        && !path.chars().any(char::is_control)
        && (relative.is_empty()
            || relative
                .split('/')
                .all(|part| !matches!(part, "" | "." | "..")))
}
