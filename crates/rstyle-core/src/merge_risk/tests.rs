use super::*;
use crate::cpg::records::DeclaredType;
use crate::facts::{Edge, EdgeKind, EffectEvent, EffectKind, PdgEdge, Receiver, sample};
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
    base.not_compared.insert("generated".into());
    let report = compare(
        &base,
        &snapshot(vec![function("f", calling(&[]))]),
        &Config::default(),
    );
    let text = report.text();
    assert!(text.contains("1 added"), "{text}");
    assert!(
        text.contains("Generated functions not compared: 1."),
        "{text}"
    );
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
