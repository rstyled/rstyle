use super::*;
use crate::cpg::records::DeclaredType;
use crate::facts::{CallEvent, Edge, EdgeKind, EffectEvent, EffectKind, PdgEdge, Receiver, sample};
use crate::shape::fixtures::method;

fn signature(result: &str) -> DeclaredSignature {
    let declared = DeclaredType {
        normalized: None,
        observed: result.into(),
        spelling: result.into(),
    };
    DeclaredSignature {
        parameters: Vec::new(),
        result: declared,
        generics: Vec::new(),
        has_self: false,
        modifiers: Vec::new(),
    }
}

fn node(kind: NodeKind, callee: Option<&str>, line: usize) -> PdgNode {
    PdgNode {
        kind,
        ty: None,
        callee_id: callee.map(str::to_owned),
        callee_sig: None,
        macro_name: None,
        lit_kind: None,
        detail: String::new(),
        line,
    }
}

fn calling(callees: &[&str]) -> Pdg {
    Pdg {
        nodes: callees
            .iter()
            .enumerate()
            .map(|(line, callee)| node(NodeKind::Call, Some(callee), line + 1))
            .collect(),
        edges: Vec::new(),
    }
}

fn function(name: &str, graph: Pdg) -> FnFacts {
    let mut facts = sample(name);
    facts.pdg = Some(graph);
    facts
}

struct Fixture {
    snapshot: Snapshot,
}

impl Fixture {
    fn new() -> Self {
        Self {
            snapshot: Snapshot::default(),
        }
    }

    fn with(mut self, facts: FnFacts) -> Self {
        self.snapshot
            .index
            .insert(facts.id.clone(), self.snapshot.facts.len());
        self.snapshot.facts.push(facts);
        self.snapshot.declared.push(Declared {
            signature: signature("()"),
            shapes: Vec::new(),
            values: Vec::new(),
            gap: None,
        });
        self
    }
}

fn snapshot(functions: Vec<FnFacts>) -> Snapshot {
    functions
        .into_iter()
        .fold(Fixture::new(), Fixture::with)
        .snapshot
}

/// What the documents alone say, with nothing else changed in the runs' commits.
fn compare(base: &Snapshot, head: &Snapshot, config: &Config) -> ChangeReport {
    let commits = Commits::Compared {
        prefix: String::new(),
        changed: Vec::new(),
    };
    super::compare(base, head, config, &commits).unwrap()
}

fn report(base: Vec<FnFacts>, head: Vec<FnFacts>) -> ChangeReport {
    compare(&snapshot(base), &snapshot(head), &Config::default())
}

fn only(report: &ChangeReport) -> &FunctionChange {
    let [change] = &report.changes[..] else {
        panic!("{report:?}");
    };
    change
}

#[test]
fn a_function_that_says_the_same_is_unchanged_whatever_line_it_is_on() {
    let moved = {
        let mut facts = function("f", calling(&["g"]));
        facts.pdg.as_mut().unwrap().nodes[0].line = 40;
        facts.line = 39;
        facts
    };
    let report = report(vec![function("f", calling(&["g"]))], vec![moved]);
    assert!(report.changes.is_empty(), "{report:?}");
    assert_eq!(report.unchanged, 1);
}

fn impl_method(id: &str, owner: &str, callee: &str) -> FnFacts {
    let mut facts = function("of", calling(&[callee]));
    facts.id = id.into();
    facts.sig = Some(method(
        owner,
        Receiver::Ref,
        vec![],
        TyShape::Tuple(Vec::new()),
    ));
    facts
}

#[test]
fn adding_an_impl_block_renumbers_nothing_that_matters() {
    let base = snapshot(vec![
        impl_method("krate::{impl#0}::of", "krate::Weights", "g"),
        {
            let mut caller = function("caller", calling(&["krate::{impl#0}::of"]));
            caller.line = 9;
            caller
        },
    ]);
    let head = snapshot(vec![
        impl_method("krate::{impl#5}::of", "krate::Weights", "g"),
        {
            let mut caller = function("caller", calling(&["krate::{impl#5}::of"]));
            caller.line = 9;
            caller
        },
    ]);
    let report = compare(
        &base.corresponded(),
        &head.corresponded(),
        &Config::default(),
    );
    assert!(report.changes.is_empty(), "{report:?}");
    assert_eq!(report.unchanged, 2);
}

#[test]
fn a_constant_changing_where_it_is_used_is_a_change_though_the_text_is_the_same() {
    let mut head = snapshot(vec![function("f", calling(&["g"]))]);
    head.declared[0].values = vec!["20".into()];
    let mut base = snapshot(vec![function("f", calling(&["g"]))]);
    base.declared[0].values = vec!["10".into()];
    let report = compare(&base, &head, &Config::default());
    let change = only(&report);
    assert_eq!(change.kind, ChangeKind::Changed);
    assert!(change.values_changed);
    assert!(report.text().contains("a known value changed"));
}

#[test]
fn a_different_callee_is_a_change_that_says_which() {
    let report = report(
        vec![function("f", calling(&["g", "h"]))],
        vec![function("f", calling(&["g", "os.WriteFile"]))],
    );
    let change = only(&report);
    assert_eq!(change.kind, ChangeKind::Changed);
    assert_eq!(change.calls_added, ["os.WriteFile"]);
    assert_eq!(change.calls_removed, ["h"]);
    assert_eq!(change.graph.nodes_added, 1);
    assert_eq!(change.graph.nodes_removed, 1);
}

#[test]
fn a_signature_is_the_same_whatever_a_frontend_spells_its_types_as() {
    let declared = |observed: &str, spelling: &str, shape| {
        let mut signature = signature("()");
        signature.result = DeclaredType {
            normalized: None,
            observed: observed.into(),
            spelling: spelling.into(),
        };
        Declared {
            signature,
            shapes: vec![Some(shape)],
            values: Vec::new(),
            gap: None,
        }
    };
    let tuple = |first: &str| {
        TyShape::Tuple(vec![
            TyShape::Prim(first.into()),
            TyShape::Prim("str".into()),
        ])
    };
    let before = declared("(t29, t10)", "", tuple("u8"));
    assert!(same_signature(
        &before,
        &declared("(t34, t12)", "", tuple("u8"))
    ));
    assert!(!same_signature(
        &before,
        &declared("(t34, t12)", "", tuple("u16"))
    ));

    let written = declared("x", "map[string]int", TyShape::Other);
    let other = declared("x", "map[int]int", TyShape::Other);
    assert!(
        !same_signature(&written, &other),
        "a shape can't tell these apart"
    );
}

#[test]
fn a_new_function_gains_what_it_does_and_a_removed_one_loses_it() {
    let doing = || {
        let mut facts = function("f", calling(&["os.WriteFile"]));
        facts.effects = vec![EffectEvent {
            kind: EffectKind::Unsafe,
            detail: "raw".into(),
            line: 1,
        }];
        facts
    };
    let report_added = report(vec![], vec![doing()]);
    let change = only(&report_added);
    assert_eq!(change.calls_added, ["os.WriteFile"]);
    assert_eq!(change.effects_gained[0].kind, "unsafe");
    assert!(!report_added.text().contains("calls more"));

    let report_removed = report(vec![doing()], vec![]);
    let change = only(&report_removed);
    assert_eq!(change.calls_removed, ["os.WriteFile"]);
    assert_eq!(change.effects_lost[0].kind, "unsafe");
}

fn with_files(functions: Vec<FnFacts>, files: &[(&str, &str)]) -> Snapshot {
    let mut snapshot = snapshot(functions);
    snapshot.files = files
        .iter()
        .map(|(path, digest)| ((*path).to_owned(), (*digest).to_owned()))
        .collect();
    snapshot
}

#[test]
fn a_file_that_changed_with_no_function_changing_needs_a_person() {
    let base = with_files(
        vec![],
        &[("config.go", "a"), ("same.go", "s"), ("gone.go", "g")],
    );
    let head = with_files(
        vec![],
        &[("config.go", "b"), ("same.go", "s"), ("new.go", "n")],
    );
    let report = compare(&base, &head, &Config::default());
    assert_eq!(
        report.changed_outside_functions,
        ["config.go", "gone.go", "new.go"]
    );
    assert!(
        report
            .text()
            .contains("config.go: changed outside any function")
    );
}

#[test]
fn a_file_a_function_change_already_points_to_is_not_said_twice() {
    let mut changed = function("f", calling(&["x"]));
    changed.path = "a.go".into();
    let mut before = function("f", calling(&[]));
    before.path = "a.go".into();
    let base = with_files(vec![before], &[("a.go", "1")]);
    let head = with_files(vec![changed], &[("a.go", "2")]);
    let report = compare(&base, &head, &Config::default());
    assert!(report.changed_outside_functions.is_empty(), "{report:?}");
}

#[test]
fn calling_something_more_often_is_a_change_in_how_often() {
    let report = report(
        vec![function("f", calling(&["g"]))],
        vec![function("f", calling(&["g", "g"]))],
    );
    assert_eq!(only(&report).calls_added, ["g"]);
}

#[test]
fn swapping_two_arguments_of_one_shape_is_a_change() {
    let swapped = |first, second| {
        let mut nodes = vec![
            node(NodeKind::Param, None, 1),
            node(NodeKind::Param, None, 1),
        ];
        nodes.push(node(NodeKind::Call, Some("g"), 2));
        let edge = |from, arg_pos| PdgEdge {
            from,
            to: 2,
            kind: EdgeKind::Data,
            arg_pos,
        };
        function(
            "f",
            Pdg {
                nodes,
                edges: vec![edge(first, 0), edge(second, 1)],
            },
        )
    };
    let report = report(vec![swapped(0, 1)], vec![swapped(1, 0)]);
    assert_eq!(only(&report).kind, ChangeKind::Changed);
    assert_eq!(only(&report).graph.edges_added, 2);
}

#[test]
fn text_with_no_change_in_the_facts_only_says_that() {
    let mut commented = function("f", calling(&["g"]));
    commented.source = "fn f() { /* note */ g() }".into();
    let report = report(vec![function("f", calling(&["g"]))], vec![commented]);
    let change = only(&report);
    assert_eq!(change.kind, ChangeKind::TextOnly);
    assert!(change.graph.is_empty());
    assert_eq!(report.unchanged, 0);
}

#[test]
fn a_function_one_run_lacks_is_added_or_removed() {
    let added = report(vec![], vec![function("f", calling(&[]))]);
    assert_eq!(only(&added).kind, ChangeKind::Added);
    let removed = report(vec![function("f", calling(&[]))], vec![]);
    assert_eq!(only(&removed).kind, ChangeKind::Removed);
}

#[test]
fn a_function_whose_tree_is_not_all_of_it_is_unknown_with_why() {
    let mut changed = function("f", calling(&["g"]));
    changed.source = "fn f() { g() }".into();
    let mut head = snapshot(vec![changed]);
    head.declared[0].gap = Some("The function holds a construct the frontend does not model");
    let report = compare(
        &snapshot(vec![function("f", calling(&["h"]))]),
        &head,
        &Config::default(),
    );
    let change = only(&report);
    assert_eq!(change.kind, ChangeKind::Unknown);
    assert!(change.reason.unwrap().contains("does not model"));
}

#[test]
fn an_unchanged_function_with_a_gap_has_nothing_to_report() {
    let mut head = snapshot(vec![function("f", calling(&["g"]))]);
    head.declared[0].gap = Some("gap");
    let report = compare(
        &snapshot(vec![function("f", calling(&["g"]))]),
        &head,
        &Config::default(),
    );
    assert!(report.changes.is_empty(), "{report:?}");
}

#[test]
fn an_effect_gained_says_what_it_is_and_one_lost_does_too() {
    let effect = |kind, detail: &str| EffectEvent {
        kind,
        detail: detail.into(),
        line: 1,
    };
    let (mut before, mut after) = (function("f", calling(&[])), function("f", calling(&[])));
    before.effects = vec![effect(EffectKind::Time, "clock")];
    after.effects = vec![effect(EffectKind::Unsafe, "raw")];
    after.source = "fn f() { unsafe { raw() } }".into();
    let report = report(vec![before], vec![after]);
    let change = only(&report);
    assert_eq!(change.effects_gained[0].kind, "unsafe");
    assert_eq!(change.effects_lost[0].kind, "time");
}

#[test]
fn a_signature_change_is_named() {
    let mut changed = function("f", calling(&[]));
    changed.source = "fn f() -> u8 {}".into();
    let mut head = snapshot(vec![changed]);
    head.declared[0].signature = signature("u8");
    let report = compare(
        &snapshot(vec![function("f", calling(&[]))]),
        &head,
        &Config::default(),
    );
    assert!(only(&report).signature_changed);
}

fn calls_of_workspace(name: &str, callee: &str) -> FnFacts {
    let mut facts = function(name, calling(&[]));
    facts.edges = vec![Edge {
        callee: format!("krate::{callee}"),
        line: 1,
        dynamic: false,
        required: false,
        args: Vec::new(),
    }];
    facts
}

#[test]
fn a_change_says_how_much_of_the_run_reaches_it() {
    let (a, b) = (
        calls_of_workspace("a", "b"),
        calls_of_workspace("b", "target"),
    );
    let (mut changed, unchanged) = (
        function("target", calling(&["x"])),
        function("target", calling(&[])),
    );
    changed.source = "fn target() { x() }".into();
    let report = report(vec![a.clone(), b.clone(), unchanged], vec![a, b, changed]);
    let change = only(&report);
    assert_eq!(change.callers, 1, "b calls it");
    assert_eq!(change.reached_by, 2, "a reaches it through b");
}

#[test]
fn a_recursive_function_does_not_reach_itself() {
    let mut recursive = calls_of_workspace("f", "f");
    let before = recursive.clone();
    recursive.source = "fn f() { f(); f() }".into();
    recursive.pdg = Some(calling(&["krate::f"]));
    let report = report(vec![before], vec![recursive]);
    assert_eq!(only(&report).reached_by, 0);
}

#[test]
fn changed_tests_are_counted_and_not_reported_as_changes() {
    let (mut base, mut head) = (Snapshot::default(), Snapshot::default());
    base.tests.insert("t".into(), "fn t() {}".into());
    head.tests.insert("t".into(), "fn t() { g() }".into());
    head.tests.insert("new".into(), "fn new() {}".into());
    base.tests.insert("same".into(), "fn same() {}".into());
    head.tests.insert("same".into(), "fn same() {}".into());
    let report = compare(&base, &head, &Config::default());
    assert!(report.changes.is_empty());
    assert_eq!(report.tests_changed, 2);
}

#[test]
fn the_summary_says_what_is_not_compared() {
    let mut base = snapshot(vec![]);
    base.not_compared.insert("generated".into(), "Debug".into());
    let report = compare(
        &base,
        &snapshot(vec![function("f", calling(&[]))]),
        &Config::default(),
    );
    let text = report.text();
    assert!(text.contains("1 added"), "{text}");
    assert!(
        text.contains("Generated functions not compared: 1, 1 of them changed."),
        "{text}"
    );
}

#[test]
fn what_a_macro_writes_compares_by_what_it_says_since_its_numbers_shift() {
    let with = |written: &[(&str, &str)]| {
        let mut run = snapshot(vec![]);
        for (identity, source) in written {
            run.not_compared
                .insert((*identity).to_owned(), (*source).to_owned());
        }
        run
    };
    let changed = |base: &Snapshot, head: &Snapshot| {
        compare(base, head, &Config::default()).not_compared_changed
    };

    let base = with(&[("{impl#1}::fmt", "Debug"), ("{impl#2}::clone", "Clone")]);
    let renumbered = with(&[("{impl#2}::fmt", "Debug"), ("{impl#3}::clone", "Clone")]);
    assert_eq!(changed(&base, &renumbered), 0);

    let one_more = with(&[
        ("{impl#1}::fmt", "Debug"),
        ("{impl#2}::clone", "Clone"),
        ("{impl#3}::fmt", "Debug"),
    ]);
    assert_eq!(changed(&base, &one_more), 1);
    assert_eq!(changed(&one_more, &base), 1);
}

fn call_of(callee: &str, library_callee: Option<&str>) -> CallEvent {
    CallEvent {
        callee: callee.into(),
        line: 1,
        dynamic: false,
        library: None,
        library_callee: library_callee.map(str::to_owned),
    }
}

fn making(name: &str, calls: Vec<CallEvent>) -> FnFacts {
    FnFacts {
        calls,
        ..function(name, calling(&[]))
    }
}

#[test]
fn a_new_library_call_nothing_classifies_is_said_by_its_identity() {
    let unlisted = || call_of("tokio::fs::write", Some("tokio::fs::write"));
    let report = report(
        vec![making("f", vec![unlisted()])],
        vec![FnFacts {
            source: "fn f() { /* more */ }".into(),
            ..making("f", vec![unlisted(), unlisted()])
        }],
    );
    let change = only(&report);
    assert_eq!(change.calls_unclassified, ["tokio::fs::write"]);
    assert!(
        report
            .text()
            .contains("  new library call nothing classifies: tokio::fs::write")
    );
}

#[test]
fn a_library_call_that_was_there_already_is_nothing_new() {
    let unlisted = || call_of("tokio::fs::write", Some("tokio::fs::write"));
    let report = report(
        vec![making("f", vec![unlisted(), unlisted()])],
        vec![FnFacts {
            source: "fn f() { /* more */ }".into(),
            ..making("f", vec![unlisted()])
        }],
    );
    assert!(only(&report).calls_unclassified.is_empty());
}

#[test]
fn a_new_function_says_each_library_call_nothing_classifies_and_a_removed_one_does_not() {
    let facts = || {
        making(
            "f",
            vec![call_of("tokio::fs::write", Some("tokio::fs::write"))],
        )
    };
    let added = report(vec![], vec![facts()]);
    assert_eq!(only(&added).calls_unclassified, ["tokio::fs::write"]);

    let removed = report(vec![facts()], vec![]);
    assert!(only(&removed).calls_unclassified.is_empty());
}

#[test]
fn a_call_to_an_unknown_target_in_either_run_is_said_of_a_change() {
    let indirect = || CallEvent {
        dynamic: true,
        ..call_of("<fn pointer>", None)
    };
    let edited = |calls: Vec<CallEvent>| FnFacts {
        source: "fn f() { /* edited */ }".into(),
        ..making("f", calls)
    };

    let gone = report(vec![making("f", vec![indirect()])], vec![edited(vec![])]);
    assert!(only(&gone).unresolved_calls);

    let came = report(vec![making("f", vec![])], vec![edited(vec![indirect()])]);
    let change = only(&came);
    assert!(change.unresolved_calls);
    assert!(change.calls_unclassified.is_empty(), "it isn't both");
    assert!(
        came.text()
            .contains("  makes a call whose target can't be known")
    );

    let neither = report(vec![making("f", vec![])], vec![edited(vec![])]);
    assert!(!only(&neither).unresolved_calls);
}

#[test]
fn the_report_leads_with_the_rating() {
    let mut config = Config::default();
    config.risk.pdg_change = Some(crate::config::Severity::Medium);
    let report = compare(
        &snapshot(vec![function("f", calling(&["a"]))]),
        &snapshot(vec![function("f", calling(&["b"]))]),
        &config,
    );

    assert_eq!(report.rating.risk, rating::Level::Medium);
    let text = report.text();
    assert!(text.starts_with("Merge risk: medium ⚠️\n"), "{text}");

    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["risk"], "medium");
    assert_eq!(json["policy_configured"], true);
    assert_eq!(json["reasons"][0]["rule"], "pdg_change");
    assert_eq!(json["reasons"][0]["function"], "f");
}

#[test]
fn what_needs_a_look_first_comes_first() {
    let mut changed = function("changed", calling(&["x"]));
    changed.path = "z.rs".into();
    let added = function("added", calling(&[]));
    let report = report(
        vec![function("changed", calling(&[]))],
        vec![changed, added],
    );
    let kinds: Vec<ChangeKind> = report.changes.iter().map(|change| change.kind).collect();
    assert_eq!(kinds, [ChangeKind::Changed, ChangeKind::Added]);
}

fn low_policy() -> Config {
    let mut config = Config::default();
    config.risk.pdg_change = Some(crate::config::Severity::Low);
    config
}

fn changed_paths(paths: &[&str]) -> Commits {
    Commits::Compared {
        prefix: String::new(),
        changed: paths
            .iter()
            .map(|path| ChangedPath {
                path: (*path).to_owned(),
                special: None,
            })
            .collect(),
    }
}

/// A saved run only holds what its frontend read, so a changed workflow is only known from the
/// commits, and it's what a gate allowing `low` must not wave through.
#[test]
fn a_changed_file_no_document_holds_keeps_a_change_from_rating_low() {
    let base = with_files(vec![], &[("src/lib.rs", "a")]);
    let commits = changed_paths(&[".github/workflows/ci.yml", "README.md"]);
    let report = super::compare(&base, &base, &low_policy(), &commits).unwrap();

    assert_eq!(report.rating.risk, rating::Level::Unknown);
    let text = report.text();
    assert!(
        text.contains("- changed files nothing here can see into: 1"),
        "{text}"
    );
    assert!(
        text.contains(".github/workflows/ci.yml: changed, and no document holds it"),
        "{text}"
    );
    assert!(
        !text.contains("README.md"),
        "documentation is for people: {text}"
    );

    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(
        json["changed_files_unread"],
        serde_json::json!([{"path": ".github/workflows/ci.yml"}])
    );
    let reason = &json["reasons"][0];
    assert_eq!(reason["rule"], "analysis_unavailable");
    assert_eq!(reason["path"], ".github/workflows/ci.yml");

    let only_docs = super::compare(&base, &base, &low_policy(), &changed_paths(&["README.md"]));
    assert_eq!(only_docs.unwrap().rating.risk, rating::Level::Low);
}

#[test]
fn a_file_a_document_holds_or_the_policy_excludes_is_not_unread() {
    let base = with_files(vec![], &[("src/lib.rs", "a")]);
    let head = with_files(vec![], &[("src/lib.rs", "b")]);
    let mut config = low_policy();
    config.exclude.0 = vec!["src/generated/**".into()];
    let commits = changed_paths(&["src/lib.rs", "src/generated/api.rs"]);
    let report = super::compare(&base, &head, &config, &commits).unwrap();

    assert_eq!(report.changed_files_unread, Some(Vec::new()));
    assert_eq!(report.changed_outside_functions, ["src/lib.rs"]);
    assert_eq!(report.changed_files_excluded, ["src/generated/api.rs"]);
    let text = report.text();
    assert!(
        text.contains("Changed files nothing here can see into: 0, and 1 the policy excludes."),
        "{text}"
    );
}

#[test]
fn commits_that_cannot_be_compared_leave_the_change_unknown_and_say_why() {
    let why = "the base run doesn't say which commit it read";
    let report = super::compare(
        &snapshot(vec![]),
        &snapshot(vec![]),
        &low_policy(),
        &Commits::Unavailable(why.into()),
    )
    .unwrap();

    assert_eq!(report.rating.risk, rating::Level::Unknown);
    assert_eq!(report.changed_files_unread, None);
    let text = report.text();
    let line = format!("- changed files can't be listed: {why}");
    assert!(text.contains(&line), "{text}");
    assert!(
        text.contains("Changed files can't be listed: the base run"),
        "{text}"
    );
    let json = serde_json::to_value(&report).unwrap();
    assert!(json["changed_files_unread"].is_null(), "{json}");
}

#[test]
fn a_policy_with_no_rules_stays_unrated_and_still_lists_what_it_cannot_see() {
    let report = super::compare(
        &snapshot(vec![]),
        &snapshot(vec![]),
        &Config::default(),
        &changed_paths(&["Cargo.lock"]),
    )
    .unwrap();
    assert_eq!(report.rating.risk, rating::Level::Unrated);
    assert!(report.text().contains("Cargo.lock: changed"));
}
