use super::super::errors::{ErrorChange, Errors};
use super::super::rating::{Level, Rating, Reason};
use super::super::{
    ChangeKind, ChangeReport, EffectChange, FunctionChange, GraphDelta, SignatureChange,
};
use super::{distinct_names, name};
use crate::cpg::failures::Source;

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
        signature: None,
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
        errors: Default::default(),
        workspace: Default::default(),
    }
}

fn effect(kind: &'static str, label: &str) -> EffectChange {
    EffectChange {
        kind,
        label: label.into(),
    }
}

/// The fold of the check named `name`, from its summary to its last item.
fn fold<'m>(markdown: &'m str, name: &str) -> &'m str {
    let start = markdown
        .find(&format!("<b>{name}</b>"))
        .unwrap_or_else(|| panic!("no {name} check: {markdown}"));
    let end = start + markdown[start..].find("</details>").unwrap();
    &markdown[start..end]
}

fn files(count: usize) -> Vec<FunctionChange> {
    let mut changes = Vec::new();
    for at in 0..count {
        let mut function = change(&format!("f{at}"), ChangeKind::Changed, 1);
        function.path = format!("src/f{at}.rs");
        changes.push(function);
    }
    changes
}

#[test]
fn it_leads_with_why_then_folds_each_check() {
    let mut moved = change("FromPackages", ChangeKind::Changed, 25);
    moved.signature_changed = true;
    moved.signature = Some(SignatureChange {
        before: "func FromPackages(p []*Package) (Effects, error)".into(),
        after: "func FromPackages(p []*Package) Effects".into(),
    });
    moved.calls_added = vec!["go::slices.Clone".into()];
    moved.calls_removed = vec!["go::github.com/o/r/internal/goeffects.packageErrors".into()];
    moved.effects_gained = vec![effect("mutation", "f.typed")];
    let gone = change("typedFiles", ChangeKind::Removed, 75);
    let over = "2 functions changed, more than the low-risk limit of 1";
    let reasons = vec![
        found("pdg_change", Level::Medium, over, &moved),
        found("pdg_change", Level::Medium, over, &gone),
    ];
    let mut report = report(Level::Medium, reasons, vec![moved, gone]);
    report.workspace = ["go::github.com/o/r/internal/goeffects.packageErrors".to_owned()].into();
    let markdown = report.markdown(Some(URL));

    let at = |side_line: &str| {
        format!(
            "{URL}/files#diff-69a4aa84cf12711d033673793bd9079bfc0cfbb545bfc1c60bd7ea36c05d0065{side_line}"
        )
    };
    let expected = format!(
        "### 🟡 Merge risk: medium\n\n- 🟡 **Changed code**: {over}.\n\n\
         <details><summary>✅ <b>Changed</b>: 2 functions in 1 file: 1 changed, 1 removed</summary>\n\n\
         - [`FromPackages`]({}): changed; signature changed; gains mutation; now calls `slices.Clone`; \
         no longer calls `goeffects.packageErrors`\n\
         - [`typedFiles`]({}): removed\n\
         - 3 test functions changed, which aren't rated\n\n</details>\n\
         <details><summary>👀 <b>Signatures</b>: 1 changed, 1 caller</summary>\n\n\
         - [`FromPackages`]({}): `func FromPackages(p []*Package) (Effects, error)` → \
         `func FromPackages(p []*Package) Effects` (1 caller)\n\n</details>\n\
         <details><summary>👀 <b>Effects</b>: 1 gained</summary>\n\n\
         - [`FromPackages`]({}) gains mutation: `f.typed`\n\n</details>\n\
         <details><summary>✅ <b>Calls</b>: 1 no effects</summary>\n\n\
         - new, no effect: `slices.Clone` in [`FromPackages`]({})\n\n</details>",
        at("R25"),
        at("L75"),
        at("R25"),
        at("R25"),
        at("R25"),
    );
    assert_eq!(markdown, expected);
}

#[test]
fn nothing_changed_reads_fine_and_says_what_it_checked() {
    let markdown = report(Level::Low, Vec::new(), Vec::new()).markdown(None);
    let expected = "### 🟢 Merge risk: low\n\nNothing holds it above low.\n\n\
        <details><summary>✅ <b>Changed</b>: no function changed, in 0 files</summary>\n\n\
        - 3 test functions changed, which aren't rated\n\n</details>\n\
        <details><summary>✅ <b>Signatures</b>: none changed</summary>\n\n\
        - none of the 0 changed functions changed its parameters or results\n\n</details>\n\
        <details><summary>✅ <b>Effects</b>: none gained or lost</summary>\n\n\
        - no function gained or lost io, network, global state, mutation or unsafe code\n\n</details>\n\
        <details><summary>✅ <b>Calls</b>: no library calls added or removed</summary>\n\n\
        - no changed function added or dropped a call outside this repository\n\n</details>";
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
fn a_rule_naming_a_call_says_which() {
    let mut load = change("load", ChangeKind::Changed, 7);
    load.calls_added = vec!["go::os.WriteFile".into()];
    load.effects_gained = vec![effect("io", "os.WriteFile")];
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
    let calls = fold(&markdown, "Calls");
    assert!(
        calls.starts_with("<b>Calls</b>: 1 with effects</summary>"),
        "{calls}"
    );
    assert!(
        calls.contains("- new, io: `os.WriteFile` in `load`"),
        "{calls}"
    );
    assert!(markdown.contains("👀 <b>Calls</b>"), "{markdown}");
}

#[test]
fn a_call_nothing_classifies_leaves_calls_unknown() {
    let mut busy = change("busy", ChangeKind::Changed, 1);
    busy.calls_added = vec!["core::iter::Iterator::f00".into(), "core::cmp::max".into()];
    busy.calls_unclassified = vec!["core::iter::Iterator::f00".into()];
    let reasons = vec![unclassified(&busy, "core::iter::Iterator::f00")];
    let markdown = report(Level::Unknown, reasons, vec![busy]).markdown(None);

    let why = "- 👀 **Unclassified library calls**: new library call nothing classifies: \
        `Iterator::f00`.\n\n";
    assert!(markdown.contains(why), "{markdown}");
    let calls = fold(&markdown, "Calls");
    assert!(
        markdown.contains("❓ <b>Calls</b>: 1 effect unknown, 1 no effects"),
        "{markdown}"
    );
    assert!(
        calls.contains("- new, effect unknown: `Iterator::f00` in `busy`\n"),
        "{calls}"
    );
    assert!(
        calls.contains("- new, no effect: `max` in `busy`"),
        "{calls}"
    );
}

#[test]
fn a_call_moved_between_functions_is_neither_new_nor_removed() {
    let mut from = change("from", ChangeKind::Changed, 1);
    from.calls_removed = vec!["go::fmt.Sprintf".into()];
    let mut to = change("to", ChangeKind::Changed, 2);
    to.calls_added = vec!["go::fmt.Sprintf".into()];
    let markdown = report(Level::Low, Vec::new(), vec![from, to]).markdown(None);
    assert!(
        markdown.contains("✅ <b>Calls</b>: no library calls added or removed"),
        "{markdown}"
    );
}

#[test]
fn calls_a_removed_function_made_read_as_one_line() {
    let mut gone = change("packageErrors", ChangeKind::Removed, 9);
    gone.calls_removed = vec!["go::slices.Sort".into(), "go::strings.Join".into()];
    let markdown = report(Level::Low, Vec::new(), vec![gone]).markdown(None);
    let calls = fold(&markdown, "Calls");
    assert!(
        calls.contains("- removed with `packageErrors`: `slices.Sort`, `strings.Join`"),
        "{calls}"
    );
}

#[test]
fn a_call_a_changed_function_drops_reads_as_removed_from_it() {
    let mut save = change("save", ChangeKind::Changed, 4);
    save.calls_removed = vec!["go::os.WriteFile".into()];
    let markdown = report(Level::Low, Vec::new(), vec![save]).markdown(None);
    assert!(
        markdown.contains("✅ <b>Calls</b>: 1 removed</summary>"),
        "{markdown}"
    );
    let calls = fold(&markdown, "Calls");
    assert!(
        calls.contains("- removed: `os.WriteFile` in `save`"),
        "{calls}"
    );
}

#[test]
fn a_signature_change_with_no_text_still_says_it_changed() {
    let mut open = change("open", ChangeKind::Changed, 12);
    open.signature_changed = true;
    open.callers = 3;
    let markdown = report(Level::Low, Vec::new(), vec![open]).markdown(None);
    let signatures = fold(&markdown, "Signatures");
    assert!(
        signatures.contains("- `open`: signature changed (3 callers)"),
        "{signatures}"
    );
}

#[test]
fn over_five_files_reads_as_one_to_look_at() {
    let small = report(Level::Low, Vec::new(), files(5)).markdown(None);
    assert!(
        small.contains("✅ <b>Changed</b>: 5 functions in 5 files: 5 changed</summary>"),
        "{small}"
    );
    let large = report(Level::Low, Vec::new(), files(6)).markdown(None);
    let look = "👀 <b>Changed</b>: 6 functions in 6 files: 6 changed \
        (over 5 files rarely reverts cleanly)</summary>";
    assert!(large.contains(look), "{large}");
}

#[test]
fn what_it_cannot_see_leaves_changed_unknown_and_listed() {
    let changes = vec![
        change("quiet", ChangeKind::TextOnly, 1),
        change("odd", ChangeKind::Unknown, 2),
    ];
    let mut unseen = report(Level::Unknown, Vec::new(), changes);
    unseen.changed_outside_functions = vec!["params/config.go".into()];
    unseen.not_compared_changed = 1;
    let markdown = unseen.markdown(Some(URL));

    let changed = fold(&markdown, "Changed");
    assert!(
        markdown.contains("❓ <b>Changed</b>: 2 functions in 2 files: 1 text only, 1 unknown"),
        "{markdown}"
    );
    let outside = format!(
        "- [`params/config.go`]({URL}/files#diff-10ff00a15ef58da7485fedeca3906c73cb236fde4b143a9d61601c63cbf1ffe8): \
         changed outside any function"
    );
    assert!(changed.contains(&outside), "{changed}");
    let generated =
        "- 1 generated function changed, which a macro or a build writes and nothing compares";
    assert!(changed.contains(generated), "{changed}");
    assert!(
        markdown.contains("✅ <b>Signatures</b>: none changed"),
        "{markdown}"
    );
}

#[test]
fn an_effect_that_moved_under_a_new_name_is_neither_gained_nor_lost() {
    let mut old = change("addPackage", ChangeKind::Removed, 84);
    old.effects_lost = vec![effect("mutation", "typed")];
    let mut new = change("(loadedFiles).add", ChangeKind::Added, 92);
    new.effects_gained = vec![effect("mutation", "f.typed")];
    let markdown = report(Level::Low, Vec::new(), vec![old, new]).markdown(None);

    let effects = fold(&markdown, "Effects");
    assert!(
        markdown.contains(
            "✅ <b>Effects</b>: 1 moved between functions, 1 new function, none does io or network"
        ),
        "{markdown}"
    );
    let moved = "- mutation `typed` moved from `addPackage` to `(loadedFiles).add` as `f.typed`";
    assert!(effects.contains(moved), "{effects}");
}

#[test]
fn an_effect_renamed_within_one_function_is_neither_gained_nor_lost() {
    let mut add = change("add", ChangeKind::Changed, 5);
    add.effects_lost = vec![effect("mutation", "typed")];
    add.effects_gained = vec![effect("mutation", "f.typed")];
    let markdown = report(Level::Low, Vec::new(), vec![add]).markdown(None);
    assert!(
        markdown.contains("✅ <b>Effects</b>: none gained or lost"),
        "{markdown}"
    );
}

#[test]
fn a_lost_effect_is_to_look_at_and_a_new_unclassified_call_is_listed() {
    let mut save = change("save", ChangeKind::Changed, 4);
    save.effects_lost = vec![effect("io", "os.WriteFile"), effect("unknown", "store.Put")];
    let mut helper = change("helper", ChangeKind::Added, 9);
    helper.effects_gained = vec![effect("unknown", "client.Do")];
    let markdown = report(Level::Low, Vec::new(), vec![save, helper]).markdown(None);

    let summary = "👀 <b>Effects</b>: 1 lost, 1 new function, none does io or network, \
        1 call nothing classifies</summary>";
    assert!(markdown.contains(summary), "{markdown}");
    let effects = fold(&markdown, "Effects");
    assert!(
        effects.contains("- `save` loses io: `os.WriteFile`"),
        "{effects}"
    );
    assert!(!effects.contains("store.Put"), "{effects}");
    assert!(
        effects.contains("- `helper` is new and calls `client.Do`, which nothing classifies"),
        "{effects}"
    );
}

#[test]
fn a_new_function_that_does_io_is_named() {
    let mut main = change("main", ChangeKind::Added, 3);
    main.effects_gained = vec![effect("io", "fmt.Fprintln")];
    let markdown = report(Level::Low, Vec::new(), vec![main]).markdown(None);
    assert!(
        markdown.contains("👀 <b>Effects</b>: 1 new function, <code>main</code> does io</summary>"),
        "{markdown}"
    );
}

#[test]
fn errors_say_where_a_change_is_made_and_who_passes_it_on() {
    let open = change("open", ChangeKind::Changed, 12);
    let closed = Source::Variable("go::example.com/errs.ErrClosed".into());
    let failing = |name: &str, line: usize, wrapped: usize| ErrorChange {
        function: format!("go::example.com/errs.{name}"),
        name: name.into(),
        path: "native/source.go".into(),
        line,
        lost: vec![Source::Message("no %s: %w".into())],
        gained: vec![closed.clone()],
        wrapped,
    };
    let mut report = report(Level::Low, Vec::new(), vec![open]);
    report.errors = Errors {
        traced: true,
        followed: 1,
        changed: vec![failing("Load", 23, 1), failing("open", 12, 0)],
        untraced: Vec::new(),
    };
    let markdown = report.markdown(None);

    let errors = fold(&markdown, "Errors");
    let line = "- `open` no longer fails with \"no …: …\"; can now fail with `errs.ErrClosed`, \
        and so does `Load`, which wraps it";
    assert!(
        markdown.contains("👀 <b>Errors</b>: 1 function fails differently"),
        "{markdown}"
    );
    assert!(errors.contains(line), "{errors}");
}

/// A function whose errors changed, which can now also come from `gained`.
fn error_change(name: &str, gained: Vec<Source>) -> ErrorChange {
    ErrorChange {
        function: format!("go::example.com/errs.{name}"),
        name: name.into(),
        path: "elsewhere.go".into(),
        line: 3,
        lost: Vec::new(),
        gained,
        wrapped: 0,
    }
}

#[test]
fn a_function_that_newly_fails_through_a_function_value_reads_unknown() {
    let mut report = report(Level::Low, Vec::new(), Vec::new());
    report.errors = Errors {
        traced: true,
        followed: 1,
        changed: vec![error_change("Each", vec![Source::Unknown])],
        untraced: Vec::new(),
    };
    let markdown = report.markdown(None);

    let summary =
        "❓ <b>Errors</b>: 1 function can now fail through a call it can't follow</summary>";
    assert!(markdown.contains(summary), "{markdown}");
    let item =
        "- `Each` can now also fail through a call it can't follow, such as a function value";
    assert!(fold(&markdown, "Errors").contains(item), "{markdown}");
}

#[test]
fn errors_say_what_changed_then_what_newly_can_t_be_followed() {
    let mut report = report(Level::Low, Vec::new(), Vec::new());
    report.errors = Errors {
        traced: true,
        followed: 2,
        changed: vec![
            error_change("open", vec![Source::Call("go::os.Open".into())]),
            error_change("Each", vec![Source::Unknown]),
        ],
        untraced: Vec::new(),
    };
    let markdown = report.markdown(None);

    let summary =
        "👀 <b>Errors</b>: 1 function fails differently, 1 more it can't follow</summary>";
    assert!(markdown.contains(summary), "{markdown}");
    let errors = fold(&markdown, "Errors");
    let items = "- `open` can now fail with an error from `os.Open`\n\
        - `Each` can now also fail through a call it can't follow, such as a function value";
    assert!(errors.contains(items), "{errors}");
}

#[test]
fn no_change_seen_is_unknown_when_a_changed_function_can_t_be_followed() {
    let mut report = report(Level::Low, Vec::new(), Vec::new());
    report.errors = Errors {
        traced: true,
        followed: 1,
        changed: Vec::new(),
        untraced: vec![error_change("Each", Vec::new())],
    };
    let markdown = report.markdown(None);

    let summary = "❓ <b>Errors</b>: no change seen, but 1 function can fail through a call it can't \
        follow</summary>";
    assert!(markdown.contains(summary), "{markdown}");
    let item = "- `Each` can fail through a call it can't follow, such as a function value";
    assert!(fold(&markdown, "Errors").contains(item), "{markdown}");
}

#[test]
fn errors_that_all_trace_and_didn_t_change_read_fine() {
    let mut report = report(Level::Low, Vec::new(), Vec::new());
    report.errors = Errors {
        traced: true,
        followed: 2,
        changed: Vec::new(),
        untraced: Vec::new(),
    };
    let markdown = report.markdown(None);

    let item = "- followed every error the 2 changed functions can return back to where it's made; \
        none was added or dropped";
    assert!(
        markdown.contains("✅ <b>Errors</b>: no function fails in new ways or stops failing"),
        "{markdown}"
    );
    assert!(fold(&markdown, "Errors").contains(item), "{markdown}");
}

#[test]
fn errors_read_fine_when_no_changed_function_returns_one() {
    let mut report = report(Level::Low, Vec::new(), Vec::new());
    report.errors = Errors {
        traced: true,
        ..Errors::default()
    };
    let markdown = report.markdown(None);

    let item = "- no changed function returns an error, and no other function's errors changed";
    assert!(
        markdown.contains("✅ <b>Errors</b>: no function fails in new ways or stops failing"),
        "{markdown}"
    );
    assert!(fold(&markdown, "Errors").contains(item), "{markdown}");
}

#[test]
fn errors_are_left_out_for_a_language_they_aren_t_read_for() {
    let markdown = report(Level::Low, Vec::new(), Vec::new()).markdown(None);
    assert!(!markdown.contains("<b>Errors</b>"), "{markdown}");
}

#[test]
fn a_summary_is_html_and_an_item_stays_markdown() {
    let odd = change("f|g<T>", ChangeKind::Unknown, 1);
    let reasons = vec![found(
        "pdg_change",
        Level::Unknown,
        "holds <T> and *ptr",
        &odd,
    )];
    let mut main = change("main<T>", ChangeKind::Added, 2);
    main.effects_gained = vec![effect("io", "os.Exit")];
    let markdown = report(Level::Unknown, reasons, vec![odd, main]).markdown(None);

    let why = "- 👀 **Changed code**: holds \\<T\\> and \\*ptr.\n";
    assert!(markdown.contains(why), "{markdown}");
    assert!(markdown.contains("- `f|g<T>`: unknown"), "{markdown}");
    assert!(
        markdown.contains("<code>main&lt;T&gt;</code> does io"),
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

/// Identities rstyle-cxx emits, whose parameters, template arguments and return types hold `::`,
/// and how the report names each.
const CXX_NAMES: [(&str, &str); 12] = [
    (
        "cxx::llvm::sys::fs::remove(const llvm::Twine &, bool)",
        "fs::remove",
    ),
    (
        "cxx::std::filesystem::remove(const std::filesystem::path &)",
        "filesystem::remove",
    ),
    (
        "cxx::llvm::sys::ExecuteAndWait(llvm::StringRef, llvm::ArrayRef<llvm::StringRef>, \
             std::optional<llvm::ArrayRef<llvm::StringRef>>, \
             std::optional<llvm::sys::ProcessStatistics> *, llvm::BitVector *)",
        "sys::ExecuteAndWait",
    ),
    ("cxx::clang::Decl::isImplicit() const", "Decl::isImplicit"),
    (
        "cxx::llvm::MemoryBuffer::getFile(const llvm::Twine &, bool, \
             std::optional<llvm::Align>)",
        "MemoryBuffer::getFile",
    ),
    (
        "cxx::std::get_if<typename, typename...>(variant<type-parameter-0-1...> *) \
             -> typename add_pointer<type-parameter-0-0>::type",
        "std::get_if",
    ),
    (
        "cxx::llvm::SmallVectorTemplateBase<type-parameter-0-0, true>::push_back(\
             typename conditional<TakesParamByValue, type-parameter-0-0>::type)",
        "SmallVectorTemplateBase::push_back",
    ),
    (
        "cxx::llvm::ArrayRef::ArrayRef<T><typename>(const vector<type-parameter-0-0> &) \
             -> void",
        "ArrayRef::ArrayRef",
    ),
    (
        "cxx::std::optional::operator bool() const -> bool",
        "optional::operator bool",
    ),
    (
        "cxx::src/database.cpp::rstylecxx::(anonymous namespace)::runFrom(llvm::StringRef, \
             const std::vector<std::basic_string<char>> &)",
        "runFrom",
    ),
    ("cxx::src/main.cpp::helper(int)", "helper"),
    ("cxx::system(const char *)", "system"),
];

#[test]
fn names_that_shorten_to_one_keep_as_much_path_as_tells_them_apart() {
    let hash = "std::collections::hash::map::{impl#4}::get";
    let btree = "alloc::collections::btree::map::{impl#20}::get";
    let alone = "core::option::{impl#0}::unwrap_or_default";
    let names = distinct_names([hash, btree, alone]);
    assert_eq!(names[hash], "hash::map::get");
    assert_eq!(names[btree], "btree::map::get");
    assert_eq!(names[alone], "option::unwrap_or_default");
}

#[test]
fn calls_tell_apart_two_callees_with_one_short_name() {
    let mut lookup = change("lookup", ChangeKind::Changed, 3);
    lookup.calls_added = vec![
        "std::collections::hash::map::{impl#4}::get".into(),
        "alloc::collections::btree::map::{impl#20}::get".into(),
    ];
    let markdown = report(Level::Low, Vec::new(), vec![lookup]).markdown(None);

    let calls = fold(&markdown, "Calls");
    assert!(
        calls.contains("- new, no effect: `btree::map::get` in `lookup`"),
        "{calls}"
    );
    assert!(
        calls.contains("- new, no effect: `hash::map::get` in `lookup`"),
        "{calls}"
    );
}

#[test]
fn a_cxx_name_drops_its_signature_and_keeps_its_scope() {
    for (identity, expected) in CXX_NAMES {
        assert_eq!(name(identity), expected, "{identity}");
    }
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
