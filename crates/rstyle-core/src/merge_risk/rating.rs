//! How much a change matters, by the policy of the run it's compared from. What changed is
//! measured and how much that matters is configured, so a kind of change no rule covers stays
//! unrated instead of being given a severity, and what a comparison can't see stays unknown. Neither
//! reads as low.

use super::{ChangeKind, FunctionChange, Unread};
use crate::config::{RiskConfig, Severity};
use serde::Serialize;
use std::collections::BTreeMap;

const PDG_CHANGE: &str = "pdg_change";
const ADDED_CALL: &str = "added_call";
const UNRESOLVED_CALLS: &str = "unresolved_calls";
const UNCLASSIFIED_CALL: &str = "unclassified_call";
const ANALYSIS_UNAVAILABLE: &str = "analysis_unavailable";
/// What a reason says before why the changed files can't be listed.
const UNLISTED: &str = "changed files can't be listed: ";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Low,
    Medium,
    High,
    /// What a comparison can't see.
    Unknown,
    /// A kind of change nothing in the policy rates.
    Unrated,
}

impl Level {
    /// How far it pulls a whole change. A measured `high` outweighs not being able to see something,
    /// which outweighs nobody having said, which outweighs any severity short of `high`.
    fn weight(self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::Unrated => 3,
            Self::Unknown => 4,
            Self::High => 5,
        }
    }

    fn strongest(self, other: Self) -> Self {
        if other.weight() > self.weight() {
            other
        } else {
            self
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Unknown => "unknown",
            Self::Unrated => "unrated",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Low => "✅",
            Self::Medium | Self::High => "⚠️",
            Self::Unknown | Self::Unrated => "👀",
        }
    }
}

impl From<Severity> for Level {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Low => Self::Low,
            Severity::Medium => Self::Medium,
            Severity::High => Self::High,
        }
    }
}

/// What an `analysis_unavailable` reason couldn't see, so the verdict sums up each apart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum Unseen {
    /// Something a document holds that isn't compared.
    #[default]
    Compared,
    /// A changed file no document holds, or one nothing here reads.
    File,
    /// Which files changed at all, when the runs' commits couldn't say.
    Listing,
}

/// One thing that decided a rating.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Reason {
    pub rule: &'static str,
    pub level: Level,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callee: Option<String>,
    #[serde(skip)]
    unseen: Unseen,
}

impl Reason {
    fn at(change: &FunctionChange, rule: &'static str, level: Level, message: String) -> Self {
        Self {
            rule,
            level,
            message,
            path: Some(change.path.clone()),
            function: Some(change.name.clone()),
            line: Some(change.line),
            callee: None,
            unseen: Unseen::default(),
        }
    }

    fn about_callee(self, callee: &str) -> Self {
        Self {
            callee: Some(callee.to_owned()),
            ..self
        }
    }

    fn outside_functions(message: String, path: Option<String>) -> Self {
        Self {
            rule: ANALYSIS_UNAVAILABLE,
            level: Level::Unknown,
            message,
            path,
            function: None,
            line: None,
            callee: None,
            unseen: Unseen::default(),
        }
    }

    fn unread(file: &Unread) -> Self {
        let message = format!("it changed, and {}", file.why());
        Self {
            unseen: Unseen::File,
            ..Self::outside_functions(message, Some(file.path.clone()))
        }
    }

    fn unlisted(why: &str) -> Self {
        let message = format!("{UNLISTED}{why}");
        Self {
            unseen: Unseen::Listing,
            ..Self::outside_functions(message, None)
        }
    }
}

/// How much a whole change matters, and why.
#[derive(Debug, Serialize)]
pub struct Rating {
    pub risk: Level,
    /// Whether the policy has any risk rule, which tells a project with none from one missing some.
    pub policy_configured: bool,
    pub reasons: Vec<Reason>,
}

/// What a rating reads of a comparison.
pub(super) struct Measured<'a> {
    pub changes: &'a [FunctionChange],
    pub changed_outside_functions: &'a [String],
    pub not_compared_changed: usize,
    /// Changed files nothing here can see into, or why the runs' commits can't say which.
    pub unread: Result<&'a [Unread], &'a str>,
}

/// What a function's own change comes to, before how many others changed has a say.
fn impact(change: &FunctionChange, policy: &RiskConfig) -> Reason {
    let (level, message) = match change.kind {
        ChangeKind::Unknown => {
            let why = change.reason.unwrap_or("its tree isn't all of it");
            (Level::Unknown, why.to_owned())
        }
        ChangeKind::TextOnly => (
            Level::Unknown,
            "its text changed outside the compared facts".to_owned(),
        ),
        ChangeKind::Changed | ChangeKind::Added | ChangeKind::Removed => match policy.pdg_change {
            Some(severity) => (
                severity.into(),
                "what it depends on or does changed".to_owned(),
            ),
            None => (
                Level::Unrated,
                "what it depends on or does changed; no severity is configured".to_owned(),
            ),
        },
    };
    Reason::at(change, PDG_CHANGE, level, message)
}

fn function_reasons(change: &FunctionChange, policy: &RiskConfig) -> Vec<Reason> {
    let unresolved = change.unresolved_calls.then(|| {
        let message = "it makes a call whose target can't be known, so call rules are incomplete";
        Reason::at(change, UNRESOLVED_CALLS, Level::Unknown, message.to_owned())
    });
    let added = policy
        .calls
        .iter()
        .filter(|rule| change.calls_added.contains(&rule.callee))
        .map(|rule| {
            let message = format!("added call: {}", rule.callee);
            Reason::at(change, ADDED_CALL, rule.level.into(), message).about_callee(&rule.callee)
        });
    // A callee with a rule has been said something about, by the project.
    let unclassified = change
        .calls_unclassified
        .iter()
        .filter(|callee| !policy.calls.iter().any(|rule| rule.callee == **callee))
        .map(|callee| {
            let message = format!("new library call nothing classifies: {callee}");
            Reason::at(change, UNCLASSIFIED_CALL, Level::Unknown, message).about_callee(callee)
        });

    std::iter::once(impact(change, policy))
        .chain(unresolved)
        .chain(added)
        .chain(unclassified)
        .collect()
}

/// A medium change to few enough functions is low. This is a policy on size, not a proof that
/// behavior is preserved, and it only lowers what's measured as a change.
fn within_size_limit(reasons: &mut [Reason], changed: usize, policy: &RiskConfig) {
    let limit = policy.pdg_low_risk_max_functions;
    if limit == 0 || changed > limit {
        return;
    }
    let medium = reasons
        .iter_mut()
        .filter(|reason| reason.rule == PDG_CHANGE && reason.level == Level::Medium);
    for reason in medium {
        reason.level = Level::Low;
        reason.message = format!("the change is within the low-risk limit of {limit} functions");
    }
}

/// What no compared function says: a file that changed outside any function, what a macro or a
/// build writes, and each changed file nothing here can see into, or why they can't be listed.
fn outside_reasons(measured: &Measured<'_>) -> Vec<Reason> {
    let files = measured.changed_outside_functions.iter().map(|path| {
        let message = "it changed outside any function, and no fact here says what that does";
        Reason::outside_functions(message.to_owned(), Some(path.clone()))
    });
    let generated = (measured.not_compared_changed > 0).then(|| {
        let message = format!(
            "{} functions a macro or a build writes changed, and they aren't compared",
            measured.not_compared_changed
        );
        Reason::outside_functions(message, None)
    });
    let unread = match measured.unread {
        Ok(files) => files.iter().map(Reason::unread).collect(),
        Err(why) => vec![Reason::unlisted(why)],
    };
    files.chain(generated).chain(unread).collect()
}

pub(super) fn rate(measured: &Measured<'_>, policy: &RiskConfig) -> Rating {
    let mut reasons: Vec<Reason> = measured
        .changes
        .iter()
        .flat_map(|change| function_reasons(change, policy))
        .collect();
    within_size_limit(&mut reasons, measured.changes.len(), policy);
    reasons.extend(outside_reasons(measured));

    let policy_configured = policy.is_set();
    let measured_risk = reasons
        .iter()
        .fold(Level::Low, |risk, reason| risk.strongest(reason.level));
    Rating {
        risk: if policy_configured {
            measured_risk
        } else {
            Level::Unrated
        },
        policy_configured,
        reasons,
    }
}

impl Rating {
    /// What can't be seen, by kind: the report below says which functions and files.
    fn unknown_lines(&self) -> Vec<String> {
        let mut by_kind: BTreeMap<(&str, Unseen), usize> = BTreeMap::new();
        for reason in self.reasons.iter().filter(|r| r.level == Level::Unknown) {
            *by_kind.entry((reason.rule, reason.unseen)).or_default() += 1;
        }
        by_kind
            .into_iter()
            .map(|((rule, unseen), count)| match (rule, unseen) {
                (PDG_CHANGE, _) => {
                    format!("- changes the compared facts can't speak for: {count}")
                }
                (UNRESOLVED_CALLS, _) => {
                    format!("- functions with a call to an unknown target: {count}")
                }
                (UNCLASSIFIED_CALL, _) => {
                    format!("- new library calls nothing classifies: {count}")
                }
                (_, Unseen::File) => format!("- changed files nothing here can see into: {count}"),
                (_, Unseen::Listing) => {
                    format!("- {UNLISTED}{}", self.unlisted().unwrap_or_default())
                }
                (_, Unseen::Compared) => format!("- changes outside what's compared: {count}"),
            })
            .collect()
    }

    /// Why the changed files can't be listed, if they can't.
    pub(super) fn unlisted(&self) -> Option<&str> {
        let reason = self.reasons.iter().find(|r| r.unseen == Unseen::Listing)?;
        reason.message.strip_prefix(UNLISTED)
    }

    /// Who hasn't said what, which tells a project with no rules from one missing some.
    fn unrated_line(&self) -> String {
        if self.policy_configured {
            "The base's policy has no severity for code impact.".to_owned()
        } else {
            "The base's policy has no risk rules.".to_owned()
        }
    }

    /// The calls a rule named at this rating, since those are why it's this one.
    fn call_lines(&self) -> Vec<String> {
        let at_rating = self
            .reasons
            .iter()
            .filter(|reason| reason.rule == ADDED_CALL && reason.level == self.risk);
        at_rating
            .map(|reason| {
                let path = reason.path.as_deref().unwrap_or_default();
                let line = reason.line.unwrap_or_default();
                let function = reason.function.as_deref().unwrap_or_default();
                format!("- {path}:{line}: {function}: {}", reason.message)
            })
            .collect()
    }

    fn code_impact(&self, changed: usize) -> (&'static str, &'static str) {
        let partial = self.reasons.iter().any(|reason| {
            reason.level == Level::Unknown
                && matches!(reason.rule, PDG_CHANGE | ANALYSIS_UNAVAILABLE)
        });
        match (partial, changed) {
            (true, _) => ("partial", "👀"),
            (false, 0) => ("unchanged", "✅"),
            (false, _) => ("changed", "👀"),
        }
    }

    /// The verdict, and what it rests on.
    pub(super) fn lines(&self, changed: usize) -> Vec<String> {
        let heading = format!("Merge risk: {} {}", self.risk.name(), self.risk.icon());
        let details = match self.risk {
            Level::Unknown => self.unknown_lines(),
            Level::Unrated => vec![self.unrated_line()],
            Level::Low | Level::Medium | Level::High => self.call_lines(),
        };
        let (verdict, icon) = self.code_impact(changed);
        let impact = format!("Code impact: {verdict} {icon}");
        let affected = format!("Affected functions: {changed}.");

        std::iter::once(heading)
            .chain(details)
            .chain([String::new(), impact, affected, String::new()])
            .collect()
    }
}

#[cfg(test)]
mod tests;
