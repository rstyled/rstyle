//! The report as Markdown, for a pull request's description. The rating comes first, with each
//! check that holds it above low and why. Two folds follow: a table of what changed at a glance,
//! and every changed function in a table per file. A check is one of the rules the rating is the
//! strongest reason of, so the headline is the worst check by construction.

use super::rating::{
    ADDED_CALL, ANALYSIS_UNAVAILABLE, Level, PDG_CHANGE, Reason, UNCLASSIFIED_CALL,
    UNRESOLVED_CALLS,
};
use super::{ChangeKind, ChangeReport, EffectChange, FunctionChange};
use std::collections::{BTreeMap, BTreeSet};

/// The rating's rules, in the order a reader should meet them, with the name each check goes by.
const CHECKS: [(&str, &str); 5] = [
    (PDG_CHANGE, "Changed code"),
    (ADDED_CALL, "Calls a rule names"),
    (UNCLASSIFIED_CALL, "Unclassified library calls"),
    (UNRESOLVED_CALLS, "Calls to unknown targets"),
    (ANALYSIS_UNAVAILABLE, "Changes outside functions"),
];

/// How many callees a check's line names, and how many unclassified calls the table lists.
const CALLEES_SHOWN: usize = 3;
const CALLS_SHOWN: usize = 10;

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
        let risk = self.rating.risk;
        let headline = format!("### {} Merge risk: {}", icon(risk), risk.name());

        [
            Some(headline),
            Some(self.why()),
            Some(self.glance()),
            self.functions(&links),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n\n")
    }

    /// Each check above low, with what it found. An unrated report says why nothing is rated.
    fn why(&self) -> String {
        if !self.rating.policy_configured {
            return "Not rated: the base's `rstyle.toml` has no `[risk]` rules.".to_owned();
        }
        let held: Vec<String> = CHECKS
            .into_iter()
            .filter_map(|(rule, title)| held(&self.rating.reasons, rule, title))
            .collect();

        match held.is_empty() {
            true => "Nothing holds it above low.".to_owned(),
            false => held.join("\n"),
        }
    }

    fn glance(&self) -> String {
        let rows = [
            ("Impact", self.impact()),
            ("Signatures", self.signatures()),
            ("Calls", self.calls()),
            ("Effects", self.effects()),
            ("Coverage", self.coverage()),
        ];
        let rows = rows.map(|(check, result)| format!("| **{check}** | {} |", cell(&result)));

        format!(
            "<details><summary><strong>At a glance</strong></summary>\n\n\
             | Check | Result |\n|---|---|\n{}\n\n</details>",
            rows.join("\n")
        )
    }

    fn files(&self) -> BTreeSet<&str> {
        self.changes
            .iter()
            .map(|change| change.path.as_str())
            .collect()
    }

    fn impact(&self) -> String {
        if self.changes.is_empty() {
            return "no function changed".to_owned();
        }
        let files = plural(self.files().len(), "file", "files");
        let functions = plural(self.changes.len(), "function", "functions");
        format!("{functions} in {files}: {}", tally(&self.changes))
    }

    /// Every changed function's signature is compared, a text-only or unknown one's too, so none
    /// changing is unchanged. What the comparison can't see is the coverage row's to say.
    fn signatures(&self) -> String {
        let changed = self
            .changes
            .iter()
            .filter(|change| change.signature_changed)
            .count();
        match changed {
            0 => "✅ unchanged".to_owned(),
            count => format!("👀 {count} changed"),
        }
    }

    /// What the calls show, with no level of its own: the checks above say how they rate.
    fn calls(&self) -> String {
        let matched = self
            .rating
            .reasons
            .iter()
            .filter(|reason| reason.rule == ADDED_CALL)
            .count();
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
        if matched > 0 {
            return format!("👀 {} a rule names", plural(matched, "call", "calls"));
        }

        let unknown = nonzero(unresolved, "unknown target", "unknown targets")
            .map(|count| format!(", {count}"));
        match unclassified + unresolved {
            0 => "✅ nothing new to flag".to_owned(),
            _ => format!(
                "👀 {} nothing classifies{}",
                plural(unclassified, "new call", "new calls"),
                unknown.unwrap_or_default()
            ),
        }
    }

    fn effects(&self) -> String {
        let gained = effect_kinds(&self.changes, |change| &change.effects_gained);
        let lost = effect_kinds(&self.changes, |change| &change.effects_lost);
        let said: Vec<String> = [("gains", gained), ("loses", lost)]
            .into_iter()
            .filter(|(_, kinds)| !kinds.is_empty())
            .map(|(verb, kinds)| format!("{verb} {}", kinds.join(", ")))
            .collect();

        match said.is_empty() {
            true => "✅ unchanged".to_owned(),
            false => format!("👀 {}", said.join("; ")),
        }
    }

    fn coverage(&self) -> String {
        let count = |kind| {
            self.changes
                .iter()
                .filter(|change| change.kind == kind)
                .count()
        };
        let outside = nonzero(self.changed_outside_functions.len(), "file", "files")
            .map(|count| count + " changed outside a function");
        let gaps = [
            outside,
            nonzero(
                count(ChangeKind::TextOnly),
                "text-only function",
                "text-only functions",
            ),
            nonzero(
                count(ChangeKind::Unknown),
                "unknown function",
                "unknown functions",
            ),
            nonzero(self.tests_changed, "test function", "test functions")
                .map(|count| count + " not rated"),
            nonzero(
                self.not_compared,
                "generated function",
                "generated functions",
            )
            .map(|count| count + " not compared"),
        ];

        let gaps: Vec<String> = gaps.into_iter().flatten().collect();
        match gaps.is_empty() {
            true => "✅ no gaps".to_owned(),
            false => format!("👀 {}", gaps.join("; ")),
        }
    }

    /// Every changed function, in a table per file, then what no single function says.
    fn functions(&self, links: &Links) -> Option<String> {
        if self.changes.is_empty() {
            return None;
        }
        let tables = self
            .files()
            .into_iter()
            .map(|path| self.file_table(path, links));
        let rest = [self.unclassified_table(), self.outside(links)];

        let body: Vec<String> = tables.chain(rest.into_iter().flatten()).collect();
        let summary = format!(
            "<strong>{}</strong> in {}",
            plural(self.changes.len(), "function", "functions"),
            plural(self.files().len(), "file", "files")
        );
        Some(format!(
            "<details><summary>{summary}</summary>\n\n{}\n\n</details>",
            body.join("\n\n")
        ))
    }

    fn file_table(&self, path: &str, links: &Links) -> String {
        let mut changes: Vec<&FunctionChange> = self
            .changes
            .iter()
            .filter(|change| change.path == path)
            .collect();
        changes.sort_by_key(|change| change.line);
        let rows = changes.into_iter().map(|change| {
            let used = format!("{} ({} direct)", change.reached_by, change.callers);
            format!(
                "| {} | {} | {used} |",
                place(change, links),
                cell(&what_changed(change))
            )
        });

        let header = "| Function | What changed | Used by |\n|---|---|--:|";
        format!(
            "**{}**\n\n{header}\n{}",
            links.file(path),
            rows.collect::<Vec<_>>().join("\n")
        )
    }

    /// The library calls nothing classifies, by how often the change makes them.
    fn unclassified_table(&self) -> Option<String> {
        let calls = self
            .changes
            .iter()
            .flat_map(|change| change.calls_unclassified.iter());
        let counted = calls.fold(BTreeMap::<String, usize>::new(), |mut counts, callee| {
            *counts.entry(name(callee)).or_default() += 1;
            counts
        });
        let mut ranked: Vec<(String, usize)> = counted.into_iter().collect();
        ranked.sort_by(|(a, a_count), (b, b_count)| b_count.cmp(a_count).then(a.cmp(b)));
        if ranked.is_empty() {
            return None;
        }

        let rest = ranked.split_off(ranked.len().min(CALLS_SHOWN));
        let rows = ranked
            .into_iter()
            .map(|(callee, count)| format!("| {} | {count} |", cell(&code(&callee))));
        let more = (!rest.is_empty()).then(|| {
            let times: usize = rest.iter().map(|(_, count)| count).sum();
            format!("| …{} more | {times} |", rest.len())
        });
        let rows: Vec<String> = rows.chain(more).collect();
        Some(format!(
            "**Unclassified library calls**\n\n| Call | Times |\n|---|--:|\n{}",
            rows.join("\n")
        ))
    }

    fn outside(&self, links: &Links) -> Option<String> {
        if self.changed_outside_functions.is_empty() {
            return None;
        }
        let files: Vec<String> = self
            .changed_outside_functions
            .iter()
            .map(|path| links.file(path))
            .collect();
        Some(format!(
            "**Changed outside any function:** {}",
            files.join(", ")
        ))
    }
}

/// A check's line under the headline when it's above low: its strongest level and what it found
/// there, with the callees it names.
fn held(reasons: &[Reason], rule: &str, title: &str) -> Option<String> {
    let found: Vec<&Reason> = reasons
        .iter()
        .filter(|reason| reason.rule == rule)
        .collect();
    let level = found
        .iter()
        .map(|reason| reason.level)
        .reduce(Level::strongest)?;
    if level.weight() <= Level::Low.weight() {
        return None;
    }

    let at_level: Vec<&Reason> = found
        .into_iter()
        .filter(|reason| reason.level == level)
        .collect();
    let messages: BTreeSet<String> = at_level
        .iter()
        .map(|reason| without_callee(reason))
        .collect();
    let said = messages
        .into_iter()
        .map(|message| escape(&message))
        .collect::<Vec<_>>()
        .join("; ");
    let callees = named_callees(&at_level);
    Some(format!("- {} **{title}**: {said}{callees}.", icon(level)))
}

/// A reason's message without the callee it names, so reasons that differ only by callee read as
/// one.
fn without_callee(reason: &Reason) -> String {
    match reason.callee.as_deref() {
        Some(callee) => reason
            .message
            .replace(callee, "")
            .trim_end_matches([':', ' '])
            .to_owned(),
        None => reason.message.trim_end_matches('.').to_owned(),
    }
}

/// The callees a check's reasons name, the first few of them.
fn named_callees(reasons: &[&Reason]) -> String {
    let callees: BTreeSet<String> = reasons
        .iter()
        .filter_map(|reason| reason.callee.as_deref())
        .map(name)
        .collect();
    if callees.is_empty() {
        return String::new();
    }

    let shown: Vec<String> = callees
        .iter()
        .take(CALLEES_SHOWN)
        .map(|callee| code(callee))
        .collect();
    let more = callees.len().saturating_sub(CALLEES_SHOWN);
    let rest = (more > 0).then(|| format!(" and {more} more"));
    format!(": {}{}", shown.join(", "), rest.unwrap_or_default())
}

/// What changed in a function, in the order a reviewer weighs it.
fn what_changed(change: &FunctionChange) -> String {
    let flags = [
        change
            .signature_changed
            .then(|| "signature changed".to_owned()),
        change
            .values_changed
            .then(|| "a known value changed".to_owned()),
        change
            .unresolved_calls
            .then(|| "calls an unknown target".to_owned()),
        nonzero(
            change.calls_unclassified.len(),
            "unclassified call",
            "unclassified calls",
        ),
        effect_change("gains", &change.effects_gained),
        effect_change("loses", &change.effects_lost),
    ];
    // All of a new or removed function's calls are what it is, not what changed.
    let moved = !matches!(change.kind, ChangeKind::Added | ChangeKind::Removed);
    let new: Vec<&String> = change
        .calls_added
        .iter()
        .filter(|callee| !change.calls_unclassified.contains(callee))
        .collect();
    let calls = [
        moved.then(|| callees("now calls", new)).flatten(),
        moved
            .then(|| callees("no longer calls", change.calls_removed.iter().collect()))
            .flatten(),
    ];

    std::iter::once(change.kind.name().to_owned())
        .chain(flags.into_iter().flatten())
        .chain(calls.into_iter().flatten())
        .collect::<Vec<_>>()
        .join("; ")
}

fn effect_change(verb: &str, effects: &[EffectChange]) -> Option<String> {
    let kinds: BTreeSet<&str> = effects.iter().map(|effect| effect.kind).collect();
    let kinds: Vec<&str> = kinds.into_iter().collect();
    (!kinds.is_empty()).then(|| format!("{verb} {}", kinds.join(", ")))
}

fn callees(label: &str, callees: Vec<&String>) -> Option<String> {
    if callees.is_empty() {
        return None;
    }
    let names: Vec<String> = callees
        .into_iter()
        .map(|callee| code(&name(callee)))
        .collect();
    Some(format!("{label} {}", names.join(", ")))
}

/// The kinds of effect the changes gained, or lost, in order.
fn effect_kinds(
    changes: &[FunctionChange],
    pick: fn(&FunctionChange) -> &[EffectChange],
) -> Vec<&'static str> {
    let kinds: BTreeSet<&'static str> = changes
        .iter()
        .flat_map(|change| pick(change).iter().map(|effect| effect.kind))
        .collect();
    kinds.into_iter().collect()
}

/// A function's name, linked to its line in the head unless it's removed, since the head doesn't
/// hold it then.
fn place(change: &FunctionChange, links: &Links) -> String {
    let named = cell(&code(&name(&change.name)));
    match change.kind {
        ChangeKind::Removed => named,
        _ => links.to(named, &change.path, Some(change.line)),
    }
}

/// How many functions of each kind, kinds with none left out.
fn tally(changes: &[FunctionChange]) -> String {
    let count = |kind| changes.iter().filter(|change| change.kind == kind).count();
    let said = KINDS
        .into_iter()
        .filter_map(|kind| nonzero(count(kind), kind.name(), kind.name()));
    said.collect::<Vec<_>>().join(", ")
}

/// The headline's mark: a color for a measured level, eyes for what needs a person.
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

fn nonzero(count: usize, one: &str, many: &str) -> Option<String> {
    (count > 0).then(|| plural(count, one, many))
}

/// A function or callee without the module path or language prefix around it: a Rust method keeps
/// its type, as `Links::to` for `markdown::Links::<'_>::to`, `ChangeReport::markdown` for
/// `markdown::<impl merge_risk::ChangeReport>::markdown` and `slice::join` for
/// `alloc::slice::{impl#0}::join`. A Go identity drops its import path's directories, as
/// `native.load` for `go::github.com/o/r/native.load`.
fn name(identity: &str) -> String {
    if let Some((owner, item)) = impl_item(identity) {
        return format!("{owner}::{item}");
    }
    let raw: Vec<&str> = identity.split("::").collect();
    if let [.., owner, block, item] = raw.as_slice()
        && block.starts_with("{impl")
    {
        return format!("{owner}::{item}");
    }

    let parts: Vec<&str> = raw
        .into_iter()
        .filter(|part| !part.starts_with(['<', '{']))
        .collect();
    match parts.as_slice() {
        [.., owner, item] if owner.starts_with(char::is_uppercase) => format!("{owner}::{item}"),
        _ => short(parts.last().copied().unwrap_or(identity)),
    }
}

/// The type and item of `…<impl path::Type>::item`.
fn impl_item(name: &str) -> Option<(&str, &str)> {
    let (_, rest) = name.rsplit_once("<impl ")?;
    let (owner, item) = rest.split_once(">::")?;
    Some((owner.rsplit("::").next().unwrap_or(owner), item))
}

/// A name without the directories of its import path: `github.com/o/r/native.f` reads as
/// `native.f`, and `(io/fs.FileInfo).Size` as `(fs.FileInfo).Size`.
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
}

/// A code span. It only holds identifiers and paths, which hold no backtick.
fn code(text: &str) -> String {
    format!("`{text}`")
}

/// Text for a table cell, where a bare `|` would end the cell.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

/// Text with what Markdown would read as syntax escaped.
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
