use super::super::rating::{Level, Rating, Reason};
use super::super::{ChangeKind, ChangeReport, EffectChange, FunctionChange, GraphDelta};
use super::short;

fn change(name: &str, kind: ChangeKind) -> FunctionChange {
    FunctionChange {
        function: name.into(),
        name: name.into(),
        path: "native/source.go".into(),
        line: 7,
        kind,
        reason: None,
        signature_changed: false,
        values_changed: false,
        graph: GraphDelta::default(),
        calls_added: Vec::new(),
        calls_removed: Vec::new(),
        unresolved_calls: false,
        calls_unclassified: Vec::new(),
        effects_gained: Vec::new(),
        effects_lost: Vec::new(),
        callers: 1,
        reached_by: 4,
    }
}

fn reason(rule: &'static str, level: Level, message: &str) -> Reason {
    Reason {
        rule,
        level,
        message: message.into(),
        path: Some("native/source.go".into()),
        function: Some("load".into()),
        line: Some(7),
        callee: None,
    }
}

fn report(risk: Level, reasons: Vec<Reason>, changes: Vec<FunctionChange>) -> ChangeReport {
    ChangeReport {
        schema_version: 1,
        rating: Rating {
            risk,
            policy_configured: true,
            reasons,
        },
        changes,
        unchanged: 1383,
        tests_changed: 3,
        not_compared: 2,
        not_compared_changed: 0,
        changed_outside_functions: Vec::new(),
    }
}

const URL: &str = "https://github.com/o/r/blob/abc";

fn at(change: FunctionChange, path: &str, line: usize) -> FunctionChange {
    FunctionChange {
        path: path.into(),
        line,
        ..change
    }
}

fn effect(kind: &'static str, label: &str) -> EffectChange {
    EffectChange {
        kind,
        label: label.into(),
    }
}

#[test]
fn it_leads_with_the_rating_and_says_a_shared_reason_once() {
    let over = "2 functions changed, more than the low-risk limit of 1";
    let reasons = vec![
        reason("pdg_change", Level::Medium, over),
        reason("pdg_change", Level::Medium, over),
    ];
    let changes = vec![
        change("load", ChangeKind::Changed),
        change("index", ChangeKind::Added),
    ];
    let markdown = report(Level::Medium, reasons, changes).markdown(None);

    let expected = format!(
        "### Merge risk: medium 🟡\n\n{over}.\n\n<details><summary><strong>Code impact:</strong> \
         changed 👀 · 2 functions in 1 file, reached by up to 4</summary>\n\n\
         1 changed, 1 added; 1383 unchanged.\n"
    );
    assert!(markdown.starts_with(&expected), "{markdown}");
}

#[test]
fn needs_a_look_says_what_moved_in_a_changed_function_and_where() {
    let mut moved = change("load", ChangeKind::Changed);
    moved.signature_changed = true;
    moved.values_changed = true;
    moved.unresolved_calls = true;
    moved.calls_unclassified = vec!["go::plugin.Open".into()];
    moved.effects_gained = vec![effect("io", "os.WriteFile"), effect("mutation", "x")];
    let mut added = change("fresh", ChangeKind::Added);
    added.signature_changed = true;
    let markdown = report(Level::Low, Vec::new(), vec![moved, added]).markdown(Some(URL));

    let look = "**Needs a look**\n\n\
        1. [`load`](https://github.com/o/r/blob/abc/native/source.go#L7) in `native/source.go`: \
        signature changed, a known value changed, calls an unknown target, \
        1 new call nothing classifies, gains io · reached by 4\n\n";
    assert!(
        markdown.contains(look),
        "a new function's facts are what it is: {markdown}"
    );
}

#[test]
fn a_section_with_nothing_to_unfold_is_one_line_with_its_verdict() {
    let markdown = report(Level::Low, Vec::new(), Vec::new()).markdown(None);
    for line in [
        "**Code impact:** unchanged ✅ · no function changed",
        "**Calls:** clean ✅ · no rule fired; every new call is classified and resolved",
        "**Signatures:** unchanged ✅ · no signature changed",
        "**Effects:** unchanged ✅ · no effect gained or lost",
    ] {
        assert!(
            markdown.contains(&format!("\n\n{line}\n\n")),
            "{line}: {markdown}"
        );
    }
    assert!(!markdown.contains("Needs a look"), "{markdown}");
    assert!(markdown.ends_with(
        "<details><summary><strong>Blind spots:</strong> none ✅ · nothing it can't see</summary>\n\n\
         1383 functions unchanged. 3 test functions changed and aren't rated. \
         2 generated functions aren't compared; 0 of them changed.\n\n</details>"
    ));
}

#[test]
fn a_rule_naming_a_call_rates_the_calls_section_and_says_where() {
    let mut writes = reason("added_call", Level::High, "added call: go::os.WriteFile");
    writes.callee = Some("go::os.WriteFile".into());
    let mut load = change("load", ChangeKind::Changed);
    load.calls_added = vec!["go::os.WriteFile".into()];
    let markdown = report(Level::High, vec![writes], vec![load]).markdown(Some(URL));

    let place =
        "[`load`](https://github.com/o/r/blob/abc/native/source.go#L7) in `native/source.go`";
    let why = format!("`go::os.WriteFile` newly called in {place}, which a rule rates high");
    assert!(
        markdown.starts_with(&format!("### Merge risk: high 🔴\n\n{why}.\n\n")),
        "{markdown}"
    );
    let look =
        format!("1. {place}: calls `go::os.WriteFile`, which a rule rates high · reached by 4");
    assert!(markdown.contains(&look), "{markdown}");
    let calls = format!(
        "<details><summary><strong>Calls:</strong> high 🔴 · 1 call a rule names</summary>\n\n\
         - {why}\n- {place}: calls more: `go::os.WriteFile`\n\n</details>"
    );
    assert!(markdown.contains(&calls), "{markdown}");
}

#[test]
fn the_functions_fold_under_code_impact_with_links_to_the_head_and_short_callees() {
    let mut changed = change("FromPackages", ChangeKind::Changed);
    changed.signature_changed = true;
    changed.values_changed = true;
    changed.unresolved_calls = true;
    changed.calls_added = vec!["go::slices.Clone".into(), "go::slices.DeleteFunc".into()];
    changed.calls_removed = vec!["go::github.com/o/r/internal/goeffects.packageErrors".into()];
    changed.effects_gained = vec![effect("io", "os.WriteFile")];
    let removed = change("typedFiles", ChangeKind::Removed);
    let markdown = report(Level::Low, Vec::new(), vec![changed, removed]).markdown(Some(URL));

    let functions = "<details><summary>All 2 functions</summary>\n\n\
        - [`FromPackages`](https://github.com/o/r/blob/abc/native/source.go#L7) in `native/source.go` changed · 1 caller, reached by 4\n  \
        - signature changed\n  \
        - a known value changed, as a constant's does\n  \
        - makes a call whose target can't be known\n  \
        - gains io: `os.WriteFile`\n  \
        - calls more: `go::slices.Clone`, `go::slices.DeleteFunc`\n  \
        - calls less: `go::goeffects.packageErrors`\n\
        - `typedFiles` in `native/source.go` removed · 1 caller, reached by 4\n\
        \n</details>";
    assert!(markdown.contains(functions), "{markdown}");
}

#[test]
fn files_are_tabled_by_path_whatever_order_the_changes_come_in() {
    let changes = vec![
        at(change("b", ChangeKind::Changed), "b.go", 1),
        at(change("a", ChangeKind::Added), "a.go", 1),
        at(change("a2", ChangeKind::Removed), "a.go", 9),
    ];
    let markdown = report(Level::Low, Vec::new(), changes).markdown(None);
    let table = "| File | Functions | Widest reach |\n|---|---|--:|\n\
        | `a.go` | 1 added, 1 removed | 4 |\n\
        | `b.go` | 1 changed | 4 |\n";
    assert!(markdown.contains(table), "{markdown}");
}

#[test]
fn effects_say_their_kinds_and_never_link_a_removed_function() {
    let mut gone = change("gone", ChangeKind::Removed);
    gone.effects_lost = vec![effect("mutation", "typed")];
    let mut fresh = change("fresh", ChangeKind::Added);
    fresh.effects_gained = vec![effect("mutation", "f.typed"), effect("io", "os.Open")];
    let markdown = report(Level::Low, Vec::new(), vec![gone, fresh]).markdown(Some(URL));

    let effects = "<details><summary><strong>Effects:</strong> changed 👀 · gains io, mutation; \
        loses mutation</summary>\n\n\
        - `gone` in `native/source.go`: loses mutation: `typed`\n\
        - [`fresh`](https://github.com/o/r/blob/abc/native/source.go#L7) in `native/source.go`: \
        gains io: `os.Open`; gains mutation: `f.typed`\n\n</details>";
    assert!(markdown.contains(effects), "{markdown}");
}

#[test]
fn blind_spots_list_what_it_cannot_see() {
    let mut outside = report(
        Level::Unknown,
        Vec::new(),
        vec![change("f", ChangeKind::TextOnly)],
    );
    outside.changed_outside_functions = vec!["params/config.go".into()];
    let markdown = outside.markdown(Some(URL));

    let spots = "<details><summary><strong>Blind spots:</strong> found 👀 · 1 file changed outside \
        a function, 1 function its facts can't speak for</summary>\n\n\
        - [`params/config.go`](https://github.com/o/r/blob/abc/params/config.go)\n\
        - [`f`](https://github.com/o/r/blob/abc/native/source.go#L7) in `native/source.go` text only \
        · 1 caller, reached by 4\n\n1383 functions unchanged.";
    assert!(markdown.contains(spots), "{markdown}");
}

#[test]
fn text_markdown_would_read_as_syntax_stays_text() {
    let mut odd = change("f", ChangeKind::Unknown);
    odd.reason = Some("holds <T> and *ptr");
    let markdown = report(Level::Unknown, Vec::new(), vec![odd]).markdown(None);
    assert!(
        markdown.contains("`f` in `native/source.go`: holds \\<T\\> and \\*ptr · reached by 4"),
        "{markdown}"
    );
}

#[test]
fn a_short_callee_keeps_its_package_and_namespace() {
    assert_eq!(
        short("go::github.com/o/r/native.(loadedFiles).reason"),
        "go::native.(loadedFiles).reason"
    );
    assert_eq!(short("go::(io/fs.FileInfo).Size"), "go::(fs.FileInfo).Size");
    assert_eq!(short("go::(*os.File).Truncate"), "go::(*os.File).Truncate");
    assert_eq!(
        short("core::clone::Clone::clone"),
        "core::clone::Clone::clone"
    );
}

#[test]
fn needs_a_look_names_ten_and_counts_the_rest() {
    let flagged = (0..12).map(|line| {
        let mut moved = at(
            change(&format!("f{line}"), ChangeKind::Changed),
            "a.go",
            line,
        );
        moved.signature_changed = true;
        moved
    });
    let markdown = report(Level::Low, Vec::new(), flagged.collect()).markdown(None);

    assert_eq!(markdown.matches("\n1. ").count(), 10, "{markdown}");
    assert!(markdown.contains("`f9` in `a.go`: signature changed · reached by 4\n\n…and 2 more in the sections below.\n\n"), "{markdown}");
}
