//! Errors: how the errors functions can return changed, or why that can't be said.

use super::super::{Links, code, name, plural};
use super::{Check, Status};
use crate::cpg::failures::Source;
use crate::merge_risk::ChangeReport;
use crate::merge_risk::errors::ErrorChange;
use std::collections::{BTreeMap, BTreeSet};

const CANT_FOLLOW: &str = "a call it can't follow, such as a function value";

/// Only for a language whose errors are read.
pub(in super::super) fn errors(report: &ChangeReport, links: &Links) -> Option<Check> {
    let errors = &report.errors;
    if !errors.traced {
        return None;
    }
    let files: BTreeSet<&str> = report
        .changes
        .iter()
        .map(|change| change.path.as_str())
        .collect();
    let named = |change: &ErrorChange| function_link(change, &files, links);
    let seen: Vec<ErrorChange> = errors
        .changed
        .iter()
        .map(without_unknown)
        .filter(|change| !change.lost.is_empty() || !change.gained.is_empty())
        .collect();
    let blind: Vec<String> = errors
        .changed
        .iter()
        .filter(|change| change.gained.contains(&Source::Unknown))
        .map(|change| format!("{} can now also fail through {CANT_FOLLOW}", named(change)))
        .collect();

    Some(match (seen.is_empty(), blind.is_empty()) {
        (false, _) => changed(&seen, blind, &named),
        (true, false) => {
            let functions = plural(blind.len(), "function", "functions");
            let summary = format!("{functions} can now fail through a call it can't follow");
            Check::new("Errors", Status::Unknown, summary, blind)
        }
        (true, true) => unchanged(report, &named),
    })
}

/// Each function that fails differently, then those that newly fail in a way it can't follow.
fn changed(
    seen: &[ErrorChange],
    blind: Vec<String>,
    named: &dyn Fn(&ErrorChange) -> String,
) -> Check {
    let lines = error_lines(seen, named);
    let mut summary = format!(
        "{} differently",
        plural(lines.len(), "function fails", "functions fail")
    );
    if !blind.is_empty() {
        summary.push_str(&format!(", {} more it can't follow", blind.len()));
    }
    Check::new("Errors", Status::Look, summary, [lines, blind].concat())
}

/// Nothing changed that can be seen, and whether all of it could be.
fn unchanged(report: &ChangeReport, named: &dyn Fn(&ErrorChange) -> String) -> Check {
    let untraced = &report.errors.untraced;
    if !untraced.is_empty() {
        let functions = plural(untraced.len(), "function", "functions");
        let summary =
            format!("no change seen, but {functions} can fail through a call it can't follow");
        let items = untraced
            .iter()
            .map(|change| format!("{} can fail through {CANT_FOLLOW}", named(change)));
        return Check::new("Errors", Status::Unknown, summary, items.collect());
    }
    let item = match report.errors.followed {
        0 => "no changed function returns an error, and no other function's errors changed"
            .to_owned(),
        count => format!(
            "followed every error the {} can return back to where it's made; none was added or dropped",
            plural(count, "changed function", "changed functions")
        ),
    };
    Check::fine(
        "Errors",
        "no function fails in new ways or stops failing",
        item,
    )
}

fn without_unknown(change: &ErrorChange) -> ErrorChange {
    let known = |sources: &[Source]| {
        sources
            .iter()
            .filter(|source| **source != Source::Unknown)
            .cloned()
            .collect()
    };
    ErrorChange {
        lost: known(&change.lost),
        gained: known(&change.gained),
        ..change.clone()
    }
}

/// One line per function that fails differently. A function whose whole change is one it passes
/// on from another, wrapped, is said on that one's line.
fn error_lines(changes: &[ErrorChange], named: &dyn Fn(&ErrorChange) -> String) -> Vec<String> {
    let wrapped_by = wrappers(changes);
    let folded: BTreeSet<&str> = wrapped_by
        .values()
        .flatten()
        .map(|change| change.function.as_str())
        .collect();
    let lines = changes
        .iter()
        .filter(|change| !folded.contains(change.function.as_str()));
    lines
        .map(|change| {
            let wrappers = wrapped_by.get(change.function.as_str()).map(|callers| {
                let names: Vec<String> = callers.iter().map(|caller| named(caller)).collect();
                format!(", and so does {}, which wraps it", names.join(", "))
            });
            format!(
                "{} {}{}",
                named(change),
                what_changed(change),
                wrappers.unwrap_or_default()
            )
        })
        .collect()
}

fn what_changed(change: &ErrorChange) -> String {
    let said = [
        ("no longer fails with", &change.lost),
        ("can now fail with", &change.gained),
    ]
    .into_iter()
    .filter(|(_, sources)| !sources.is_empty())
    .map(|(verb, sources)| format!("{verb} {}", read_as(sources)));
    said.collect::<Vec<_>>().join("; ")
}

/// Each function another's change wraps: the caller's change is the callee's, wrapped once more.
fn wrappers(changes: &[ErrorChange]) -> BTreeMap<&str, Vec<&ErrorChange>> {
    let mut folded: BTreeMap<&str, Vec<&ErrorChange>> = BTreeMap::new();
    for caller in changes {
        let callee = changes.iter().find(|callee| {
            callee.function != caller.function
                && (&callee.lost, &callee.gained) == (&caller.lost, &caller.gained)
                && caller.wrapped > callee.wrapped
        });
        if let Some(callee) = callee {
            folded
                .entry(callee.function.as_str())
                .or_default()
                .push(caller);
        }
    }
    folded
}

/// Sources as a reader meets them: the message, the call or the variable.
fn read_as(sources: &[Source]) -> String {
    let said: BTreeSet<String> = sources.iter().map(source_text).collect();
    said.into_iter().collect::<Vec<_>>().join(" or ")
}

fn source_text(source: &Source) -> String {
    match source {
        Source::Message(message) if message.is_empty() => "an error with a computed message".into(),
        Source::Message(message) => format!("\"{}\"", readable_message(message)),
        Source::Call(callee) => format!("an error from {}", code(&name(callee))),
        Source::Variable(variable) => code(&name(variable)),
        Source::Unknown => "an error it can't trace".into(),
    }
}

/// A message's first line, with each format verb read as `…`. A literal `%%` stays a `%`.
fn readable_message(message: &str) -> String {
    let first = message.lines().next().unwrap_or_default();
    let parts: Vec<String> = first.split("%%").map(without_verbs).collect();
    let readable = parts.join("%");
    readable.trim_end_matches([':', ' ']).replace('`', "'")
}

/// Text with each `%` and the verb after it, its flags, width and precision included, read as `…`.
fn without_verbs(text: &str) -> String {
    let mut pieces = text.split('%');
    let lead = pieces.next().unwrap_or_default().to_owned();
    let rest = pieces.map(|piece| {
        let mut characters = piece
            .char_indices()
            .skip_while(|(_, at)| "-+# 0123456789.".contains(*at));
        let after = characters.nth(1).map_or(piece.len(), |(at, _)| at);
        format!("…{}", &piece[after..])
    });
    std::iter::once(lead).chain(rest).collect()
}

/// A function's name, linked to its line when its file is in the change.
fn function_link(change: &ErrorChange, files: &BTreeSet<&str>, links: &Links) -> String {
    let named = code(&name(&change.name));
    match files.contains(change.path.as_str()) && change.line > 0 {
        true => links.to(named, &change.path, Some(('R', change.line))),
        false => named,
    }
}

#[cfg(test)]
mod tests {
    use super::{Source, readable_message, source_text};

    #[test]
    fn each_source_reads_as_a_reviewer_meets_it() {
        let read = |source| source_text(&source);
        assert_eq!(
            read(Source::Message(String::new())),
            "an error with a computed message"
        );
        assert_eq!(read(Source::Message("no name".into())), "\"no name\"");
        assert_eq!(
            read(Source::Call("go::os.Open".into())),
            "an error from `os.Open`"
        );
        assert_eq!(read(Source::Variable("go::io.EOF".into())), "`io.EOF`");
        assert_eq!(read(Source::Unknown), "an error it can't trace");
    }

    #[test]
    fn a_message_reads_its_verbs_as_an_ellipsis_and_keeps_a_literal_percent() {
        assert_eq!(readable_message("load %s: %w"), "load …: …");
        assert_eq!(readable_message("%-8.2f%% done\nmore"), "…% done");
    }
}
