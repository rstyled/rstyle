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
        "**Merge risk: medium ⚠️**\n- {over}\n\n\
         **Code impact:** changed 👀 · 1 changed, 1 added, 1383 unchanged\n\n"
    );
    assert!(markdown.starts_with(&expected), "{markdown}");
    assert!(markdown.ends_with(
        "Test functions changed: 3 · generated functions not compared: 2, 0 of them changed"
    ));
}

#[test]
fn the_functions_fold_away_with_links_to_the_head_and_short_callees() {
    let mut changed = change("FromPackages", ChangeKind::Changed);
    changed.signature_changed = true;
    changed.values_changed = true;
    changed.unresolved_calls = true;
    changed.calls_added = vec!["go::slices.Clone".into(), "go::slices.DeleteFunc".into()];
    changed.calls_removed = vec!["go::github.com/o/r/internal/goeffects.packageErrors".into()];
    changed.effects_gained = vec![EffectChange {
        kind: "io",
        label: "os.WriteFile".into(),
    }];
    let removed = change("typedFiles", ChangeKind::Removed);
    let markdown = report(Level::Low, Vec::new(), vec![changed, removed]).markdown(Some(URL));

    let functions = "<details><summary>2 affected functions</summary>\n\n\
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
fn a_rule_naming_a_call_says_where_it_is_made() {
    let mut writes = reason("added_call", Level::High, "added call: go::os.WriteFile");
    writes.callee = Some("go::os.WriteFile".into());
    let markdown = report(Level::High, vec![writes], Vec::new()).markdown(Some(URL));
    let line = "- added call `go::os.WriteFile` in \
        [`load`](https://github.com/o/r/blob/abc/native/source.go#L7) in `native/source.go`";
    assert!(
        markdown.starts_with(&format!("**Merge risk: high ⚠️**\n{line}\n")),
        "{markdown}"
    );
}

#[test]
fn a_file_changed_outside_any_function_is_in_plain_sight() {
    let mut outside = report(Level::Unknown, Vec::new(), Vec::new());
    outside.changed_outside_functions = vec!["params/config.go".into()];
    let markdown = outside.markdown(Some(URL));
    let listed = "**Changed outside any function**, which no fact here says the effect of:\n\
        - [`params/config.go`](https://github.com/o/r/blob/abc/params/config.go)";
    assert!(markdown.contains(listed), "{markdown}");
    assert!(
        !markdown.contains("<details>"),
        "no function changed: {markdown}"
    );
}

#[test]
fn text_markdown_would_read_as_syntax_stays_text() {
    let mut odd = change("f", ChangeKind::Unknown);
    odd.reason = Some("holds <T> and *ptr");
    let markdown = report(Level::Unknown, Vec::new(), vec![odd]).markdown(None);
    assert!(
        markdown.contains("  - holds \\<T\\> and \\*ptr"),
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
