use super::*;
use crate::{ByteSpan, HirFnFacts};

fn operation(id: &str, operator: Option<&str>, operands: &[&str]) -> Operation {
    Operation {
        id: id.into(),
        category: NodeCategory::Computation,
        span: ByteSpan { start: 0, end: 0 },
        construct_span: ByteSpan { start: 0, end: 0 },
        original_kind: "expr_binary".into(),
        role: "value".into(),
        ty: "u32".into(),
        symbol: None,
        definition: None,
        operator: operator.map(str::to_owned),
        literal: None,
        construct: None,
        write_form: None,
        return_form: None,
        position: None,
        operands: operands.iter().map(|id| (*id).into()).collect(),
    }
}

fn factor(id: &str, literal: Option<&str>) -> Operation {
    Operation {
        category: if literal.is_some() {
            NodeCategory::Computation
        } else {
            NodeCategory::FormalInput
        },
        original_kind: if literal.is_some() {
            "expr_lit"
        } else {
            "param"
        }
        .into(),
        literal: literal.map(str::to_owned),
        ..operation(id, None, &[])
    }
}

fn product_graph(left: &str, right: Option<&str>) -> Graph {
    let nodes = vec![
        factor("numerator", None),
        factor("left", Some(left)),
        factor("right", right),
        operation("product", Some("*"), &["left", "right"]),
        operation("quotient", Some("/"), &["numerator", "product"]),
    ];
    let transfers = [
        "E",
        "numerator",
        "left",
        "right",
        "product",
        "quotient",
        "X",
    ]
    .windows(2)
    .map(|pair| super::super::Transfer {
        source: pair[0].into(),
        target: pair[1].into(),
        kind: "normal".into(),
        loop_role: None,
    })
    .collect();

    Graph {
        nodes,
        transfers,
        value_sites: Vec::new(),
    }
}

fn checked_facts() -> HirFnFacts {
    // Only build metadata is used; this test authors its own producer graph.
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.build.frontend_options = Default::default();
    facts.build.overflow_checks.value = Some(true);

    facts
}

#[test]
fn folded_product_proves_divisor_nonzero_only_when_known_and_in_range() {
    let facts = checked_facts();
    for (left, right, expected_product, product_safe, division_safe) in [
        ("2", Some("3"), Some("6"), true, true),
        ("0", Some("3"), Some("0"), true, false),
        ("4294967295", Some("2"), None, false, false),
        ("2", None, None, false, false),
    ] {
        let graph = product_graph(left, right);
        let analysis = super::super::analysis::analyze(&graph).unwrap();
        let assessment = assess(&facts, &graph, &analysis).unwrap();

        assert_eq!(assessment.checked_operations.len(), 2);
        let product = &assessment.checked_operations[0];
        let quotient = &assessment.checked_operations[1];
        assert_eq!(product.operation_id, "product");
        assert_eq!(product.omitted_panic_impossible, product_safe);
        assert_eq!(quotient.operation_id, "quotient");
        assert_eq!(quotient.resolved_type, "u32");
        assert_eq!(
            quotient.operand_constants,
            [None, expected_product.map(str::to_owned)]
        );
        assert_eq!(quotient.omitted_panic_impossible, division_safe);
        assert_eq!(
            assessment.limitations.len(),
            usize::from(!product_safe) + usize::from(!division_safe)
        );
    }
}
