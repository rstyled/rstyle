//! What a reviewer reads first: each check's status, one line on what it found, and every item
//! behind that line. A check that can't see something says unknown, never fine.

use super::{Links, code, nonzero, place, plural, tally, what_changed};
use crate::merge_risk::{ChangeKind, ChangeReport, FunctionChange};
use std::collections::BTreeSet;

mod calls;
mod effects;
mod errors;

pub(super) use calls::calls;
pub(super) use effects::effects;
pub(super) use errors::errors;

/// The most production files a change touches and still reads as small: in a backtest of rstyle's
/// pull requests, few that touched more still reverted cleanly a week later.
const SMALL_CHANGE_FILES: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Status {
    Fine,
    Look,
    Unknown,
}

impl Status {
    /// Something to look at wins over something unknown, and either over nothing.
    fn of(look: bool, unknown: bool) -> Self {
        match (look, unknown) {
            (true, _) => Self::Look,
            (false, true) => Self::Unknown,
            (false, false) => Self::Fine,
        }
    }

    pub(super) fn icon(self) -> &'static str {
        match self {
            Self::Fine => "✅",
            Self::Look => "👀",
            Self::Unknown => "❓",
        }
    }
}

pub(super) struct Check {
    pub name: &'static str,
    pub status: Status,
    pub summary: String,
    pub items: Vec<String>,
}

impl Check {
    /// A check that found nothing, and says what it looked at.
    fn fine(name: &'static str, summary: &str, looked_at: String) -> Self {
        Self::new(name, Status::Fine, summary.into(), vec![looked_at])
    }

    fn new(name: &'static str, status: Status, summary: String, items: Vec<String>) -> Self {
        Self {
            name,
            status,
            summary,
            items,
        }
    }
}

/// How big the change is, and every function it touches.
pub(super) fn changed(report: &ChangeReport, links: &Links) -> Check {
    let files: BTreeSet<&str> = report
        .changes
        .iter()
        .map(|change| change.path.as_str())
        .chain(report.changed_outside_functions.iter().map(String::as_str))
        .collect();
    let items = report
        .changes
        .iter()
        .map(|change| format!("{}: {}", place(change, links), what_changed(change)))
        .chain(outside_items(report, links))
        .collect();
    let summary = match report.changes.is_empty() {
        true => format!(
            "no function changed, in {}",
            plural(files.len(), "file", "files")
        ),
        false => format!(
            "{} in {}: {}",
            plural(report.changes.len(), "function", "functions"),
            plural(files.len(), "file", "files"),
            tally(&report.changes)
        ),
    };

    let unseen = report
        .changes
        .iter()
        .any(|change| change.kind == ChangeKind::Unknown)
        || !report.changed_outside_functions.is_empty()
        || report.not_compared_changed > 0;
    let (status, summary) = match (unseen, files.len() > SMALL_CHANGE_FILES) {
        (true, _) => (Status::Unknown, summary),
        (false, true) => (
            Status::Look,
            format!("{summary} (over {SMALL_CHANGE_FILES} files rarely reverts cleanly)"),
        ),
        (false, false) => (Status::Fine, summary),
    };
    Check::new("Changed", status, summary, items)
}

/// What no function's line says: files changed outside any function, and what isn't compared.
fn outside_items(report: &ChangeReport, links: &Links) -> Vec<String> {
    let files = report
        .changed_outside_functions
        .iter()
        .map(|path| format!("{}: changed outside any function", links.file(path)));
    let generated = nonzero(
        report.not_compared_changed,
        "generated function changed",
        "generated functions changed",
    )
    .map(|count| format!("{count}, which a macro or a build writes and nothing compares"));
    let tests = nonzero(report.tests_changed, "test function", "test functions")
        .map(|count| format!("{count} changed, which aren't rated"));
    files.chain(generated).chain(tests).collect()
}

/// Every function whose parameters or results changed, before and after.
pub(super) fn signatures(report: &ChangeReport, links: &Links) -> Check {
    let reshaped: Vec<&FunctionChange> = report
        .changes
        .iter()
        .filter(|change| change.signature_changed)
        .collect();
    if reshaped.is_empty() {
        let compared = plural(
            report.changes.len(),
            "changed function",
            "changed functions",
        );
        let item = format!("none of the {compared} changed its parameters or results");
        return Check::fine("Signatures", "none changed", item);
    }

    let callers: usize = reshaped.iter().map(|change| change.callers).sum();
    let items = reshaped
        .iter()
        .map(|change| reshaped_item(change, links))
        .collect();
    let summary = format!(
        "{} changed, {}",
        reshaped.len(),
        plural(callers, "caller", "callers")
    );
    Check::new("Signatures", Status::Look, summary, items)
}

fn reshaped_item(change: &FunctionChange, links: &Links) -> String {
    let callers = plural(change.callers, "caller", "callers");
    match &change.signature {
        Some(signature) => format!(
            "{}: {} → {} ({callers})",
            place(change, links),
            code(&signature.before),
            code(&signature.after)
        ),
        None => format!("{}: signature changed ({callers})", place(change, links)),
    }
}

/// Counts as a reader says them, the ones with none left out: `2 gained, 1 lost`.
fn counted(counts: &[(usize, &str)]) -> Vec<String> {
    let said = counts.iter().filter(|(count, _)| *count > 0);
    said.map(|(count, label)| format!("{count} {label}"))
        .collect()
}
