//! Changed files select consumer presentation; they never grant source evidence.
use super::{Coverage, Execution};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fs::File, io::Read, path::Path};

const SCHEMA: &str = "rstyle.changed-files/1";
const MAX_BYTES: usize = 1024 * 1024;
const MAX_PATHS: usize = 4096;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug)]
pub(in crate::clones) struct Selection {
    paths: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    paths: Vec<String>,
}

impl Selection {
    pub fn load(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        File::open(path)
            .and_then(|file| file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes))
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Self::from_bytes(&bytes).map_err(|error| format!("{}: {error}", path.display()))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("Changed-file manifest exceeds the 1 MiB byte limit".into());
        }
        let manifest: Manifest =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if manifest.schema != SCHEMA {
            return Err("Unsupported changed-file manifest schema".into());
        }
        if manifest.paths.len() > MAX_PATHS {
            return Err("Changed-file manifest exceeds the 4096-path limit".into());
        }
        for path in &manifest.paths {
            validate_path(path)?;
        }

        let mut paths = manifest.paths;
        paths.sort_unstable();
        paths.dedup();
        Ok(Self { paths })
    }

    pub fn contains(&self, path: &str) -> bool {
        self.paths
            .binary_search_by(|selected| selected.as_str().cmp(path))
            .is_ok()
    }
}

fn validate_path(path: &str) -> Result<(), String> {
    if path.len() > MAX_PATH_BYTES
        || !path.ends_with(".rs")
        || path.contains(['\\', ':'])
        || path.chars().any(char::is_control)
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(
            "Changed-file paths must be normalized relative Rust paths of at most 4096 bytes"
                .into(),
        );
    }
    Ok(())
}

pub(in crate::clones) struct Owner<'a> {
    pub path: Option<&'a str>,
    pub complete: bool,
    pub outside_scope: bool,
    pub visible_findings: usize,
}

#[derive(Debug, Serialize)]
pub(in crate::clones) struct Report<'a> {
    pub coverage: Coverage,
    pub visible_findings: usize,
    pub unclassified_functions: usize,
    pub paths: Vec<PathReport<'a>>,
}

#[derive(Debug, Serialize)]
pub(in crate::clones) struct PathReport<'a> {
    pub path: Cow<'a, str>,
    pub matched_functions: usize,
    pub unavailable_functions: usize,
    #[serde(skip_serializing_if = "is_zero")]
    pub outside_scope_functions: usize,
    pub visible_findings: usize,
    pub coverage: Coverage,
    pub status: Status,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::clones) enum Status {
    Observed,
    Unobserved,
}

impl<'a> PathReport<'a> {
    fn unobserved(path: &'a str) -> Self {
        Self {
            path: Cow::Borrowed(path),
            matched_functions: 0,
            unavailable_functions: 0,
            outside_scope_functions: 0,
            visible_findings: 0,
            coverage: Coverage::Incomplete,
            status: Status::Unobserved,
        }
    }

    fn include(&mut self, owner: Owner<'_>) {
        self.include_coverage(&owner);
        self.visible_findings += owner.visible_findings;
    }

    fn include_coverage(&mut self, owner: &Owner<'_>) {
        self.matched_functions += 1;
        self.unavailable_functions += usize::from(!owner.complete && !owner.outside_scope);
        self.outside_scope_functions += usize::from(owner.outside_scope);
    }

    fn finish(&mut self) {
        self.status = if self.matched_functions > 0 {
            Status::Observed
        } else {
            Status::Unobserved
        };
        self.coverage = if self.status == Status::Observed && self.unavailable_functions == 0 {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        };
    }
}

pub(in crate::clones) fn evaluate<'selection, 'owner>(
    selection: &'selection Selection,
    owners: impl IntoIterator<Item = Owner<'owner>>,
) -> Report<'selection> {
    let mut paths: Vec<_> = selection
        .paths
        .iter()
        .map(|path| PathReport::unobserved(path))
        .collect();
    let unclassified_functions = include_owners(&mut paths, owners);
    for path in &mut paths {
        path.finish();
    }

    Report {
        coverage: if unclassified_functions == 0
            && paths.iter().all(|path| path.coverage == Coverage::Complete)
        {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        },
        visible_findings: paths.iter().map(|path| path.visible_findings).sum(),
        unclassified_functions,
        paths,
    }
}

fn include_owners<'a>(
    paths: &mut [PathReport<'_>],
    owners: impl IntoIterator<Item = Owner<'a>>,
) -> usize {
    let mut unclassified = 0;
    for owner in owners {
        let Some(path) = owner.path else {
            unclassified += usize::from(!paths.is_empty());
            continue;
        };
        if let Ok(index) = paths.binary_search_by(|entry| entry.path.as_ref().cmp(path)) {
            paths[index].include(owner);
        }
    }
    unclassified
}

/// Selected presentation cannot establish completion for an incomplete consumer.
pub(super) fn permits_success(
    execution: Execution,
    coverage: Coverage,
    selection: Option<&Report<'_>>,
) -> bool {
    execution == Execution::NotRequested
        || (execution == Execution::Completed
            && coverage == Coverage::Complete
            && selection.is_none_or(Report::permits_success))
}

impl Report<'_> {
    pub(in crate::clones) fn include_findings<'a>(&mut self, paths: impl Iterator<Item = &'a str>) {
        for path in paths {
            if let Ok(index) = self
                .paths
                .binary_search_by(|entry| entry.path.as_ref().cmp(path))
            {
                self.paths[index].visible_findings += 1;
                self.visible_findings += 1;
            }
        }
    }

    pub(in crate::clones) fn into_owned(self) -> Report<'static> {
        Report {
            coverage: self.coverage,
            visible_findings: self.visible_findings,
            unclassified_functions: self.unclassified_functions,
            paths: self
                .paths
                .into_iter()
                .map(|path| PathReport {
                    path: Cow::Owned(path.path.into_owned()),
                    matched_functions: path.matched_functions,
                    unavailable_functions: path.unavailable_functions,
                    outside_scope_functions: path.outside_scope_functions,
                    visible_findings: path.visible_findings,
                    coverage: path.coverage,
                    status: path.status,
                })
                .collect(),
        }
    }

    pub fn permits_success(&self) -> bool {
        self.coverage == Coverage::Complete
    }

    pub fn render(&self) -> String {
        self.render_for("source rules")
    }

    pub(in crate::clones) fn render_for(&self, consumer: &str) -> String {
        let mut output = format!(
            "\nChanged-file {consumer}: {:?} coverage; {} selected files, {} visible findings; {} unclassified functions.",
            self.coverage,
            self.paths.len(),
            self.visible_findings,
            self.unclassified_functions
        );
        for path in &self.paths {
            output.push_str(&format!(
                "\n{}: {:?}, {:?} coverage; {} matched functions, {} unavailable; {} visible findings.",
                path.path, path.status, path.coverage, path.matched_functions,
                path.unavailable_functions, path.visible_findings
            ));
            if path.outside_scope_functions > 0 {
                output.push_str(&format!(
                    " {} functions outside requested scope.",
                    path.outside_scope_functions
                ));
            }
        }
        output
    }
}

#[cfg(test)]
mod tests;
