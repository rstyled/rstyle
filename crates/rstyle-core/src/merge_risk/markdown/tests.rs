use super::super::rating::{Level, Rating, Reason};
use super::super::{ChangeKind, ChangeReport, EffectChange, FunctionChange, GraphDelta};
use super::{display, short};

const URL: &str = "https://github.com/o/r/blob/abc";

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
fn it_says_which_check_holds_the_rating_above_low_and_lists_what_it_found() {
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
        "### Merge risk: medium 🟡\n\n**Why it isn't low:**\n\n\
         - **Changed code** is medium 🟡: {over}.\n\n\
         <details><summary><strong>At a glance:</strong> 2 functions in 1 file, used by up to 4</summary>\n\n\
         **Impact:** 2 functions in 1 file · 1 changed, 1 removed · 1383 unchanged · used by up to 4\n\
         **Signatures:** 1 changed 👀\n\
         **Calls:** classified ✅ · no call rule matched; every new call is classified and resolved\n\
         **Effects:** gains mutation 👀\n\
         **Coverage:** limited 👀 · 3 changed test functions not rated · 2 generated functions not \
         compared (0 changed)\n\n</details>\n\n\
         <details><summary><strong>Changed code:</strong> medium 🟡 · {over}</summary>\n\n\
         - [`FromPackages`]({URL}/native/source.go#L25) in `native/source.go` — changed · \
         signature changed · gains mutation `f.typed` · calls more `go::slices.Clone` · \
         calls less `go::goeffects.packageErrors` · used by 4 (1 directly)\n\
         - `typedFiles` in `native/source.go` — removed · used by 4 (1 directly)\n\n</details>"
    );
    assert_eq!(markdown, expected);
}

#[test]
fn several_checks_share_one_fold_each_under_its_own_heading() {
    let mut load = change("load", ChangeKind::Changed, 7);
    load.calls_added = vec!["go::os.WriteFile".into()];
    let mut writes = found(
        "added_call",
        Level::High,
        "added call: go::os.WriteFile",
        &load,
    );
    writes.callee = Some("go::os.WriteFile".into());
    let within = "the change is within the low-risk limit of 3 functions";
    let size = found("pdg_change", Level::Low, within, &load);
    let markdown = report(Level::High, vec![size, writes], vec![load]).markdown(Some(URL));

    let why = "**Why it isn't low:**\n\n- **Calls a rule names** is high 🔴: added call.\n\n";
    assert!(markdown.contains(why), "{markdown}");
    assert!(
        !markdown.contains("- **Changed code**"),
        "a low check doesn't hold it: {markdown}"
    );
    assert!(
        markdown.contains("**Calls:** matched 👀 · 1 call rule match\n"),
        "{markdown}"
    );
    let found = format!(
        "<details><summary><strong>What it found:</strong> Changed code 🟢 · Calls a rule names 🔴\
         </summary>\n\n\
         #### Changed code: low 🟢\n\n{within}\n\n\
         - [`load`]({URL}/native/source.go#L7) in `native/source.go` — changed · \
         calls more `go::os.WriteFile` · used by 4 (1 directly)\n\n\
         #### Calls a rule names: high 🔴\n\nadded call\n\n\
         - [`load`]({URL}/native/source.go#L7) in `native/source.go` calls `go::os.WriteFile`\n\n\
         </details>"
    );
    assert!(markdown.ends_with(&found), "{markdown}");
}

#[test]
fn nothing_found_is_said_once_and_no_check_folds() {
    let markdown = report(Level::Low, Vec::new(), Vec::new()).markdown(None);
    let expected = "### Merge risk: low 🟢\n\n**Every check is low or found nothing.**\n\n\
        <details><summary><strong>At a glance:</strong> no function changed</summary>\n\n\
        **Impact:** no function changed · 1383 unchanged\n**Signatures:** unchanged ✅\n\
        **Calls:** classified ✅ · no call rule matched; every new call is classified and resolved\n\
        **Effects:** unchanged ✅\n\
        **Coverage:** limited 👀 · 3 changed test functions not rated · 2 generated functions not \
        compared (0 changed)\n\n</details>";
    assert_eq!(markdown, expected);
}

#[test]
fn an_unrated_report_says_why_nothing_is_rated() {
    let mut unrated = report(Level::Unrated, Vec::new(), Vec::new());
    unrated.rating.policy_configured = false;
    let markdown = unrated.markdown(None);
    let expected = "### Merge risk: unrated ⚪\n\n**Not rated:** the base's `rstyle.toml` has no \
        `[risk]` rules, so nothing here is rated. The checks below still show what each rule found.";
    assert!(markdown.starts_with(expected), "{markdown}");
}

fn unclassified(change: &FunctionChange, callee: &str) -> Reason {
    let message = format!("new library call nothing classifies: {callee}");
    Reason {
        callee: Some(callee.into()),
        ..found("unclassified_call", Level::Unknown, &message, change)
    }
}

#[test]
fn each_kind_of_reason_in_a_check_gets_a_heading_the_stronger_first() {
    let first = change("first", ChangeKind::Changed, 1);
    let second = change("second", ChangeKind::Changed, 2);
    let quiet = change("quiet", ChangeKind::TextOnly, 3);
    let depends = "what it depends on or does changed";
    let reasons = vec![
        found("pdg_change", Level::Unrated, depends, &first),
        found("pdg_change", Level::Unrated, depends, &second),
        found(
            "pdg_change",
            Level::Unknown,
            "its text changed outside the compared facts",
            &quiet,
        ),
    ];
    let markdown = report(Level::Unknown, reasons, vec![first, second, quiet]).markdown(None);

    let changed = "<details><summary><strong>Changed code:</strong> unknown 👀 · its text changed \
        outside the compared facts; what it depends on or does changed: 2 functions</summary>\n\n\
        **its text changed outside the compared facts** — unknown 👀\n\n\
        - `quiet` in `native/source.go` — text only · used by 4 (1 directly)\n\n\
        **what it depends on or does changed: 2 functions** — unrated ⚪\n\n";
    assert!(markdown.contains(changed), "{markdown}");
    let signatures =
        "**Signatures:** incomplete 👀 · 0 signature changes recorded; comparison has gaps\n";
    assert!(markdown.contains(signatures), "{markdown}");
    assert!(
        markdown.contains("**Coverage:** limited 👀 · 1 text-only function · "),
        "{markdown}"
    );
}

#[test]
fn library_calls_nothing_classifies_are_counted_by_call_and_function() {
    let map = "core::iter::Iterator::map";
    let mut first = change("first", ChangeKind::Changed, 1);
    first.calls_unclassified = vec![map.into(), "core::iter::Iterator::collect".into()];
    let mut second = change("second", ChangeKind::Changed, 2);
    second.calls_unclassified = vec![map.into()];
    let reasons = vec![
        unclassified(&first, map),
        unclassified(&first, "core::iter::Iterator::collect"),
        unclassified(&second, map),
    ];
    let markdown = report(Level::Unknown, reasons, vec![first, second]).markdown(None);

    let calls = format!(
        "<details><summary><strong>Unclassified library calls:</strong> unknown 👀 · \
         new library call nothing classifies: 3 calls in 2 functions</summary>\n\n\
         - `first` in `native/source.go` calls `{map}`\n"
    );
    assert!(markdown.contains(&calls), "{markdown}");
    let line = "**Calls:** incomplete 👀 · no call rule matched; 3 new calls nothing classifies\n";
    assert!(markdown.contains(line), "{markdown}");
}

#[test]
fn a_long_list_says_how_many_more_there_are() {
    let changes: Vec<FunctionChange> = (1..=27)
        .map(|line| change(&format!("f{line}"), ChangeKind::Added, line))
        .collect();
    let over = "27 functions changed, more than the low-risk limit of 3";
    let reasons = changes
        .iter()
        .map(|change| found("pdg_change", Level::Medium, over, change))
        .collect();
    let markdown = report(Level::Medium, reasons, changes).markdown(None);

    assert_eq!(markdown.matches("\n- `f").count(), 25, "{markdown}");
    let last =
        "`f25` in `native/source.go` — added · used by 4 (1 directly)\n- …and 2 more\n\n</details>";
    assert!(markdown.contains(last), "{markdown}");
}

#[test]
fn what_markdown_or_html_would_read_as_syntax_stays_text() {
    let odd = change("f", ChangeKind::Unknown, 1);
    let reasons = vec![found(
        "pdg_change",
        Level::Unknown,
        "holds <T> and *ptr",
        &odd,
    )];
    let markdown = report(Level::Unknown, reasons, vec![odd]).markdown(None);

    let why = "- **Changed code** is unknown 👀: holds \\<T\\> and \\*ptr.\n";
    assert!(markdown.contains(why), "{markdown}");
    assert!(
        markdown.contains("unknown 👀 · holds &lt;T&gt; and *ptr</summary>"),
        "{markdown}"
    );
}

#[test]
fn a_name_keeps_its_type_and_drops_the_module_its_file_gives() {
    assert_eq!(
        display("merge_risk::markdown::Links::<'_>::to"),
        "Links::to"
    );
    assert_eq!(
        display("merge_risk::markdown::<impl merge_risk::ChangeReport>::markdown"),
        "ChangeReport::markdown"
    );
    assert_eq!(display("merge_risk::markdown::code"), "code");
    assert_eq!(
        display("(loadedFiles).addFailed"),
        "(loadedFiles).addFailed"
    );
}

#[test]
fn a_short_callee_keeps_its_package_and_namespace() {
    assert_eq!(
        short("go::github.com/o/r/native.(loadedFiles).reason"),
        "go::native.(loadedFiles).reason"
    );
    assert_eq!(short("go::(io/fs.FileInfo).Size"), "go::(fs.FileInfo).Size");
    assert_eq!(
        short("core::clone::Clone::clone"),
        "core::clone::Clone::clone"
    );
}

#[test]
fn a_change_outside_any_function_lists_its_file_and_a_build_writes_none() {
    let outside = Reason {
        rule: "analysis_unavailable",
        level: Level::Unknown,
        message: "it changed outside any function".into(),
        path: Some("params/config.go".into()),
        function: None,
        line: None,
        callee: None,
    };
    let generated = Reason {
        message: "1 functions a macro or a build writes changed".into(),
        path: None,
        ..outside.clone()
    };
    let mut changed = report(Level::Unknown, vec![outside, generated], Vec::new());
    changed.changed_outside_functions = vec!["params/config.go".into()];
    let markdown = changed.markdown(Some(URL));

    let check = format!(
        "**it changed outside any function** — unknown 👀\n\n\
         - [`params/config.go`]({URL}/params/config.go)\n\n\
         **1 functions a macro or a build writes changed** — unknown 👀\n\n</details>"
    );
    assert!(markdown.ends_with(&check), "{markdown}");
    let coverage = "**Coverage:** limited 👀 · 1 file changed outside a function · ";
    assert!(markdown.contains(coverage), "{markdown}");
}
