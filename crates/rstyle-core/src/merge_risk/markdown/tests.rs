use super::super::rating::{Level, Rating, Reason};
use super::super::{ChangeKind, ChangeReport, EffectChange, FunctionChange, GraphDelta};
use super::name;

const URL: &str = "https://github.com/o/r/pull/123";

fn change(name: &str, kind: ChangeKind, line: usize) -> FunctionChange {
    FunctionChange {
        function: name.into(),
        name: name.into(),
        path: "native/source.go".into(),
        line,
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

/// A reason found in `change`, as the rating gives one.
fn found(rule: &'static str, level: Level, message: &str, change: &FunctionChange) -> Reason {
    Reason {
        rule,
        level,
        message: message.into(),
        path: Some(change.path.clone()),
        function: Some(change.name.clone()),
        line: Some(change.line),
        callee: None,
    }
}

fn unclassified(change: &FunctionChange, callee: &str) -> Reason {
    let message = format!("new library call nothing classifies: {callee}");
    Reason {
        callee: Some(callee.into()),
        ..found("unclassified_call", Level::Unknown, &message, change)
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

fn effect(kind: &'static str, label: &str) -> EffectChange {
    EffectChange {
        kind,
        label: label.into(),
    }
}

#[test]
fn it_leads_with_why_and_folds_a_glance_and_a_table_per_file() {
    let mut moved = change("FromPackages", ChangeKind::Changed, 25);
    moved.signature_changed = true;
    moved.calls_added = vec!["go::slices.Clone".into()];
    moved.calls_removed = vec!["go::github.com/o/r/internal/goeffects.packageErrors".into()];
    moved.effects_gained = vec![effect("mutation", "f.typed")];
    let gone = change("typedFiles", ChangeKind::Removed, 75);
    let over = "2 functions changed, more than the low-risk limit of 1";
    let reasons = vec![
        found("pdg_change", Level::Medium, over, &moved),
        found("pdg_change", Level::Medium, over, &gone),
    ];
    let markdown = report(Level::Medium, reasons, vec![moved, gone]).markdown(Some(URL));

    let expected = format!(
        "### 🟡 Merge risk: medium\n\n- 🟡 **Changed code**: {over}.\n\n\
         <details><summary><strong>At a glance</strong></summary>\n\n\
         | Check | Result |\n|---|---|\n\
         | **Impact** | 2 functions in 1 file: 1 changed, 1 removed |\n\
         | **Signatures** | 👀 1 changed |\n\
         | **Calls** | ✅ nothing new to flag |\n\
         | **Effects** | 👀 gains mutation |\n\
         | **Coverage** | 👀 3 test functions not rated; 2 generated functions not compared |\n\n\
         </details>\n\n\
         <details><summary><strong>2 functions</strong> in 1 file</summary>\n\n\
         **[`native/source.go`]({URL}/files#diff-69a4aa84cf12711d033673793bd9079bfc0cfbb545bfc1c60bd7ea36c05d0065)**\n\n\
         | Function | What changed | Used by |\n|---|---|--:|\n\
         | [`FromPackages`]({URL}/files#diff-69a4aa84cf12711d033673793bd9079bfc0cfbb545bfc1c60bd7ea36c05d0065R25) | changed; signature changed; \
         gains mutation; now calls `slices.Clone`; no longer calls `goeffects.packageErrors` \
         | 4 (1 direct) |\n\
         | [`typedFiles`]({URL}/files#diff-69a4aa84cf12711d033673793bd9079bfc0cfbb545bfc1c60bd7ea36c05d0065L75) | removed | 4 (1 direct) |\n\n</details>"
    );
    assert_eq!(markdown, expected);
}

#[test]
fn nothing_found_says_so_and_no_function_table_follows() {
    let markdown = report(Level::Low, Vec::new(), Vec::new()).markdown(None);
    let expected = "### 🟢 Merge risk: low\n\nNothing holds it above low.\n\n\
        <details><summary><strong>At a glance</strong></summary>\n\n\
        | Check | Result |\n|---|---|\n\
        | **Impact** | no function changed |\n\
        | **Signatures** | ✅ unchanged |\n\
        | **Calls** | ✅ nothing new to flag |\n\
        | **Effects** | ✅ unchanged |\n\
        | **Coverage** | 👀 3 test functions not rated; 2 generated functions not compared |\n\n\
        </details>";
    assert_eq!(markdown, expected);
}

#[test]
fn an_unrated_report_says_why_nothing_is_rated() {
    let mut unrated = report(Level::Unrated, Vec::new(), Vec::new());
    unrated.rating.policy_configured = false;
    let expected = "### ⚪ Merge risk: unrated\n\n\
        Not rated: the base's `rstyle.toml` has no `[risk]` rules.\n\n";
    assert!(unrated.markdown(None).starts_with(expected));
}

#[test]
fn a_rule_naming_a_call_says_which_and_a_low_check_holds_nothing() {
    let mut load = change("load", ChangeKind::Changed, 7);
    load.calls_added = vec!["go::os.WriteFile".into()];
    let within = "the change is within the low-risk limit of 3 functions";
    let mut writes = found(
        "added_call",
        Level::High,
        "added call: go::os.WriteFile",
        &load,
    );
    writes.callee = Some("go::os.WriteFile".into());
    let reasons = vec![found("pdg_change", Level::Low, within, &load), writes];
    let markdown = report(Level::High, reasons, vec![load]).markdown(None);

    let why = "### 🔴 Merge risk: high\n\n\
        - 🔴 **Calls a rule names**: added call: `os.WriteFile`.\n\n";
    assert!(markdown.starts_with(why), "{markdown}");
    assert!(
        markdown.contains("| **Calls** | 👀 1 call a rule names |\n"),
        "{markdown}"
    );
    let row = "| `load` | changed; now calls `os.WriteFile` |";
    assert!(markdown.contains(row), "{markdown}");
}

#[test]
fn unclassified_calls_are_counted_by_callee_the_most_first() {
    let callees: Vec<String> = (0..12)
        .map(|index| format!("core::iter::Iterator::f{index:02}"))
        .collect();
    let mut busy = change("busy", ChangeKind::Changed, 1);
    busy.calls_unclassified = callees.clone();
    busy.unresolved_calls = true;
    let mut other = change("other", ChangeKind::Added, 2);
    other.calls_unclassified = vec![callees[11].clone()];
    let reasons = callees
        .iter()
        .map(|callee| unclassified(&busy, callee))
        .collect();
    let markdown = report(Level::Unknown, reasons, vec![busy, other]).markdown(None);

    let why = "- 👀 **Unclassified library calls**: new library call nothing classifies: \
        `Iterator::f00`, `Iterator::f01`, `Iterator::f02` and 9 more.\n\n";
    assert!(markdown.contains(why), "{markdown}");
    let row = "| **Calls** | 👀 13 new calls nothing classifies, 1 unknown target |\n";
    assert!(markdown.contains(row), "{markdown}");
    let table = "**Unclassified library calls**\n\n| Call | Times |\n|---|--:|\n\
        | `Iterator::f11` | 2 |\n| `Iterator::f00` | 1 |\n";
    assert!(markdown.contains(table), "{markdown}");
    let last = "| `Iterator::f08` | 1 |\n| …2 more | 2 |\n";
    assert!(markdown.contains(last), "{markdown}");
    let busy =
        "| `busy` | changed; calls an unknown target; 12 unclassified calls | 4 (1 direct) |";
    assert!(markdown.contains(busy), "{markdown}");
}

#[test]
fn what_it_cannot_see_is_in_the_glance_and_listed() {
    let changes = vec![
        change("quiet", ChangeKind::TextOnly, 1),
        change("odd", ChangeKind::Unknown, 2),
    ];
    let mut unseen = report(Level::Unknown, Vec::new(), changes);
    unseen.changed_outside_functions = vec!["params/config.go".into()];
    let markdown = unseen.markdown(Some(URL));

    for row in [
        "| **Signatures** | ✅ unchanged |\n",
        "| **Coverage** | 👀 1 file changed outside a function; 1 text-only function; \
         1 unknown function; 3 test functions not rated; 2 generated functions not compared |\n",
    ] {
        assert!(markdown.contains(row), "{row}: {markdown}");
    }
    let outside = format!(
        "**Changed outside any function:** [`params/config.go`]({URL}/files#diff-10ff00a15ef58da7485fedeca3906c73cb236fde4b143a9d61601c63cbf1ffe8)\n\n</details>"
    );
    assert!(markdown.ends_with(&outside), "{markdown}");
}

#[test]
fn text_that_would_break_the_layout_stays_text() {
    let odd = change("f|g", ChangeKind::Unknown, 1);
    let reasons = vec![found(
        "pdg_change",
        Level::Unknown,
        "holds <T> and *ptr",
        &odd,
    )];
    let markdown = report(Level::Unknown, reasons, vec![odd]).markdown(None);

    let why = "- 👀 **Changed code**: holds \\<T\\> and \\*ptr.\n";
    assert!(markdown.contains(why), "{markdown}");
    assert!(
        markdown.contains("| `f\\|g` | unknown | 4 (1 direct) |"),
        "{markdown}"
    );
}

#[test]
fn a_name_drops_the_path_around_it_and_keeps_its_owner() {
    assert_eq!(name("merge_risk::markdown::Links::<'_>::to"), "Links::to");
    assert_eq!(
        name("merge_risk::markdown::<impl merge_risk::ChangeReport>::markdown"),
        "ChangeReport::markdown"
    );
    assert_eq!(name("alloc::slice::{impl#0}::join"), "slice::join");
    assert_eq!(name("merge_risk::markdown::code"), "code");
    assert_eq!(
        name("go::github.com/o/r/native.(loadedFiles).reason"),
        "native.(loadedFiles).reason"
    );
    assert_eq!(name("go::(io/fs.FileInfo).Size"), "(fs.FileInfo).Size");
    assert_eq!(name("(loadedFiles).addFailed"), "(loadedFiles).addFailed");
}

#[test]
fn diff_links_hash_exact_paths_and_use_the_correct_side() {
    let cases = [
        (
            "src/a b.rs",
            "6fb7bacc941e44a163d2dbb1b6e5edfea4e6c008fc0b952d6c7d0b01f73974a3",
        ),
        (
            "src/é.rs",
            "a547d8165e279612d9975b2f6aba12e049347fb77973a381af9e613773d07f37",
        ),
    ];
    for (path, hash) in cases {
        for kind in [ChangeKind::Added, ChangeKind::Changed, ChangeKind::Removed] {
            let mut function = change("f", kind, 42);
            function.path = path.into();
            let markdown = report(Level::Low, Vec::new(), vec![function]).markdown(Some(URL));
            let side = if kind == ChangeKind::Removed {
                'L'
            } else {
                'R'
            };
            let target = format!("{URL}/files#diff-{hash}");
            assert!(
                markdown.contains(&format!("[`{path}`]({target})")),
                "{markdown}"
            );
            assert!(
                markdown.contains(&format!("[`f`]({target}{side}42)")),
                "{markdown}"
            );
            assert!(!markdown.contains("/blob/"), "{markdown}");
        }
    }
}

#[test]
fn diff_urls_with_a_files_suffix_or_trailing_slash_give_the_same_report() {
    let report = report(
        Level::Low,
        Vec::new(),
        vec![change("f", ChangeKind::Added, 1)],
    );
    let expected = report.markdown(Some(URL));
    for suffix in ["/", "/files", "/files/"] {
        assert_eq!(report.markdown(Some(&format!("{URL}{suffix}"))), expected);
    }
}

#[test]
fn an_unknown_line_links_to_the_file_diff() {
    let report = report(
        Level::Low,
        Vec::new(),
        vec![change("f", ChangeKind::Removed, 0)],
    );
    let markdown = report.markdown(Some(URL));
    assert!(markdown.contains(&format!("[`f`]({URL}/files#diff-69a4aa84cf12711d033673793bd9079bfc0cfbb545bfc1c60bd7ea36c05d0065)")), "{markdown}");
}
