//! Changed files select source-rule presentation; they never grant source evidence.
use super::Coverage;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::Path};

const SCHEMA: &str = "rstyle.changed-files/1";
const MAX_BYTES: usize = 1024 * 1024;
const MAX_PATHS: usize = 4096;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug)]
pub(super) struct Selection {
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

pub(super) struct Owner<'a> {
    pub path: Option<&'a str>,
    pub complete: bool,
    pub visible_findings: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct Report<'a> {
    pub coverage: Coverage,
    pub visible_findings: usize,
    pub unclassified_functions: usize,
    pub paths: Vec<PathReport<'a>>,
}

#[derive(Debug, Serialize)]
pub(super) struct PathReport<'a> {
    pub path: &'a str,
    pub matched_functions: usize,
    pub unavailable_functions: usize,
    pub visible_findings: usize,
    pub coverage: Coverage,
    pub status: Status,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Status {
    Observed,
    Unobserved,
}

impl<'a> PathReport<'a> {
    fn unobserved(path: &'a str) -> Self {
        Self {
            path,
            matched_functions: 0,
            unavailable_functions: 0,
            visible_findings: 0,
            coverage: Coverage::Incomplete,
            status: Status::Unobserved,
        }
    }

    fn include(&mut self, owner: Owner<'_>) {
        self.matched_functions += 1;
        self.unavailable_functions += usize::from(!owner.complete);
        self.visible_findings += owner.visible_findings;
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

pub(super) fn evaluate<'selection, 'owner>(
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
        if let Ok(index) = paths.binary_search_by_key(&path, |entry| entry.path) {
            paths[index].include(owner);
        }
    }
    unclassified
}

impl Report<'_> {
    pub fn permits_success(&self) -> bool {
        self.coverage == Coverage::Complete
    }

    pub fn render(&self) -> String {
        let mut output = format!(
            "\nChanged-file source rules: {:?} coverage; {} selected files, {} visible findings; {} unclassified functions.",
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
        }
        output
    }
}

#[cfg(test)]
mod tests;
