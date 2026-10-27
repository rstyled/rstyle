//! Complete mutable memory admission retains source and resealed proof attacks.
mod matching;
mod memory_checks;
mod mutations;
use crate::*;
use rstyle_pdg::field_locations::{self, FieldObservations, scalar_formals};

const CONTROLS: &str = r#"
fn alias(cur: &mut usize) { let saved = cur; *saved -= 1; }
fn escape(cur: &mut usize) { consume(cur); }
fn closure(cur: &mut usize) -> usize { let read = || *cur; read() }
fn consume(_: &mut usize) {}
fn helper(cur: &mut usize) { *cur -= 1; }
fn noop(_cur: &mut usize) {}
fn caller() -> usize { let mut value = 2; helper(&mut value); value }
fn noop_caller() -> usize { let mut value = 2; noop(&mut value); value }
fn rebind<'a>(mut cur: &'a mut usize, other: &'a mut usize) { cur = other; *cur = 1; }
fn other_type(cur: &mut u32) { *cur -= 1; }
fn mutable_slice(cur: &mut [u8]) { cur[0] = 1; }
fn readonly(cur: &usize) -> usize { *cur }
fn unrelated() -> usize { 3 }
"#;

#[test]
fn mutable_scalar_formals_verify_complete_original_memory_and_reject_corruptions() {
    let selection: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../eval/coverage-development/backtrack-match/selection.json"
    )))
    .unwrap();
    let source = selection["function"]["source"].as_str().unwrap();
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{source}\n{CONTROLS}"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "backtrack_match")
        .unwrap();
    assert_eq!(function["facts"]["source"]["bytes"], source);
    assert_eq!(function["outcome"], "admitted", "{}", function["reasons"]);
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    check_original_access_inventory(&facts);
    check_access_corruptions(&facts);
    check_missing_and_contradictory_evidence(&facts);
    check_unsupported_controls(&report);
    check_helper_boundary(&report);
    check_verified_pair(function);
}

fn check_unsupported_controls(report: &Value) {
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let fields = facts
            .build
            .frontend_options
            .data::<FieldObservations>()
            .unwrap();
        match name {
            "alias" | "escape" | "rebind" | "closure" => {
                let evidence = fields.scalar_formal_accesses.as_ref().unwrap();
                assert_ne!(
                    evidence.status,
                    scalar_formals::InventoryStatus::Complete,
                    "{name}"
                );
                assert!(scalar_formals::verify(&facts, evidence).is_err(), "{name}");
                assert_eq!(function["outcome"], "excluded", "{name}");
            }
            "other_type" | "mutable_slice" => {
                if let Some(evidence) = fields.scalar_formal_accesses.as_ref() {
                    assert_ne!(
                        evidence.status,
                        scalar_formals::InventoryStatus::Complete,
                        "{name}"
                    );
                }
                assert_eq!(function["outcome"], "excluded", "{name}");
            }
            "readonly" | "unrelated" => {
                assert!(fields.scalar_formal_accesses.is_none(), "{name}");
                assert_eq!(function["outcome"], "admitted", "{name}");
            }
            _ => {}
        }
    }
}

fn check_helper_boundary(report: &Value) {
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        match name {
            "helper" | "noop" => {
                assert_eq!(function["outcome"], "admitted", "{name}");
                let wire = shared_evidence::wire(function);
                assert!(
                    wire["certified"][shared_evidence::index(function)].is_null(),
                    "{name} exported a read-only helper summary"
                );
            }
            "caller" | "noop_caller" => {
                assert_eq!(function["outcome"], "excluded", "{name}");
                assert!(
                    function["reasons"].to_string().contains("helper"),
                    "{name}: {}",
                    function["reasons"]
                );
            }
            _ => {}
        }
    }
}

fn check_original_access_inventory(facts: &rstyle_pdg::HirFnFacts) {
    let fields = facts
        .build
        .frontend_options
        .data::<FieldObservations>()
        .unwrap();
    let evidence = fields.scalar_formal_accesses.as_ref().unwrap();
    assert_eq!(evidence.status, scalar_formals::InventoryStatus::Complete);
    assert_eq!(evidence.formals.len(), 5);
    assert_eq!(evidence.accesses.len(), 8);
    check_access_order(evidence);
    assert_eq!(fields.possibly_aliasing_formals, [[0, 0], [0, 3], [3, 3]]);
    assert_eq!(
        evidence.possibly_aliasing_formals,
        [
            [0, 0],
            [0, 1],
            [0, 3],
            [0, 4],
            [1, 1],
            [1, 3],
            [1, 4],
            [3, 3],
            [3, 4],
            [4, 4],
        ]
    );
    field_locations::verify(facts, &fields).unwrap();

    let observation = facts
        .build
        .frontend_options
        .data::<rstyle_pdg::concrete_types::InterfaceObservations>()
        .unwrap();
    for index in [1, 4] {
        assert!(matches!(
            observation.parameters[index],
            rstyle_pdg::concrete_types::Observation::Captured {
                ref ty
            } if *ty == rstyle_pdg::concrete_types::TypeTree::MutableScalar("usize".into())
        ));
        assert_eq!(
            facts.signature.params[index].ty.normalized.as_deref(),
            Some(r#"{"kind":"mutable_scalar","value":"usize"}"#)
        );
    }
}

fn check_access_order(evidence: &scalar_formals::ScalarFormalAccesses) {
    use scalar_formals::AccessKind::{Load, Store};
    assert_eq!(
        evidence
            .accesses
            .iter()
            .map(|access| access.parameter)
            .collect::<Vec<_>>(),
        [4, 1, 1, 4, 1, 1, 4, 4]
    );
    assert_eq!(
        evidence
            .accesses
            .iter()
            .map(|access| access.kind)
            .collect::<Vec<_>>(),
        [Load, Load, Load, Load, Load, Store, Load, Store]
    );
    for access in &evidence.accesses[..4] {
        assert!(access.compound.is_none());
    }
    for access in &evidence.accesses[4..] {
        assert_eq!(access.compound.as_ref().unwrap().operator, "-=");
    }
}

fn check_access_corruptions(facts: &rstyle_pdg::HirFnFacts) {
    let fields = facts
        .build
        .frontend_options
        .data::<FieldObservations>()
        .unwrap();
    let original = fields.scalar_formal_accesses.as_ref().unwrap();
    let mut changed = original.clone();
    changed.possibly_aliasing_formals.remove(1);
    assert!(scalar_formals::verify(facts, &changed).is_err());
    let mut changed = original.clone();
    changed.accesses.pop();
    assert!(scalar_formals::verify(facts, &changed).is_err());
    let mut changed = original.clone();
    changed.accesses.swap(6, 7);
    assert!(scalar_formals::verify(facts, &changed).is_err());
    let mut changed = original.clone();
    changed.accesses.swap(5, 6);
    assert!(scalar_formals::verify(facts, &changed).is_err());
    let mut changed = original.clone();
    changed.formals.swap(1, 4);
    assert!(scalar_formals::verify(facts, &changed).is_err());
}

fn check_missing_and_contradictory_evidence(facts: &rstyle_pdg::HirFnFacts) {
    let fields = facts
        .build
        .frontend_options
        .data::<FieldObservations>()
        .unwrap();
    let original = fields.scalar_formal_accesses.as_ref().unwrap();
    let mut changed = facts.clone();
    remove_first_deref_dispatch(&mut changed.body);
    let missing = scalar_formals::observe_accesses(&changed, original.formals.clone());
    assert!(matches!(
        missing.status,
        scalar_formals::InventoryStatus::Unknown { .. }
    ));
    assert!(scalar_formals::verify(&changed, original).is_err());
    let mut changed = facts.clone();
    changed.body.children[0].hir = changed.body.hir.clone();
    let merged = scalar_formals::observe_accesses(&changed, original.formals.clone());
    assert!(matches!(
        merged.status,
        scalar_formals::InventoryStatus::Unknown { .. }
    ));
    assert!(
        scalar_formals::verify(&changed, original)
            .unwrap_err()
            .contains("repeated identities")
    );
    let mut changed = facts.clone();
    let parameter = &mut changed.signature.params[1];
    parameter.ty.observed = "&usize".into();
    changed
        .bindings
        .iter_mut()
        .find(|binding| binding.hir == parameter.pat.hir)
        .unwrap()
        .ty
        .observed = "&usize".into();
    assert!(scalar_formals::verify(&changed, original).is_err());
    let over_budget =
        scalar_formals::observe_accesses(facts, vec![original.formals[1].clone(); 65]);
    assert!(matches!(
        over_budget.status,
        scalar_formals::InventoryStatus::Unknown { .. }
    ));
    assert!(over_budget.possibly_aliasing_formals.is_empty());
}

fn remove_first_deref_dispatch(node: &mut rstyle_pdg::HirNode) -> bool {
    if node.kind == "expr_unary" {
        node.dispatch = None;
        return true;
    }
    node.children.iter_mut().any(remove_first_deref_dispatch)
}

fn check_verified_pair(function: &Value) {
    let (store, index) = data_unit(function);
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = shared_evidence::pair(&store, function);
    assert_eq!(facts.build.overflow_checks.value, Some(true));
    assert_eq!(pair.pdg.profile.version, "3.128.0");
    rstyle_pdg::verification_policy::strict(|| {
        rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts)
    })
    .unwrap();
    rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
    memory_checks::check(pair);
    mutations::check(pair, &facts);
}
