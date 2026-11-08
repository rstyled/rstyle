//! The report as Markdown, for a pull request's description. The rating comes first, with the
//! checks that hold it above low and why. Two folds follow: a glance at the change, and what the
//! checks found, with the functions behind each of their reasons. A check is one of the rules the
//! rating is the strongest reason of, so the headline is the worst check by construction.

use super::rating::{
    ADDED_CALL, ANALYSIS_UNAVAILABLE, Level, PDG_CHANGE, Reason, UNCLASSIFIED_CALL,
    UNRESOLVED_CALLS,
};
use super::{ChangeKind, ChangeReport, EffectChange, FunctionChange};
use std::collections::BTreeSet;

/// The rating's rules, in the order a reader should meet them, with the name each check goes by.
const CHECKS: [(&str, &str); 5] = [
    (PDG_CHANGE, "Changed code"),
    (ADDED_CALL, "Calls a rule names"),
    (UNCLASSIFIED_CALL, "Unclassified library calls"),
    (UNRESOLVED_CALLS, "Calls to unknown targets"),
    (ANALYSIS_UNAVAILABLE, "Changes outside functions"),
];

/// How many functions a reason lists before saying how many more there are.
const SHOWN: usize = 25;

impl ChangeReport {
    /// `source_url` is what a file's path is joined to for a link, such as
    /// `https://github.com/OWNER/REPO/blob/HEAD`. Without it, paths aren't links. A removed
    /// function is never one, since the head doesn't hold it.
    pub fn markdown(&self, source_url: Option<&str>) -> String {
        let links = Links(source_url.map(|url| url.trim_end_matches('/')));
        let checks: Vec<Check> = CHECKS
            .into_iter()
            .filter_map(|(rule, title)| Check::of(&self.rating.reasons, rule, title))
            .collect();

        let found = self.found(&checks, &links);
        [self.headline(), self.why(&checks), self.glance()]
            .into_iter()
            .chain(found)
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn headline(&self) -> String {
        format!("### Merge risk: {}", status(self.rating.risk))
    }

    /// Every check above low, with what it found. An unrated report says why nothing is rated.
    fn why(&self, checks: &[Check]) -> String {
        if !self.rating.policy_configured {
            return "**Not rated:** the base's `rstyle.toml` has no `[risk]` rules, so nothing \
                    here is rated. The checks below still show what each rule found."
                .to_owned();
        }
        let held: Vec<String> = checks
            .iter()
            .filter(|check| check.level.weight() > Level::Low.weight())
            .map(Check::held)
            .collect();

        match held.is_empty() {
            true => "**Every check is low or found nothing.**".to_owned(),
            false => format!("**Why it isn't low:**\n\n{}", held.join("\n")),
        }
    }

    /// One fold for everything the checks found. Its summary is the one check's, or each check's
    /// name and mark; inside, each check is a heading over its reasons.
    fn found(&self, checks: &[Check], links: &Links) -> Option<String> {
        let (summary, body) = match checks {
            [] => return None,
            [only] => (only.summary_line(), only.body(self, links)),
            many => {
                let marks = many.iter().map(Check::mark).collect::<Vec<_>>().join(" · ");
                let sections = many.iter().map(|check| check.section(self, links));
                let body = sections.collect::<Vec<_>>().join("\n\n");
                (format!("<strong>What it found:</strong> {marks}"), body)
            }
        };
        Some(format!(
            "<details><summary>{summary}</summary>\n\n{body}\n\n</details>"
        ))
    }

    fn glance(&self) -> String {
        let scope = match self.most_used() {
            Some(most) => format!("{}, used by up to {most}", self.scope()),
            None => "no function changed".to_owned(),
        };
        format!(
            "<details><summary><strong>At a glance:</strong> {scope}</summary>\n\n{}\n\n</details>",
            self.category_lines().join("\n")
        )
    }

    fn scope(&self) -> String {
        let files: BTreeSet<&str> = self
            .changes
            .iter()
            .map(|change| change.path.as_str())
            .collect();
        format!(
            "{} in {}",
            plural(self.changes.len(), "function", "functions"),
            plural(files.len(), "file", "files")
        )
    }

    fn most_used(&self) -> Option<usize> {
        self.changes.iter().map(|change| change.reached_by).max()
    }

    /// One line per kind of fact, each with what it found.
    fn category_lines(&self) -> [String; 5] {
        [
            self.impact_line(),
            self.signatures_line(),
            self.calls_line(),
            self.effects_line(),
            self.coverage_line(),
        ]
    }

    fn impact_line(&self) -> String {
        let Some(most) = self.most_used() else {
            return format!(
                "**Impact:** no function changed · {} unchanged",
                self.unchanged
            );
        };
        format!(
            "**Impact:** {} · {} · {} unchanged · used by up to {most}",
            self.scope(),
            tally(&self.changes),
            self.unchanged
        )
    }

    /// Whether some of the change is out of the compared facts' sight.
    fn has_gaps(&self) -> bool {
        let unseen = |change: &FunctionChange| {
            matches!(change.kind, ChangeKind::TextOnly | ChangeKind::Unknown)
        };
        self.changes.iter().any(unseen) || !self.changed_outside_functions.is_empty()
    }

    fn signatures_line(&self) -> String {
        let changed = self
            .changes
            .iter()
            .filter(|change| change.signature_changed)
            .count();
        match (changed, self.has_gaps()) {
            (0, true) => "**Signatures:** incomplete 👀 · 0 signature changes recorded; \
                          comparison has gaps"
                .to_owned(),
            (0, false) => "**Signatures:** unchanged ✅".to_owned(),
            (count, _) => format!("**Signatures:** {count} changed 👀"),
        }
    }

    /// What the calls show, with no level of its own: the checks below say how they rate.
    fn calls_line(&self) -> String {
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
        let found = [
            nonzero(unclassified, "new call", "new calls")
                .map(|calls| calls + " nothing classifies"),
            nonzero(unresolved, "function", "functions")
                .map(|count| count + " calling an unknown target"),
        ];

        let found: Vec<String> = found.into_iter().flatten().collect();
        let gaps = (found.is_empty() && self.has_gaps()).then(|| "comparison has gaps".to_owned());
        let gaps: Vec<String> = found.into_iter().chain(gaps).collect();
        calls_verdict(matched, &gaps)
    }

    fn effects_line(&self) -> String {
        let gained = effect_kinds(&self.changes, |change| &change.effects_gained);
        let lost = effect_kinds(&self.changes, |change| &change.effects_lost);
        let said: Vec<String> = [("gains", gained), ("loses", lost)]
            .into_iter()
            .filter(|(_, kinds)| !kinds.is_empty())
            .map(|(verb, kinds)| format!("{verb} {}", kinds.join(", ")))
            .collect();

        match said.is_empty() {
            true => "**Effects:** unchanged ✅".to_owned(),
            false => format!("**Effects:** {} 👀", said.join("; ")),
        }
    }

    fn coverage_line(&self) -> String {
        let count = |kind| {
            self.changes
                .iter()
                .filter(|change| change.kind == kind)
                .count()
        };
        let generated = nonzero(
            self.not_compared,
            "generated function",
            "generated functions",
        )
        .map(|count| {
            format!(
                "{count} not compared ({} changed)",
                self.not_compared_changed
            )
        });
        let gaps = [
            nonzero(self.changed_outside_functions.len(), "file", "files")
                .map(|count| count + " changed outside a function"),
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
            nonzero(
                self.tests_changed,
                "changed test function",
                "changed test functions",
            )
            .map(|count| count + " not rated"),
            generated,
        ];

        let gaps: Vec<String> = gaps.into_iter().flatten().collect();
        match gaps.is_empty() {
            true => "**Coverage:** no listed gaps ✅".to_owned(),
            false => format!("**Coverage:** limited 👀 · {}", gaps.join(" · ")),
        }
    }

    fn change_at(&self, reason: &Reason) -> Option<&FunctionChange> {
        self.changes.iter().find(|change| {
            reason.path.as_deref() == Some(change.path.as_str())
                && reason.line == Some(change.line)
                && reason.function.as_deref() == Some(change.name.as_str())
        })
    }
}

/// The calls line: a matched rule first, then anything the calls can't say.
fn calls_verdict(matched: usize, gaps: &[String]) -> String {
    let rules = match matched {
        0 => "no call rule matched".to_owned(),
        count => plural(count, "call rule match", "call rule matches"),
    };
    let said = std::iter::once(rules)
        .chain(gaps.iter().cloned())
        .collect::<Vec<_>>();

    match (matched, gaps.is_empty()) {
        (0, true) => "**Calls:** classified ✅ · no call rule matched; every new call is \
                      classified and resolved"
            .to_owned(),
        (0, false) => format!("**Calls:** incomplete 👀 · {}", said.join("; ")),
        _ => format!("**Calls:** matched 👀 · {}", said.join("; ")),
    }
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

/// One rule's reasons, as a check: its strongest level and its reasons grouped by what they say.
struct Check<'a> {
    rule: &'static str,
    title: &'static str,
    level: Level,
    groups: Vec<Group<'a>>,
}

/// The reasons of one check that say the same thing at the same level.
struct Group<'a> {
    level: Level,
    message: String,
    reasons: Vec<&'a Reason>,
}

impl<'a> Check<'a> {
    /// The check for `rule`, or none when it found nothing.
    fn of(reasons: &'a [Reason], rule: &'static str, title: &'static str) -> Option<Self> {
        let found: Vec<&Reason> = reasons
            .iter()
            .filter(|reason| reason.rule == rule)
            .collect();
        let level = found
            .iter()
            .map(|reason| reason.level)
            .reduce(Level::strongest)?;
        let mut groups: Vec<Group> = found.into_iter().fold(Vec::new(), group_into);

        groups.sort_by_key(|group| std::cmp::Reverse(group.level.weight()));
        Some(Self {
            rule,
            title,
            level,
            groups,
        })
    }

    /// What it found, as the first two of its groups.
    fn summary(&self) -> String {
        let said: Vec<String> = self
            .groups
            .iter()
            .take(2)
            .map(|group| group.counted(self.rule))
            .collect();
        let more = self.groups.len().saturating_sub(2);
        let rest = (more > 0).then(|| format!("; and {more} more"));
        said.join("; ") + &rest.unwrap_or_default()
    }

    /// Its line under "Why it isn't low".
    fn held(&self) -> String {
        let summary = escape(self.summary().trim_end_matches('.'));
        format!("- **{}** is {}: {summary}.", self.title, status(self.level))
    }

    /// Its name and level, as the fold that holds several checks names it.
    fn mark(&self) -> String {
        format!("{} {}", self.title, icon(self.level))
    }

    /// Its summary, when it's the only check in the fold.
    fn summary_line(&self) -> String {
        format!(
            "<strong>{}:</strong> {} · {}",
            self.title,
            status(self.level),
            html(&self.summary())
        )
    }

    /// Its heading and reasons, when the fold holds several checks.
    fn section(&self, report: &ChangeReport, links: &Links) -> String {
        let heading = format!("#### {}: {}", self.title, status(self.level));
        // Several kinds of reason each get a heading in the body, which says what a summary would.
        let summary = (self.groups.len() == 1).then(|| format!("\n\n{}", escape(&self.summary())));

        let body = self.body(report, links);
        format!("{heading}{}\n\n{body}", summary.unwrap_or_default())
    }

    /// Its reasons, each with the functions it was found in.
    fn body(&self, report: &ChangeReport, links: &Links) -> String {
        let listing = Listing {
            rule: self.rule,
            titled: self.groups.len() > 1,
            report,
            links,
        };
        let lines = self.groups.iter().flat_map(|group| group.lines(&listing));
        lines.collect::<Vec<_>>().join("\n").trim().to_owned()
    }
}

/// Adds a reason to the group that says what it says, by its message without the callee it names.
fn group_into<'a>(mut groups: Vec<Group<'a>>, reason: &'a Reason) -> Vec<Group<'a>> {
    let message = match reason.callee.as_deref() {
        Some(callee) => reason
            .message
            .replace(callee, "")
            .trim_end_matches([':', ' '])
            .to_owned(),
        None => reason.message.clone(),
    };
    let same = |group: &&mut Group| group.level == reason.level && group.message == message;

    match groups.iter_mut().find(same) {
        Some(group) => group.reasons.push(reason),
        None => groups.push(Group {
            level: reason.level,
            message,
            reasons: vec![reason],
        }),
    }
    groups
}

impl Group<'_> {
    /// Its message, with how many functions it was found in unless the message already says.
    fn counted(&self, rule: &str) -> String {
        let functions: BTreeSet<(&Option<String>, Option<usize>, &Option<String>)> = self
            .reasons
            .iter()
            .filter(|reason| reason.function.is_some())
            .map(|reason| (&reason.path, reason.line, &reason.function))
            .collect();
        let count = functions.len();
        if count <= 1 || self.message.starts_with(&format!("{count} ")) {
            return self.message.clone();
        }

        let in_functions = plural(count, "function", "functions");
        match rule {
            ADDED_CALL | UNCLASSIFIED_CALL => {
                let calls = plural(self.reasons.len(), "call", "calls");
                format!("{}: {calls} in {in_functions}", self.message)
            }
            _ => format!("{}: {in_functions}", self.message),
        }
    }

    /// Its reasons, one line each, under its own heading when the check holds more than one group.
    fn lines(&self, listing: &Listing) -> Vec<String> {
        let title = listing.titled.then(|| {
            let said = escape(&self.counted(listing.rule));
            format!("**{said}** — {}\n", status(self.level))
        });
        let listed = self.reasons.iter().take(SHOWN);
        let listed = listed.filter_map(|reason| listing.line(reason));

        let more = self.reasons.len().saturating_sub(SHOWN);
        let rest = (more > 0).then(|| format!("- …and {more} more"));
        title
            .into_iter()
            .chain(listed)
            .chain(rest)
            .chain([String::new()])
            .collect()
    }
}

/// What a check's reasons are listed with.
struct Listing<'a> {
    rule: &'a str,
    /// Whether each group of reasons gets a heading, as when a check holds more than one.
    titled: bool,
    report: &'a ChangeReport,
    links: &'a Links<'a>,
}

impl Listing<'_> {
    /// Where a reason was found, the callee it names, and for a changed function, what changed.
    fn line(&self, reason: &Reason) -> Option<String> {
        let change = self.report.change_at(reason);
        let place = match (change, &reason.path) {
            (Some(change), _) => place(change, self.links),
            (None, Some(path)) => self.links.file(path),
            (None, None) => return None,
        };
        let callee = reason
            .callee
            .as_deref()
            .map(|callee| format!(" calls {}", code(&short(callee))));

        let facts = change
            .filter(|_| self.rule == PDG_CHANGE)
            .map(|change| format!(" — {}", facts(change)));
        Some(format!(
            "- {place}{}{}",
            callee.unwrap_or_default(),
            facts.unwrap_or_default()
        ))
    }
}

/// What a changed function's line says of it, ending with what uses it.
fn facts(change: &FunctionChange) -> String {
    let flags = [
        change
            .signature_changed
            .then(|| "signature changed".to_owned()),
        change
            .values_changed
            .then(|| "a known value changed".to_owned()),
    ];
    let effects = effect_facts("gains", &change.effects_gained)
        .chain(effect_facts("loses", &change.effects_lost));
    // All of a new or removed function's calls are what it is, not what changed.
    let moved = !matches!(change.kind, ChangeKind::Added | ChangeKind::Removed);
    let calls = [
        moved
            .then(|| listed("calls more", &change.calls_added))
            .flatten(),
        moved
            .then(|| listed("calls less", &change.calls_removed))
            .flatten(),
    ];

    let used = format!(
        "used by {} ({} directly)",
        change.reached_by, change.callers
    );
    std::iter::once(change.kind.name().to_owned())
        .chain(flags.into_iter().flatten())
        .chain(effects)
        .chain(calls.into_iter().flatten())
        .chain([used])
        .collect::<Vec<_>>()
        .join(" · ")
}

fn effect_facts<'a>(
    verb: &'a str,
    effects: &'a [EffectChange],
) -> impl Iterator<Item = String> + 'a {
    effects
        .iter()
        .map(move |effect| format!("{verb} {} {}", effect.kind, code(&effect.label)))
}

fn listed(label: &str, callees: &[String]) -> Option<String> {
    if callees.is_empty() {
        return None;
    }
    let names: Vec<String> = callees.iter().map(|callee| code(&short(callee))).collect();
    Some(format!("{label} {}", names.join(", ")))
}

/// A function's name and file, linked to its line in the head unless it's removed, since the head
/// doesn't hold it then.
fn place(change: &FunctionChange, links: &Links) -> String {
    let name = code(&display(&change.name));
    let linked = match change.kind {
        ChangeKind::Removed => name,
        _ => links.to(name, &change.path, Some(change.line)),
    };
    format!("{linked} in {}", code(&change.path))
}

/// How many functions of each kind, kinds with none left out.
fn tally(changes: &[FunctionChange]) -> String {
    let kinds = [
        ChangeKind::Changed,
        ChangeKind::Added,
        ChangeKind::Removed,
        ChangeKind::TextOnly,
        ChangeKind::Unknown,
    ];
    let count = |kind| changes.iter().filter(|change| change.kind == kind).count();
    let said = kinds
        .into_iter()
        .filter_map(|kind| nonzero(count(kind), kind.name(), kind.name()));
    said.collect::<Vec<_>>().join(", ")
}

/// A level as every place that names one says it: its name and mark.
fn status(level: Level) -> String {
    format!("{} {}", level.name(), icon(level))
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

fn nonzero(count: usize, one: &str, many: &str) -> Option<String> {
    (count > 0).then(|| plural(count, one, many))
}

/// A function's name without the module path its file already gives: a Rust method keeps its type,
/// as `Links::to` for `markdown::Links::<'_>::to` and `ChangeReport::markdown` for
/// `markdown::<impl merge_risk::ChangeReport>::markdown`. A Go name is already short.
fn display(name: &str) -> String {
    if let Some((owner, item)) = impl_item(name) {
        return format!("{owner}::{item}");
    }
    let parts: Vec<&str> = name
        .split("::")
        .filter(|part| !part.starts_with('<'))
        .collect();
    match parts.as_slice() {
        [.., owner, item] if owner.starts_with(char::is_uppercase) => format!("{owner}::{item}"),
        _ => parts.last().copied().unwrap_or(name).to_owned(),
    }
}

/// The type and item of `…<impl path::Type>::item`.
fn impl_item(name: &str) -> Option<(&str, &str)> {
    let (_, rest) = name.rsplit_once("<impl ")?;
    let (owner, item) = rest.split_once(">::")?;
    Some((owner.rsplit("::").next().unwrap_or(owner), item))
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
}

/// A code span. It only holds identifiers and paths, which hold no backtick.
fn code(text: &str) -> String {
    format!("`{text}`")
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

/// Text for a `<summary>`, where Markdown isn't read but HTML is.
fn html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests;
