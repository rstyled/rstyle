//! The report as Markdown, for a pull request's description: the rating and why it's that one
//! first, the functions behind it folded away, and what needs a person in plain sight.

use super::rating::{ADDED_CALL, Level, Reason};
use super::{ChangeKind, ChangeReport, EffectChange, FunctionChange};
use std::collections::BTreeSet;

impl ChangeReport {
    /// `source_url` is what a file's path is joined to for a link, such as
    /// `https://github.com/OWNER/REPO/blob/HEAD`. Without it, paths aren't links. A removed
    /// function is never one, since the head doesn't hold it.
    pub fn markdown(&self, source_url: Option<&str>) -> String {
        let links = Links(source_url.map(|url| url.trim_end_matches('/')));
        [
            Some(self.verdict(&links)),
            Some(self.impact()),
            self.outside(&links),
            self.functions(&links),
            Some(self.counts()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n\n")
    }

    fn verdict(&self, links: &Links) -> String {
        let risk = self.rating.risk;
        let heading = format!("**Merge risk: {} {}**", risk.name(), risk.icon());
        let why = match risk {
            Level::Unknown => self.rating.unknown_lines(),
            Level::Unrated => vec![format!("- {}", self.rating.unrated_line())],
            Level::Low | Level::Medium | Level::High => self.decisive(links),
        };
        std::iter::once(heading)
            .chain(why)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The reasons at the rating, since those are why it's this one. A call a rule names is said
    /// where it's made; any other reason is said once, however many functions share it.
    fn decisive(&self, links: &Links) -> Vec<String> {
        let at_rating = self
            .rating
            .reasons
            .iter()
            .filter(|reason| reason.level == self.rating.risk);
        let mut said = BTreeSet::new();
        at_rating
            .filter_map(|reason| match reason.rule {
                ADDED_CALL => Some(added_call(reason, links)),
                _ => said
                    .insert(reason.message.as_str())
                    .then(|| format!("- {}", escape(&reason.message))),
            })
            .collect()
    }

    fn impact(&self) -> String {
        let (verdict, icon) = self.rating.code_impact(self.changes.len());
        let kinds = [
            ChangeKind::Changed,
            ChangeKind::Added,
            ChangeKind::Removed,
            ChangeKind::TextOnly,
            ChangeKind::Unknown,
        ];

        let counts = kinds
            .into_iter()
            .map(|kind| (self.count(kind), kind))
            .filter(|(count, _)| *count > 0)
            .map(|(count, kind)| format!("{count} {}", kind.name()));
        let unchanged = std::iter::once(format!("{} unchanged", self.unchanged));
        let tally = counts.chain(unchanged).collect::<Vec<_>>().join(", ");

        format!("**Code impact:** {verdict} {icon} · {tally}")
    }

    fn outside(&self, links: &Links) -> Option<String> {
        if self.changed_outside_functions.is_empty() {
            return None;
        }
        let files = self
            .changed_outside_functions
            .iter()
            .map(|path| format!("- {}", links.file(path)));
        let heading = "**Changed outside any function**, which no fact here says the effect of:";
        Some(
            std::iter::once(heading.to_owned())
                .chain(files)
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    fn functions(&self, links: &Links) -> Option<String> {
        let count = self.changes.len();
        if count == 0 {
            return None;
        }

        let noun = if count == 1 { "function" } else { "functions" };
        let summary = format!("<details><summary>{count} affected {noun}</summary>\n");
        let changes = self
            .changes
            .iter()
            .flat_map(|change| function_lines(change, links));
        let lines = std::iter::once(summary)
            .chain(changes)
            .chain(["\n</details>".to_owned()]);

        Some(lines.collect::<Vec<_>>().join("\n"))
    }

    fn counts(&self) -> String {
        format!(
            "Test functions changed: {} · generated functions not compared: {}, {} of them changed",
            self.tests_changed, self.not_compared, self.not_compared_changed
        )
    }
}

fn added_call(reason: &Reason, links: &Links) -> String {
    let callee = code(&short(reason.callee.as_deref().unwrap_or_default()));
    let function = reason.function.as_deref().unwrap_or_default();
    let place = match (&reason.path, reason.line) {
        (Some(path), line) => links.function(function, path, line),
        (None, _) => code(function),
    };
    format!("- added call {callee} in {place}")
}

fn function_lines(change: &FunctionChange, links: &Links) -> Vec<String> {
    let details = function_details(change).into_iter();
    std::iter::once(function_summary(change, links))
        .chain(details.map(|detail| format!("  - {detail}")))
        .collect()
}

fn function_summary(change: &FunctionChange, links: &Links) -> String {
    let name = match change.kind {
        ChangeKind::Removed => Links(None).function(&change.name, &change.path, None),
        _ => links.function(&change.name, &change.path, Some(change.line)),
    };
    let callers = if change.callers == 1 {
        "caller"
    } else {
        "callers"
    };

    format!(
        "- {name} {} · {} {callers}, reached by {}",
        change.kind.name(),
        change.callers,
        change.reached_by
    )
}

fn function_details(change: &FunctionChange) -> Vec<String> {
    let facts = [
        change.reason.map(escape),
        change
            .signature_changed
            .then(|| "signature changed".to_owned()),
        change
            .values_changed
            .then(|| "a known value changed, as a constant's does".to_owned()),
        change
            .unresolved_calls
            .then(|| "makes a call whose target can't be known".to_owned()),
        listed(
            "new library calls nothing classifies",
            &change.calls_unclassified,
        ),
    ];
    // All of a new or removed function's calls are what it is, not what changed.
    let calls = match change.kind {
        ChangeKind::Added | ChangeKind::Removed => [None, None],
        _ => [
            listed("calls more", &change.calls_added),
            listed("calls less", &change.calls_removed),
        ],
    };

    let effects = [
        effect_details("gains", &change.effects_gained),
        effect_details("loses", &change.effects_lost),
    ];
    facts
        .into_iter()
        .flatten()
        .chain(effects.into_iter().flatten())
        .chain(calls.into_iter().flatten())
        .collect()
}

/// One line per effect kind gained or lost, naming what causes it.
fn effect_details(verb: &str, effects: &[EffectChange]) -> Vec<String> {
    let kinds: BTreeSet<&str> = effects.iter().map(|effect| effect.kind).collect();
    kinds
        .into_iter()
        .filter_map(|kind| {
            let labels: Vec<String> = effects
                .iter()
                .filter(|effect| effect.kind == kind)
                .map(|effect| effect.label.clone())
                .collect();
            listed(&format!("{verb} {kind}"), &labels)
        })
        .collect()
}

fn listed(label: &str, callees: &[String]) -> Option<String> {
    if callees.is_empty() {
        return None;
    }
    let names: Vec<String> = callees.iter().map(|callee| code(&short(callee))).collect();
    Some(format!("{label}: {}", names.join(", ")))
}

/// A callee without the directories of its import path, which a Go identity spells in full:
/// `go::github.com/o/r/native.f` reads as `go::native.f`, and `go::(io/fs.FileInfo).Size` as
/// `go::(fs.FileInfo).Size`. The text and JSON reports keep the identity a rule names.
fn short(identity: &str) -> String {
    let mut shortened = String::with_capacity(identity.len());
    let mut segment = String::new();
    for character in identity.chars() {
        match character {
            '/' => segment.clear(),
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '-' | '_' | '~' => segment.push(character),
            _ => {
                shortened.push_str(&segment);
                segment.clear();
                shortened.push(character);
            }
        }
    }

    shortened.push_str(&segment);
    shortened
}

/// Where the report links a file to, if anywhere.
struct Links<'a>(Option<&'a str>);

impl Links<'_> {
    /// `text` as a link to a line of a file, or as it is.
    fn to(&self, text: String, path: &str, line: Option<usize>) -> String {
        let Some(root) = self.0 else {
            return text;
        };
        let anchor = line.map(|line| format!("#L{line}")).unwrap_or_default();
        format!("[{text}]({root}/{}{anchor})", path.replace(' ', "%20"))
    }

    fn file(&self, path: &str) -> String {
        self.to(code(path), path, None)
    }

    fn function(&self, name: &str, path: &str, line: Option<usize>) -> String {
        format!("{} in {}", self.to(code(name), path, line), code(path))
    }
}

/// A code span, fenced with more backticks than the text holds in a row.
fn code(text: &str) -> String {
    let longest = text
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    if longest == 0 {
        return format!("`{text}`");
    }
    let fence = "`".repeat(longest + 1);
    format!("{fence} {text} {fence}")
}

/// Text with what Markdown or HTML would read as syntax escaped.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if "\\`*_[]<>#|~".contains(character) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(test)]
mod tests;
