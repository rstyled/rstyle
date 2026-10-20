//! Source-selected contracts compose with bounded Option graphs.
use super::*;
use rstyle_pdg::{HirFnFacts, HirNode, checked_options};
use serde_json::json;

fn node<'a>(facts: &'a HirFnFacts, kind: &str) -> &'a HirNode {
    find(&facts.body, kind).unwrap()
}

fn find<'a>(node: &'a HirNode, kind: &str) -> Option<&'a HirNode> {
    if node.kind == kind {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, kind))
}

#[test]
fn checked_access_contracts_verify_installed_ci_targets() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let output = std::process::Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(output.status.success());
        if !std::path::Path::new(String::from_utf8(output.stdout).unwrap().trim()).is_dir() {
            eprintln!("target std not installed: {target}");
            continue;
        }
        for no_std in [false, true] {
            target_contracts(target, no_std);
        }
    }
}

fn target_contracts(target: &str, no_std: bool) {
    let directory = workspace();
    if no_std {
        let path = directory.path().join("src/lib.rs");
        let source = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, format!("#![no_std]\n{source}")).unwrap();
    }
    let (output, report) = clones(
        directory.path(),
        &["--target", target, "--extraction-profile", "hir-data-v3"],
    );
    assert!(output.status.success(), "{target}: {report}");
    check_aggregate_documents(&report);
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    for function in report["functions"].as_array().unwrap() {
        let facts: HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
        if matches!(facts.name.as_str(), "byte" | "alias") {
            assert_eq!(
                function["outcome"],
                if facts.name == "byte" {
                    "admitted"
                } else {
                    "excluded"
                }
            );
            let matched = node(&facts, "expr_match");
            let checked = checked_options::verify_match(matched, &facts).unwrap();
            assert_eq!(checked.some_arm, 0);
            assert_eq!(checked.none_arm, 1);
            assert_eq!(
                checked.binding.ty.as_ref().unwrap().normalized.as_deref(),
                Some("u8")
            );
            reject_pattern_corruption(matched, &facts);
            if facts.name == "byte" {
                get_contract(&facts);
            }
        } else if matches!(
            facts.name.as_str(),
            "mutable" | "nested" | "lookalike::read"
        ) {
            assert!(checked_options::verify_match(node(&facts, "expr_match"), &facts).is_err());
        }
    }
}

fn get_contract(facts: &HirFnFacts) {
    let get = node(facts, "expr_method_call");
    let checked = checked_options::verify_get(get, facts).unwrap();
    assert_eq!(checked.scalar, "u8");
    assert_eq!(checked.receiver.snippet, "bytes");
    assert_eq!(checked.index.snippet, "index");
    for field in ["crate_hash", "definition_hash", "path"] {
        let mut forged = get.clone();
        let resolution = forged.res.as_mut().unwrap();
        let mut callee: Value = serde_json::from_str(&resolution.description).unwrap();
        callee[field] = json!("wrong");
        resolution.description = serde_json::to_string(&callee).unwrap();
        assert!(
            checked_options::verify_get(&forged, facts).is_err(),
            "{field}"
        );
    }
    let mut forged = get.clone();
    forged.children.swap(0, 1);
    assert!(checked_options::verify_get(&forged, facts).is_err());
    let mut forged = get.clone();
    forged.dispatch.as_mut().unwrap().operand_tys[1] = "u8".into();
    assert!(checked_options::verify_get(&forged, facts).is_err());
    let mut forged = facts.clone();
    forged.build.target_triple = rstyle_pdg::StringFact::known("unregistered");
    assert!(checked_options::verify_get(get, &forged).is_err());
}

fn reject_pattern_corruption(matched: &HirNode, facts: &HirFnFacts) {
    let mut forged = matched.clone();
    forged.children[2].children[0] = forged.children[1].children[0].clone();
    assert!(checked_options::verify_match(&forged, facts).is_err());
    let mut forged = matched.clone();
    let resolution = forged.children[1].children[0].res.as_mut().unwrap();
    let mut callee: Value = serde_json::from_str(&resolution.description).unwrap();
    callee["definition_hash"] = json!("wrong");
    resolution.description = serde_json::to_string(&callee).unwrap();
    assert!(checked_options::verify_match(&forged, facts).is_err());
    let mut forged = matched.clone();
    forged.children[1].children[0].children[0].children[0].binding = Some("forged".into());
    assert!(checked_options::verify_match(&forged, facts).is_err());
    let mut forged = facts.clone();
    forged.bindings.retain(|binding| binding.is_param);
    assert!(checked_options::verify_match(matched, &forged).is_err());
}

#[test]
fn checked_access_contracts_bound_source_shapes_and_preserve_saved_operands() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), boundary_source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["functions"].as_array().unwrap().len(), 7);
    for function in report["functions"].as_array().unwrap() {
        let facts: HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
        assert_eq!(
            function["outcome"],
            if matches!(facts.name.as_str(), "reversed" | "saved") {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
        let matched = checked_options::verify_match(node(&facts, "expr_match"), &facts);
        let get = checked_options::verify_get(node(&facts, "expr_method_call"), &facts);
        assert_eq!(
            matched.is_ok(),
            matches!(
                facts.name.as_str(),
                "reversed" | "saved" | "array" | "explicit"
            )
        );
        assert_eq!(
            get.is_ok(),
            matches!(
                facts.name.as_str(),
                "reversed" | "saved" | "guarded" | "borrowed"
            )
        );
        if facts.name == "reversed" {
            let checked = matched.unwrap();
            assert_eq!((checked.some_arm, checked.none_arm), (1, 0));
        } else if facts.name == "saved" {
            let checked = get.unwrap();
            assert_eq!(checked.receiver.snippet, "selected");
            assert_eq!(checked.index.snippet, "{ selected = second; index }");
        }
    }
}

fn boundary_source() -> &'static str {
    r#"
    pub fn reversed(bytes: &[u8], index: usize) -> bool {
        match bytes.get(index) { None => index == bytes.len(), Some(&b) => b <= 127 || b >= 192 }
    }
    pub fn saved(first: &[u8], second: &[u8], index: usize) -> u8 {
        let mut selected = first;
        let saved = selected.get({ selected = second; index });
        match saved { Some(&byte) => byte, None => selected.len() as u8 }
    }
    pub fn guarded(bytes: &[u8], index: usize) -> u8 {
        match bytes.get(index) { Some(&byte) if byte > 0 => byte, _ => 0 }
    }
    pub fn borrowed(bytes: &[u8], index: usize) -> u8 {
        match bytes.get(index) { Some(byte) => *byte, None => 0 }
    }
    pub fn array(bytes: &[u8; 4], index: usize) -> u8 {
        match bytes.get(index) { Some(&byte) => byte, None => 0 }
    }
    pub fn explicit(bytes: &[u8], index: usize) -> u8 {
        match bytes.get::<usize>(index) { Some(&byte) => byte, None => 0 }
    }
    pub fn ranged(bytes: &[u8], index: usize) -> usize {
        match bytes.get(0..index) { Some(value) => value.len(), None => 0 }
    }
    "#
}
