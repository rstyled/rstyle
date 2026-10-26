//! Fixed ordinary declaration and adversaries against source/component identity.
use super::*;
use crate::{ByteSpan, HirFnFacts, ResolvedTy};

fn node(facts: &HirFnFacts, kind: &str, text: &str, token: Option<&str>) -> HirNode {
    let start = facts.source.bytes.find(text).unwrap() as u32;
    let mut node = facts.body.clone();
    node.hir.local = start;
    node.kind = kind.into();
    node.span = ByteSpan {
        start,
        end: start + text.len() as u32,
    };
    node.snippet = text.into();
    node.ty = token.map(|token| ResolvedTy {
        normalized: Some(token.into()),
        observed: token.into(),
        spelling: String::new(),
    });
    node.children.clear();
    node.res = None;
    node.dispatch = None;
    node.expansion = None;
    node.binding = None;
    node.binding_mut = None;
    node.param_index = None;
    node.lit = None;
    node.is_unsafe_block = false;
    node
}

pub(super) fn fixture() -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.build.frontend_options.insert(
        "extraction_profile".into(),
        serde_json::json!("hir-data-v3"),
    );
    facts.source.bytes = "let (number, consumed) = pair;".into();
    let tuple = crate::data_types::token(&crate::concrete_types::TypeTree::Tuple(vec![
        crate::concrete_types::TypeTree::Scalar("u32".into()),
        crate::concrete_types::TypeTree::Scalar("usize".into()),
    ]))
    .unwrap();
    let mut pattern = node(
        &facts,
        "pat_scalar_tuple",
        "(number, consumed)",
        Some(&tuple),
    );
    facts.bindings = [("number", "u32"), ("consumed", "usize")]
        .into_iter()
        .enumerate()
        .map(|(index, (name, token))| {
            let mut pat = node(&facts, "pat_binding", name, Some(token));
            let symbol = format!("binding:{index}");
            pat.binding = Some(symbol.clone());
            pat.binding_mut = Some(false);
            let binding = BindingFact {
                symbol,
                name: name.into(),
                decl_span: pat.span,
                scope: "local".into(),
                hir: pat.hir.clone(),
                is_param: false,
                is_mut: false,
                ty: pat.ty.clone().unwrap(),
            };
            pattern.children.push(pat);
            binding
        })
        .collect();
    let value = node(&facts, "expr_path", "pair", Some(&tuple));
    let mut statement = node(&facts, "stmt_let", "let (number, consumed) = pair;", None);
    statement.children = vec![pattern, value];
    facts.body = statement;
    facts
}

#[test]
fn exact_flat_scalar_slots_keep_original_binding_order() {
    let facts = fixture();
    let checked = verify(&facts.body, &facts).unwrap();
    assert_eq!(
        checked
            .entries
            .iter()
            .map(|entry| (entry.position, entry.token))
            .collect::<Vec<_>>(),
        [(0, "u32"), (1, "usize")]
    );
    assert_eq!(checked.initializer.snippet, "pair");
}

#[test]
fn changed_binding_roster_source_and_component_types_reject() {
    for mutation in [
        "order",
        "mutable",
        "symbol",
        "type",
        "duplicate",
        "source",
        "initializer",
    ] {
        let mut facts = fixture();
        match mutation {
            "order" => facts.body.children[0].children.swap(0, 1),
            "mutable" => facts.bindings[0].is_mut = true,
            "symbol" => facts.body.children[0].children[0].binding = Some("invented".into()),
            "type" => {
                facts.bindings[0].ty.normalized = Some("u16".into());
                facts.body.children[0].children[0]
                    .ty
                    .as_mut()
                    .unwrap()
                    .normalized = Some("u16".into());
            }
            "duplicate" => facts.bindings.push(facts.bindings[0].clone()),
            "source" => facts.body.children[0].snippet.push(' '),
            "initializer" => facts.body.children.push(facts.body.children[1].clone()),
            _ => unreachable!(),
        }
        assert!(verify(&facts.body, &facts).is_err(), "{mutation}");
    }
}
