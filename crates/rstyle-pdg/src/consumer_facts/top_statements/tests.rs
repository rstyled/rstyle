use super::super::relative_span;
use super::*;

// Build source-faithful direct nodes independently of production observation.
fn captured(source: &str) -> HirFnFacts {
    let mut facts = super::super::tests::facts(source);
    let function: syn::ItemFn = syn::parse_str(source).unwrap();
    let mut block = facts.body.clone();
    block.kind = "expr_block".into();
    block.span = relative_span(source, function.block.span());
    block.snippet = source[block.span.start as usize..block.span.end as usize].into();
    block.expansion = None;
    block.children = function
        .block
        .stmts
        .iter()
        .enumerate()
        .map(|(index, statement)| {
            let mut node = block.clone();
            node.children.clear();
            node.hir.local += index as u32 + 1;
            node.kind = match statement {
                syn::Stmt::Local(_) => "stmt_let",
                syn::Stmt::Item(_) => "stmt_item",
                syn::Stmt::Expr(_, Some(_)) | syn::Stmt::Macro(_) => "stmt_semi",
                syn::Stmt::Expr(_, None) if index + 1 == function.block.stmts.len() => "expr_block",
                syn::Stmt::Expr(_, None) => "stmt_expr",
            }
            .into();
            node.span = relative_span(source, statement.span());
            node.snippet = source[node.span.start as usize..node.span.end as usize].into();
            node
        })
        .collect();
    facts.body = block;
    facts
}

#[test]
fn direct_counts_keep_nested_work_independent_of_strict_body_support() {
    for (source, expected) in [
        ("fn empty() {}", 0),
        ("fn tail() { 42 }", 1),
        (
            "fn nested() { let x = || { first(); second(); }; { third(); fourth(); } x() }",
            3,
        ),
        (
            "fn items() { fn inner() { first(); second(); } inner(); }",
            2,
        ),
        ("fn body_macro() { let x = nested!(); consume(x); }", 2),
    ] {
        let facts = captured(source);
        let observed = super::super::inspect_with_statements(&facts).unwrap();
        assert_eq!(
            observed.top_statements,
            Availability::Known(expected),
            "{source}"
        );
        assert_eq!(observed.source, Availability::Known(source));
        assert!(matches!(observed.signature, Availability::Known(_)));
    }
}

#[test]
fn direct_expansion_empty_and_async_forms_have_no_complete_count() {
    for source in [
        "fn direct() { expand!(); }",
        "fn declaration() { macro_rules! expand { () => {} } }",
        "fn conditional() { #[cfg(unix)] let x = 1; }",
        "fn conditional() { #[cfg_attr(unix, allow(unused))] let x = 1; }",
        "fn conditional() { #[some_attribute] fn inner() {} }",
        "async fn suspended() { call().await; }",
        "fn empty_statement() { ; }",
    ] {
        let facts = captured(source);
        let observed = super::super::inspect_with_statements(&facts).unwrap();
        assert!(
            matches!(observed.top_statements, Availability::Unsupported { .. }),
            "{source}"
        );
        assert_eq!(observed.source, Availability::Known(source));
        assert!(matches!(observed.signature, Availability::Known(_)));
    }
}

#[test]
fn missing_changed_or_duplicate_original_children_remain_unknown() {
    let original = captured("fn example() { let x = 1; consume(x); }");
    for change in 0..7 {
        let mut facts = original.clone();
        match change {
            0 => facts.body.children.clear(),
            1 => facts.body.kind = "unimplemented_body".into(),
            2 => facts.body.children[0].kind = "expr_call".into(),
            3 => facts.body.children[1] = facts.body.children[0].clone(),
            4 => facts.body.children[0].span.end -= 1,
            5 => facts.body.children[0].snippet = "different".into(),
            _ => facts.body.snippet = "{}".into(),
        }
        assert!(matches!(
            super::super::inspect_with_statements(&facts)
                .unwrap()
                .top_statements,
            Availability::Unknown { .. }
        ));
    }
}

#[test]
fn absent_original_ownership_cannot_supply_statement_count() {
    let mut facts = captured("fn empty() {}");
    facts.build.frontend_options = Default::default();
    assert!(matches!(
        super::super::inspect_with_statements(&facts)
            .unwrap()
            .top_statements,
        Availability::Unknown { .. }
    ));
}

#[test]
fn indexed_spans_preserve_utf8_and_crlf_original_positions() {
    let source = "fn café() {\r\n let π = 1;\r\n consume(π);\r\n π\r\n}";
    let facts = captured(source);
    let function: syn::ItemFn = syn::parse_str(source).unwrap();
    let positions = Positions::new(source);
    assert_eq!(
        span(&positions, function.block.span()),
        relative_span(source, function.block.span())
    );
    for statement in &function.block.stmts {
        assert_eq!(
            span(&positions, statement.span()),
            relative_span(source, statement.span())
        );
    }
    assert_eq!(
        super::super::inspect_with_statements(&facts)
            .unwrap()
            .top_statements,
        Availability::Known(3)
    );
}

#[test]
fn many_direct_statements_preserve_every_original_roster_position() {
    let mut source = String::from("fn many() {\n");
    for index in 0..1000 {
        source.push_str(&format!(" let value_{index} = {index};\n"));
    }
    source.push_str(" value_999\n}");
    let mut facts = captured(&source);
    assert_eq!(
        super::super::inspect_with_statements(&facts)
            .unwrap()
            .top_statements,
        Availability::Known(1001)
    );
    facts.body.children.swap(998, 999);
    assert!(matches!(
        super::super::inspect_with_statements(&facts)
            .unwrap()
            .top_statements,
        Availability::Unknown { .. }
    ));
}

#[test]
fn ordinary_inspection_leaves_statement_work_unrequested() {
    for source in ["fn empty() {}", "fn macro_body() { unknown!(); }"] {
        let facts = captured(source);
        let ordinary = super::super::inspect(&facts).unwrap();
        assert_eq!(
            ordinary.top_statements,
            Availability::NotApplicable {
                reason: "Statement count was not requested".into(),
            }
        );
        assert_eq!(ordinary.source, Availability::Known(source));
        assert!(matches!(ordinary.signature, Availability::Known(_)));
    }
}
