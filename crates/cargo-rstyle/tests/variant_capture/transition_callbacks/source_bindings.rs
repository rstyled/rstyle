//! Coordinated evidence edits must fail original binding proof before graph checks.
use super::*;
use rstyle_pdg::{HirFnFacts, HirNode, unit_store::EvidenceFacts};

const BINDING_SOURCE: &str = r#"
#[derive(Clone, Copy)]
pub struct Transition { start: u8, end: u8 }
impl Transition {
    pub fn matches_byte(&self, byte: u8) -> bool {
        self.start <= byte && byte <= self.end
    }
    pub fn gate(&self, other: &Self, a: &[u8], at: usize) -> bool {
        a.get(at).map_or(false, |&b| self.matches_byte(b))
    }
    pub fn alternate(&self, other: &Self, a: &[u8], at: usize) -> bool {
        a.get(at).map_or(false, |&b| other.matches_byte(b))
    }
    pub fn raw_identifiers(&self, r#match: &Self, a: &[u8], at: usize) -> bool {
        a.get(at).map_or(false, |&r#type| r#match.matches_byte(r#type))
    }
    pub fn fixed(&self, fixed: u8, a: &[u8], at: usize) -> bool {
        a.get(at).map_or(false, |&b| self.matches_byte(fixed))
    }
    pub fn shadowed(&self, other: &Self, a: &[u8], at: usize) -> bool {
        { let other = self; a.get(at).map_or(false, |&b| other.matches_byte(b)) }
    }
}
"#;

fn binding_capture() -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), BINDING_SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

fn owned(unit: &rstyle_pdg::unit_store::UnitStore, entry: &Value) -> HirFnFacts {
    let analysis = unit.analysis();
    analysis
        .function(shared_evidence::index(entry))
        .unwrap()
        .facts()
        .clone()
}

fn closure(node: &HirNode) -> &HirNode {
    if node.kind == "expr_closure" {
        return node;
    }
    node.children.iter().find_map(find_closure).unwrap()
}

fn find_closure(node: &HirNode) -> Option<&HirNode> {
    if node.kind == "expr_closure" {
        return Some(node);
    }
    node.children.iter().find_map(find_closure)
}

fn closure_mut(node: &mut HirNode) -> &mut HirNode {
    if node.kind == "expr_closure" {
        return node;
    }
    node.children.iter_mut().find_map(find_closure_mut).unwrap()
}

fn find_closure_mut(node: &mut HirNode) -> Option<&mut HirNode> {
    if node.kind == "expr_closure" {
        return Some(node);
    }
    node.children.iter_mut().find_map(find_closure_mut)
}

fn payload_binding(closure: &HirNode) -> &HirNode {
    let pattern = &closure.children[0];
    assert_eq!(pattern.kind, "pat_shared_copy");
    &pattern.children[0]
}

fn environment(closure: &HirNode) -> rstyle_pdg::option_callbacks::Environment {
    serde_json::from_str(&closure.res.as_ref().unwrap().description).unwrap()
}

fn coordinated_receiver(facts: &mut HirFnFacts, rename: bool) {
    let other = facts.signature.params[1].pat.hir.clone();
    let callback = closure_mut(&mut facts.body);
    let mut captured = environment(callback);
    captured.captures[0].root = other.clone();
    callback.children[1].children[0]
        .res
        .as_mut()
        .unwrap()
        .target = Some(other.clone());
    callback.res.as_mut().unwrap().description = serde_json::to_string(&captured).unwrap();

    if rename {
        facts
            .bindings
            .iter_mut()
            .find(|binding| binding.hir == other)
            .unwrap()
            .name = "self".into();
    }
}

fn forge_environment(facts: &mut HirFnFacts, template: &HirFnFacts, parameter: usize) {
    let root = facts.signature.params[parameter].pat.hir.clone();
    let callback = closure_mut(&mut facts.body);
    let mut captured = environment(closure(&template.body));
    captured.captures[0].root = root.clone();
    let exact = captured.shared_formal.as_mut().unwrap();
    exact.closure = callback.hir.clone();
    exact.uses[0].hir = callback.children[1].children[0].hir.clone();
    callback.children[1].children[0]
        .res
        .as_mut()
        .unwrap()
        .target = Some(root);
    callback.res.as_mut().unwrap().description = serde_json::to_string(&captured).unwrap();
}

fn coordinated_payload(facts: &mut HirFnFacts, template: &HirFnFacts, rename: bool) {
    forge_environment(facts, template, 0);
    let callback = closure_mut(&mut facts.body);
    let binding = payload_binding(callback).hir.clone();
    callback.children[1].children[1]
        .res
        .as_mut()
        .unwrap()
        .target = Some(binding.clone());

    if rename {
        facts
            .bindings
            .iter_mut()
            .find(|record| record.hir == binding)
            .unwrap()
            .name = "fixed".into();
    }
}

fn reject_source_binding(facts: &HirFnFacts, expected: &str) {
    // classify does not consume a graph: rejection cannot come from comparing
    // edited frontend evidence against an unchanged serialized graph.
    let verdict = rstyle_pdg::eligibility::classify(facts);
    assert!(
        verdict
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.reason.contains(expected)),
        "expected {expected}; got {:?}",
        verdict.diagnostics
    );
}

#[test]
fn callback_source_binding_rejects_coordinated_same_type_roots_and_payload_targets() {
    let (_directory, report) = binding_capture();
    for name in [
        "Transition::gate",
        "Transition::alternate",
        "Transition::raw_identifiers",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    let unit = super::super::word_classifiers::verify_admissions(&report);
    let gate = owned(&unit, function(&report, "Transition::gate"));

    reject_receiver_substitutions(&gate);
    let fixed = owned(&unit, function(&report, "Transition::fixed"));
    reject_payload_substitutions(&fixed, &gate);
    let shadowed = owned(&unit, function(&report, "Transition::shadowed"));
    let mut changed = shadowed.clone();
    forge_environment(&mut changed, &gate, 1);
    assert_eq!(changed.source, shadowed.source);
    reject_source_binding(&changed, "Callback source binding");
}

fn reject_receiver_substitutions(gate: &HirFnFacts) {
    for rename in [false, true] {
        let mut changed = gate.clone();
        coordinated_receiver(&mut changed, rename);
        assert_eq!(changed.source, gate.source);
        assert_eq!(
            closure(&changed.body).children[1].children[0].snippet,
            "self"
        );
        reject_source_binding(
            &changed,
            "Callback source binding receiver disagrees with its original formal",
        );
    }
}

fn reject_payload_substitutions(fixed: &HirFnFacts, gate: &HirFnFacts) {
    for rename in [false, true] {
        let mut changed = fixed.clone();
        coordinated_payload(&mut changed, gate, rename);
        assert_eq!(changed.source, fixed.source);
        assert_eq!(
            closure(&changed.body).children[1].children[1].snippet,
            "fixed"
        );
        reject_source_binding(
            &changed,
            "Callback source binding argument differs from its original payload parameter",
        );
    }
}
