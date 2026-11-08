//! The report as Markdown, for a pull request's description. The rating and why come first, then
//! the functions that need a look, then one folded section per kind of fact. Each section's summary
//! holds its verdict, so the report reads folded.

use super::rating::{ADDED_CALL, Level, Reason};
use super::{ChangeKind, ChangeReport, EffectChange, FunctionChange};
use std::collections::{BTreeMap, BTreeSet};

/// How many functions "Needs a look" names before pointing at the sections.
const LOOK_LIMIT: usize = 10;

/// What every section lists functions by: changed, added, removed, text only, unknown.
const KINDS: [ChangeKind; 5] = [
    ChangeKind::Changed,
    ChangeKind::Added,
    ChangeKind::Removed,
    ChangeKind::TextOnly,
    ChangeKind::Unknown,
];

impl ChangeReport {
    /// `source_url` is what a file's path is joined to for a link, such as
    /// `https://github.com/OWNER/REPO/blob/HEAD`. Without it, paths aren't links. A removed
    /// function is never one, since the head doesn't hold it.
    pub fn markdown(&self, source_url: Option<&str>) -> String {
        let links = Links(source_url.map(|url| url.trim_end_matches('/')));
        let sections = [
            self.impact_section(&links),
            self.calls_section(&links),
            self.signatures_section(&links),
            self.effects_section(&links),
            self.blind_spots_section(&links),
        ];

        [Some(self.verdict(&links)), self.needs_a_look(&links)]
            .into_iter()
            .flatten()
            .chain(sections.into_iter().map(Section::render))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn verdict(&self, links: &Links) -> String {
        let risk = self.rating.risk;
        let heading = format!("### Merge risk: {} {}", risk.name(), icon(risk));
        let why: Vec<String> = match risk {
            Level::Unknown => self.rating.unknown_lines(),
            Level::Unrated => vec![self.rating.unrated_line()],
            Level::Low | Level::Medium | Level::High => self.decisive(links),
        };

        let why = match why.as_slice() {
            [one] => format!("{}.", one.trim_start_matches("- ")),
            many => many.join("\n"),
        };
        format!("{heading}\n\n{why}")
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
                ADDED_CALL => Some(format!("- {}", added_call(reason, links))),
                _ => said
                    .insert(reason.message.as_str())
                    .then(|| format!("- {}", escape(&reason.message))),
            })
            .collect()
    }

    /// Each function a person should read first: what a rule names, what can't be seen, and what
    /// changed in what it takes, holds or does, in the order the report puts them.
    fn needs_a_look(&self, links: &Links) -> Option<String> {
        let flagged: Vec<String> = self
            .changes
            .iter()
            .filter_map(|change| look_line(change, &self.rule_hits(change), links))
            .collect();
        if flagged.is_empty() {
            return None;
        }

        let more = flagged.len().saturating_sub(LOOK_LIMIT);
        let rest = (more > 0).then(|| format!("\n\n…and {more} more in the sections below."));
        let lines = flagged
            .iter()
            .take(LOOK_LIMIT)
            .map(|line| format!("1. {line}"));
        let list = lines.collect::<Vec<_>>().join("\n");
        Some(format!(
            "**Needs a look**\n\n{list}{}",
            rest.unwrap_or_default()
        ))
    }

    fn rule_hits(&self, change: &FunctionChange) -> Vec<&Reason> {
        let at = |reason: &&Reason| {
            reason.rule == ADDED_CALL
                && reason.path.as_deref() == Some(change.path.as_str())
                && reason.line == Some(change.line)
        };
        self.rating.reasons.iter().filter(at).collect()
    }

    fn impact_section(&self, links: &Links) -> Section {
        let (verdict, icon) = self.rating.code_impact(self.changes.len());
        let files = by_file(&self.changes);
        let widest = self.changes.iter().map(|change| change.reached_by).max();
        let detail = match widest {
            Some(widest) => format!(
                "{} in {}, reached by up to {widest}",
                plural(self.changes.len(), "function", "functions"),
                plural(files.len(), "file", "files")
            ),
            None => "no function changed".to_owned(),
        };

        let body = (!self.changes.is_empty()).then(|| self.impact_body(&files, links));
        Section {
            title: "Code impact",
            verdict: format!("{verdict} {icon}"),
            detail,
            body: body.unwrap_or_default(),
        }
    }

    fn impact_body(
        &self,
        files: &BTreeMap<&str, Vec<&FunctionChange>>,
        links: &Links,
    ) -> Vec<String> {
        let totals = format!(
            "{}; {} unchanged.",
            tally(self.changes.iter()),
            self.unchanged
        );
        let header = [
            "| File | Functions | Widest reach |".to_owned(),
            "|---|---|--:|".to_owned(),
        ];
        let rows = files.iter().map(|(path, changes)| {
            let widest = changes
                .iter()
                .map(|change| change.reached_by)
                .max()
                .unwrap_or(0);
            format!(
                "| {} | {} | {widest} |",
                links.file(path),
                tally(changes.iter().copied())
            )
        });

        let functions = self
            .changes
            .iter()
            .flat_map(|change| function_lines(change, links));
        let folded = format!(
            "<details><summary>All {}</summary>\n\n{}\n\n</details>",
            plural(self.changes.len(), "function", "functions"),
            functions.collect::<Vec<_>>().join("\n")
        );
        [totals, String::new()]
            .into_iter()
            .chain(header)
            .chain(rows)
            .chain([String::new(), folded])
            .collect()
    }

    fn calls_section(&self, links: &Links) -> Section {
        let hits: Vec<&Reason> = self
            .rating
            .reasons
            .iter()
            .filter(|reason| reason.rule == ADDED_CALL)
            .collect();
        let unclassified: usize = self
            .changes
            .iter()
            .map(|change| change.calls_unclassified.len())
            .sum();
        let unresolved = self
            .changes
            .iter()
            .filter(|change| change.unresolved_calls)
            .count();

        let (verdict, detail) = calls_verdict(&hits, unclassified, unresolved);
        let body = hits
            .iter()
            .map(|hit| format!("- {}", added_call(hit, links)))
            .chain(
                self.changes
                    .iter()
                    .filter_map(|change| calls_line(change, links)),
            );
        Section {
            title: "Calls",
            verdict,
            detail,
            body: body.collect(),
        }
    }

    fn signatures_section(&self, links: &Links) -> Section {
        let changed: Vec<&FunctionChange> = self
            .changes
            .iter()
            .filter(|change| change.signature_changed)
            .collect();
        let body = changed.iter().map(|change| {
            let callers = plural(change.callers, "caller", "callers");
            format!(
                "- {} · {callers}",
                links.function(&change.name, &change.path, Some(change.line))
            )
        });

        let (verdict, detail) = match changed.len() {
            0 => ("unchanged ✅", "no signature changed".to_owned()),
            count => ("changed 👀", plural(count, "function", "functions")),
        };
        Section {
            title: "Signatures",
            verdict: verdict.to_owned(),
            detail,
            body: body.collect(),
        }
    }

    fn effects_section(&self, links: &Links) -> Section {
        let kinds = |pick: fn(&FunctionChange) -> &[EffectChange]| -> BTreeSet<&str> {
            self.changes
                .iter()
                .flat_map(|change| pick(change).iter().map(|effect| effect.kind))
                .collect()
        };
        let (gained, lost) = (
            kinds(|change| &change.effects_gained),
            kinds(|change| &change.effects_lost),
        );
        let said = [("gains", gained), ("loses", lost)]
            .into_iter()
            .filter(|(_, kinds)| !kinds.is_empty())
            .map(|(verb, kinds)| {
                format!(
                    "{verb} {}",
                    kinds.into_iter().collect::<Vec<_>>().join(", ")
                )
            });

        let detail = said.collect::<Vec<_>>().join("; ");
        let body = self
            .changes
            .iter()
            .filter_map(|change| effects_line(change, links));
        let (verdict, detail) = match detail.is_empty() {
            true => ("unchanged ✅", "no effect gained or lost".to_owned()),
            false => ("changed 👀", detail),
        };
        Section {
            title: "Effects",
            verdict: verdict.to_owned(),
            detail,
            body: body.collect(),
        }
    }

    /// What the comparison can't see, so a person has to: text outside any function, functions
    /// whose facts don't say what changed, and generated code.
    fn blind_spots_section(&self, links: &Links) -> Section {
        let unseen: Vec<&FunctionChange> = self
            .changes
            .iter()
            .filter(|change| matches!(change.kind, ChangeKind::TextOnly | ChangeKind::Unknown))
            .collect();
        let counts = [
            (
                self.changed_outside_functions.len(),
                "file changed outside a function",
                "files changed outside a function",
            ),
            (
                unseen.len(),
                "function its facts can't speak for",
                "functions its facts can't speak for",
            ),
            (
                self.not_compared_changed,
                "generated function changed",
                "generated functions changed",
            ),
        ];
        let found: Vec<String> = counts
            .into_iter()
            .filter(|(count, _, _)| *count > 0)
            .map(|(count, one, many)| plural(count, one, many))
            .collect();

        let files = self
            .changed_outside_functions
            .iter()
            .map(|path| format!("- {}", links.file(path)));
        let functions = unseen.iter().map(|change| function_summary(change, links));
        let listed: Vec<String> = files.chain(functions).collect();
        let separator = (!listed.is_empty()).then(String::new);
        let body = listed.into_iter().chain(separator).chain([self.coverage()]);
        let (verdict, detail) = match found.is_empty() {
            true => ("none ✅", "nothing it can't see".to_owned()),
            false => ("found 👀", found.join(", ")),
        };
        Section {
            title: "Blind spots",
            verdict: verdict.to_owned(),
            detail,
            body: body.collect(),
        }
    }

    fn coverage(&self) -> String {
        format!(
            "{} unchanged. {} changed and aren't rated. {} aren't compared; {} of them changed.",
            plural(self.unchanged, "function", "functions"),
            plural(self.tests_changed, "test function", "test functions"),
            plural(
                self.not_compared,
                "generated function",
                "generated functions"
            ),
            self.not_compared_changed
        )
    }
}

/// One folded part of the report. Its title, verdict and what the verdict rests on make its
/// summary, so it reads while folded; one with nothing to unfold is a line.
struct Section {
    title: &'static str,
    verdict: String,
    detail: String,
    body: Vec<String>,
}

impl Section {
    fn render(self) -> String {
        let Self {
            title,
            verdict,
            detail,
            body,
        } = self;
        if body.is_empty() {
            return format!("**{title}:** {verdict} · {detail}");
        }

        let summary = format!("<strong>{title}:</strong> {verdict} · {detail}");
        format!(
            "<details><summary>{summary}</summary>\n\n{}\n\n</details>",
            body.join("\n")
        )
    }
}

/// The verdict's mark: a color for a measured level, eyes for what needs a person.
fn icon(level: Level) -> &'static str {
    match level {
        Level::Low => "🟢",
        Level::Medium => "🟡",
        Level::High => "🔴",
        Level::Unknown => "👀",
        Level::Unrated => "⚪",
    }
}

fn plural(count: usize, one: &str, many: &str) -> String {
    let noun = if count == 1 { one } else { many };
    format!("{count} {noun}")
}

/// How many functions of each kind, kinds with none left out.
fn tally<'a>(changes: impl Iterator<Item = &'a FunctionChange>) -> String {
    let changes: Vec<&FunctionChange> = changes.collect();
    let counts = KINDS.into_iter().map(|kind| {
        let count = changes.iter().filter(|change| change.kind == kind).count();
        (count, kind)
    });
    let said = counts
        .filter(|(count, _)| *count > 0)
        .map(|(count, kind)| format!("{count} {}", kind.name()));
    said.collect::<Vec<_>>().join(", ")
}

fn by_file(changes: &[FunctionChange]) -> BTreeMap<&str, Vec<&FunctionChange>> {
    let mut files: BTreeMap<&str, Vec<&FunctionChange>> = BTreeMap::new();
    for change in changes {
        files.entry(change.path.as_str()).or_default().push(change);
    }
    files
}

/// The strongest rule a new call hits, else whether any call can't be judged, else clean.
fn calls_verdict(hits: &[&Reason], unclassified: usize, unresolved: usize) -> (String, String) {
    let strongest = hits.iter().map(|hit| hit.level).reduce(Level::strongest);
    match strongest {
        Some(level) => (
            format!("{} {}", level.name(), icon(level)),
            format!("{} a rule names", plural(hits.len(), "call", "calls")),
        ),
        None if unclassified + unresolved > 0 => (
            "unknown 👀".to_owned(),
            format!(
                "{} nothing classifies, {} with a call to an unknown target",
                plural(unclassified, "new library call", "new library calls"),
                plural(unresolved, "function", "functions")
            ),
        ),
        None => (
            "clean ✅".to_owned(),
            "no rule fired; every new call is classified and resolved".to_owned(),
        ),
    }
}

/// A function and what's said of it, or nothing when nothing is.
fn noted(
    change: &FunctionChange,
    links: &Links,
    notes: Vec<String>,
    separator: &str,
) -> Option<String> {
    (!notes.is_empty()).then(|| format!("{}: {}", place(change, links), notes.join(separator)))
}

/// A function's line in "Needs a look", if anything about it needs one.
fn look_line(change: &FunctionChange, hits: &[&Reason], links: &Links) -> Option<String> {
    let line = noted(change, links, look_notes(change, hits), ", ")?;
    Some(format!("{line} · reached by {}", change.reached_by))
}

fn look_notes(change: &FunctionChange, hits: &[&Reason]) -> Vec<String> {
    let rules = hits.iter().map(|hit| {
        let callee = code(&short(hit.callee.as_deref().unwrap_or_default()));
        format!("calls {callee}, which a rule rates {}", hit.level.name())
    });
    let unknown = (change.kind == ChangeKind::Unknown)
        .then(|| escape(change.reason.unwrap_or("its tree isn't all of it")));
    // A new or removed function's facts are all of it; only a changed one's say what moved.
    let changed = (change.kind == ChangeKind::Changed).then(|| changed_notes(change));

    rules
        .chain(unknown)
        .chain(changed.into_iter().flatten())
        .collect()
}

fn changed_notes(change: &FunctionChange) -> Vec<String> {
    let unclassified = change.calls_unclassified.len();
    let outside: BTreeSet<&str> = change
        .effects_gained
        .iter()
        .map(|effect| effect.kind)
        .filter(|kind| *kind != "mutation")
        .collect();
    let facts = [
        change
            .signature_changed
            .then(|| "signature changed".to_owned()),
        change
            .values_changed
            .then(|| "a known value changed".to_owned()),
        change
            .unresolved_calls
            .then(|| "calls an unknown target".to_owned()),
        (unclassified > 0).then(|| {
            plural(
                unclassified,
                "new call nothing classifies",
                "new calls nothing classifies",
            )
        }),
    ];

    let gains = outside.into_iter().map(|kind| format!("gains {kind}"));
    facts.into_iter().flatten().chain(gains).collect()
}

/// A function's name and file, linked to its line in the head unless it's removed, since the head
/// doesn't hold it then.
fn place(change: &FunctionChange, links: &Links) -> String {
    match change.kind {
        ChangeKind::Removed => Links(None).function(&change.name, &change.path, None),
        _ => links.function(&change.name, &change.path, Some(change.line)),
    }
}

fn calls_line(change: &FunctionChange, links: &Links) -> Option<String> {
    let moved = !matches!(change.kind, ChangeKind::Added | ChangeKind::Removed);
    let parts = [
        moved
            .then(|| listed("calls more", &change.calls_added))
            .flatten(),
        moved
            .then(|| listed("calls less", &change.calls_removed))
            .flatten(),
        listed(
            "new library calls nothing classifies",
            &change.calls_unclassified,
        ),
        change
            .unresolved_calls
            .then(|| "calls an unknown target".to_owned()),
    ];
    let line = noted(change, links, parts.into_iter().flatten().collect(), "; ")?;
    Some(format!("- {line}"))
}

fn effects_line(change: &FunctionChange, links: &Links) -> Option<String> {
    let parts = effect_details("gains", &change.effects_gained)
        .into_iter()
        .chain(effect_details("loses", &change.effects_lost));
    let line = noted(change, links, parts.collect(), "; ")?;
    Some(format!("- {line}"))
}

fn added_call(reason: &Reason, links: &Links) -> String {
    let callee = code(&short(reason.callee.as_deref().unwrap_or_default()));
    let function = reason.function.as_deref().unwrap_or_default();
    let path = reason.path.as_deref().unwrap_or_default();
    let place = links.function(function, path, reason.line);

    format!(
        "{callee} newly called in {place}, which a rule rates {}",
        reason.level.name()
    )
}

fn function_lines(change: &FunctionChange, links: &Links) -> Vec<String> {
    let details = function_details(change).into_iter();
    std::iter::once(function_summary(change, links))
        .chain(details.map(|detail| format!("  - {detail}")))
        .collect()
}

fn function_summary(change: &FunctionChange, links: &Links) -> String {
    let name = place(change, links);
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

/// A code span. It only holds identifiers and paths, which hold no backtick.
fn code(text: &str) -> String {
    format!("`{text}`")
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
