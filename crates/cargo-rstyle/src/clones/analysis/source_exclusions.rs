//! Workspace globs apply only to checked original source paths.
use super::super::Function;
use rstyle_core::exclusions::Matcher;
use rstyle_pdg::consumer_facts::{Availability, LocalFacts};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum Report {
    Included { path: String },
    Excluded { path: String },
    NotApplicable { path: String, reason: String },
    Unavailable { reason: String },
}

pub(super) fn evaluate(
    facts: &LocalFacts<'_>,
    function: &Function,
    root: &Path,
    matcher: &Matcher,
) -> Report {
    evaluate_source(&facts.source, function, root, matcher)
}

pub(super) fn evaluate_source(
    source: &Availability<&str>,
    function: &Function,
    root: &Path,
    matcher: &Matcher,
) -> Report {
    if let Some(reason) = unavailable_source(source) {
        return Report::Unavailable { reason };
    }
    let path = function
        .facts
        .view()
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str);
    match path {
        Some(path) => report_for_path(path, root, matcher),
        None => Report::Unavailable {
            reason: "Original source path was not captured".into(),
        },
    }
}

fn unavailable_source(source: &Availability<&str>) -> Option<String> {
    match source {
        Availability::Known(_) => None,
        Availability::Unknown { reason }
        | Availability::Unsupported { reason }
        | Availability::NotApplicable { reason } => Some(reason.clone()),
    }
}

fn report_for_path(path: &str, root: &Path, matcher: &Matcher) -> Report {
    let original = Path::new(path);
    if !valid_absolute(original) || !valid_absolute(root) {
        return Report::Unavailable {
            reason: "Source exclusion requires absolute original and workspace paths without dot components".into(),
        };
    }
    let Ok(relative) = original.strip_prefix(root) else {
        return Report::NotApplicable {
            path: path.into(),
            reason:
                "Original source is outside the workspace root; workspace exclusions do not apply"
                    .into(),
        };
    };
    let Some(relative) = relative.to_str().filter(|relative| !relative.is_empty()) else {
        return Report::Unavailable {
            reason: "Original source has no valid workspace-relative file path".into(),
        };
    };

    let path = relative.replace(std::path::MAIN_SEPARATOR, "/");
    if matcher.is_excluded(Path::new(&path)) {
        Report::Excluded { path }
    } else {
        Report::Included { path }
    }
}

fn valid_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.to_str().is_some_and(|text| {
            !text
                .split(std::path::MAIN_SEPARATOR)
                .any(|part| matches!(part, "." | ".."))
        })
}

impl Report {
    pub(super) fn excluded(&self) -> bool {
        matches!(self, Self::Excluded { .. })
    }

    pub(super) fn complete(&self) -> bool {
        !matches!(self, Self::Unavailable { .. })
    }
}

#[cfg(test)]
mod tests;
