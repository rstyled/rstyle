//! The report as Markdown, for a pull request's description. The rating comes first, with each
//! rule that holds it above low and why. Then each check a reviewer reads: its status, one line on
//! what it found, and a fold with every item behind that line, each linked to its line in the diff.

use super::rating::{
    ADDED_CALL, ANALYSIS_UNAVAILABLE, Level, PDG_CHANGE, Reason, UNCLASSIFIED_CALL,
    UNRESOLVED_CALLS,
};
use super::{ChangeKind, ChangeReport, EffectChange, FunctionChange};
use checks::Check;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

mod checks;

/// The rating's rules, in the order a reader should meet them, with the name each check goes by.
const CHECKS: [(&str, &str); 5] = [
    (PDG_CHANGE, "Changed code"),
    (ADDED_CALL, "Calls a rule names"),
    (UNCLASSIFIED_CALL, "Unclassified library calls"),
    (UNRESOLVED_CALLS, "Calls to unknown targets"),
    (ANALYSIS_UNAVAILABLE, "Changes outside functions"),
];

/// How many callees a rule's line names.
const CALLEES_SHOWN: usize = 3;

const KINDS: [ChangeKind; 5] = [
    ChangeKind::Changed,
    ChangeKind::Added,
    ChangeKind::Removed,
    ChangeKind::TextOnly,
    ChangeKind::Unknown,
];

impl ChangeReport {
    /// `pull_request_url` names the GitHub PR whose diff the report links to, such as
    /// `https://github.com/OWNER/REPO/pull/123`. Removed functions link to the base side;
    /// other functions link to the head side. Without a URL, paths and names stay plain text.
    pub fn markdown(&self, pull_request_url: Option<&str>) -> String {
        let links =
            Links(pull_request_url.map(|url| url.trim_end_matches('/').trim_end_matches("/files")));
        let risk = self.rating.risk;
        let headline = format!("### {} Merge risk: {}", icon(risk), risk.name());

        [headline, self.why(), self.checks(&links)].join("\n\n")
    }

    /// Each check, as a status line that opens to what it found.
    fn checks(&self, links: &Links) -> String {
        let checks = [
            Some(checks::changed(self, links)),
            checks::errors(self, links),
            Some(checks::signatures(self, links)),
            Some(checks::effects(self, links)),
            Some(checks::calls(self, links)),
        ];
        let folds = checks.iter().flatten().map(fold);
        folds.collect::<Vec<_>>().join("\n")
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
}

/// A check as a status line that opens to every item behind it.
fn fold(check: &Check) -> String {
    let items: Vec<String> = check.items.iter().map(|item| format!("- {item}")).collect();
    format!(
        "<details><summary>{} <b>{}</b>: {}</summary>\n\n{}\n\n</details>",
        check.status.icon(),
        check.name,
        inline_html(&check.summary),
        items.join("\n")
    )
}

/// Markdown text as HTML, for a fold's summary line, where GitHub reads no Markdown: code spans
/// become `<code>` and the rest is escaped.
fn inline_html(text: &str) -> String {
    let mut html = String::with_capacity(text.len());
    for (at, part) in text.split('`').enumerate() {
        let escaped = part
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        match at % 2 {
            1 => html.push_str(&format!("<code>{escaped}</code>")),
            _ => html.push_str(&escaped),
        }
    }
    html
}

/// A rule's line under the headline when it's above low: its strongest level and what it found
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

/// A function's name, linked to its line on the appropriate side of the PR diff.
fn place(change: &FunctionChange, links: &Links) -> String {
    let named = code(&name(&change.name));
    let side = match change.kind {
        ChangeKind::Removed => 'L',
        _ => 'R',
    };
    let line = (change.line > 0).then_some((side, change.line));
    links.to(named, &change.path, line)
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
/// `native.load` for `go::github.com/o/r/native.load`. A C++ identity is read by [`cxx_name`].
fn name(identity: &str) -> String {
    if let Some(path) = cxx_path(identity) {
        return cxx_name(&path);
    }
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

/// A C++ callee's scopes and name, without what its signature adds: its template arguments,
/// parameters, qualifiers and return type, any of which can hold a `::` of its own.
/// `cxx::std::get_if<typename>(variant<…> *) -> typename add_pointer<…>::type` is `std::get_if`.
fn cxx_path(identity: &str) -> Option<String> {
    let signature = identity.strip_prefix("cxx::")?;
    let mut angles = 0_usize;
    let mut path = String::with_capacity(signature.len());
    for character in signature.chars() {
        match character {
            '<' => angles += 1,
            '>' => angles = angles.saturating_sub(1),
            // A scope such as `(anonymous namespace)` starts with one, and parameters follow a name.
            '(' if angles == 0 && !path.is_empty() && !path.ends_with("::") => break,
            _ if angles == 0 => path.push(character),
            _ => {}
        }
    }
    Some(path)
}

/// A C++ name with the scope it's in, since a type in C++ is as often lowercase as not and a
/// namespace tells `fs::remove` from `filesystem::remove`. A file or an anonymous namespace says
/// nothing a reader can use, so a name in one stands alone.
fn cxx_name(path: &str) -> String {
    let mut parts = path.rsplit("::");
    let item = parts.next().unwrap_or(path);
    match parts.next() {
        Some(scope) if !scope.starts_with('(') && !scope.contains(['/', '.']) => {
            format!("{scope}::{item}")
        }
        _ => item.to_owned(),
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
    fn to(&self, text: String, path: &str, line: Option<(char, usize)>) -> String {
        let Some(root) = self.0 else {
            return text;
        };
        // GitHub diff anchors hash the exact repository-relative UTF-8 path, not a URL-encoded path.
        let digest = Sha256::digest(path.as_bytes());
        let anchor = line
            .map(|(side, line)| format!("{side}{line}"))
            .unwrap_or_default();
        format!("[{text}]({root}/files#diff-{digest:x}{anchor})")
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

#[cfg(test)]
mod tests;
