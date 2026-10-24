use super::*;
use syn::visit::Visit;
mod fixture;
use fixture::Fixture;

fn arguments(bytes: &str) -> Result<Vec<syn::Expr>, syn::Error> {
    syn::parse_str::<arguments::Arguments>(bytes).map(|args| args.0)
}

#[derive(Default)]
struct DropItems(usize);

impl<'ast> Visit<'ast> for DropItems {
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if item
            .trait_
            .as_ref()
            .is_some_and(|(_, path, _)| path.is_ident("Drop"))
        {
            self.0 += 1;
        }
        syn::visit::visit_item_impl(self, item);
    }
}

#[test]
fn hidden_drop_items_survive_in_every_value_and_option_expression() {
    let hidden = "{ struct Hidden; impl Drop for Hidden { fn drop(&mut self) {} } 1 }";
    for bytes in [
        format!("{hidden}, 2"),
        format!("1, {hidden}"),
        format!("1 => width({hidden})"),
        format!("1 => fill('0') width({hidden}), 2"),
        format!("1 => width(2), 3 => precision({hidden})"),
    ] {
        let values = arguments(&bytes).unwrap();
        let mut visitor = DropItems::default();
        for value in values {
            visitor.visit_expr(&value);
        }
        assert_eq!(
            visitor.0, 1,
            "Ownership traversal lost an actual argument: {bytes}"
        );
    }
}

#[test]
fn original_template_requires_all_rules_cfgs_and_original_export() {
    assert!(source::template(source::MODULE, &[]).is_ok());
    assert!(source::template(source::MODULE, &["doc".into()]).is_err());
    for changed in [
        source::MODULE.replacen(".with_width($e)", ".with_alternate($e)", 1),
        source::MODULE.replacen("#[cfg(not(doc))]", "#[cfg(doc)]", 1),
        source::MODULE.replace("as padded_width_of;", "as another_macro;"),
        source::MODULE.replace("(@inner []", "(@changed []"),
    ] {
        assert_ne!(changed, source::MODULE);
        assert!(source::template(&changed, &[]).is_err());
    }
}

#[test]
fn reexport_matches_original_rustc_coordinates_after_independent_cfg_check() {
    let mut fixture = Fixture::original();
    let site = &fixture.namespaces.declarations[0].site;
    assert_eq!(
        site.declaration.span,
        crate::ByteSpan {
            start: 8241,
            end: 8306
        }
    );
    assert_eq!(
        &fixture.census.files[1].bytes[8241..8306],
        "pub use __not_public_at_root__padded_width_of as padded_width_of;",
    );
    assert!(verify(fixture.input()).is_ok());
    fixture.namespaces.declarations[0]
        .site
        .declaration
        .span
        .start = 8224;
    assert!(verify(fixture.input()).is_err());
    fixture.namespaces.declarations[0]
        .site
        .declaration
        .span
        .start = 8241;
    fixture.cfgs.push("doc".into());
    assert!(
        verify(fixture.input())
            .err()
            .unwrap()
            .contains("reexport is inactive")
    );
}

#[test]
fn public_argument_grammar_exhausts_values_options_and_trailing_tokens() {
    assert!(arguments("").unwrap().is_empty());
    assert_eq!(
        arguments("a, b => width(2) fill('0'), c,").unwrap().len(),
        5
    );
    assert_eq!(arguments("a => alternate(true) sign_plus(false) sign_minus(false) align(x) precision(2) sign_aware_zero_pad(true)").unwrap().len(), 7);
    for malformed in [
        "@inner [] [0]",
        "@options width(2)",
        "a =>",
        "a => unknown(2)",
        "a => width()",
        "a => width(1, 2)",
        "a => width(2) junk",
        "a b",
        "a,,",
        "a => width(2);",
        "a => width(2) => fill('0')",
    ] {
        assert!(
            arguments(malformed).is_err(),
            "Accepted unconsumed source tokens: {malformed}"
        );
    }
}

#[test]
fn original_source_receipt_preserves_every_expression_through_checked_joins() {
    let fixture = Fixture::original();
    let receipt = verify(fixture.input()).unwrap();
    assert!(Arc::ptr_eq(receipt.census(), &fixture.census));
    assert_eq!(receipt.definition(), &fixture.export.target);
    let hidden = "{ struct Hidden; impl Drop for Hidden { fn drop(&mut self) {} } 1 }";
    let invocation: syn::ExprMacro = syn::parse_str(&format!(
        "smart_display::padded_width_of!({hidden}, 2 => width({hidden}) fill('0'), 3)"
    ))
    .unwrap();
    let expressions = receipt
        .expressions(&invocation.mac, &fixture.export.target)
        .unwrap();
    assert_eq!(expressions.len(), 5);
    let mut visitor = DropItems::default();
    for expression in expressions {
        visitor.visit_expr(&expression);
    }
    assert_eq!(visitor.0, 2);
}

#[test]
fn receipt_rejects_changed_original_source_and_namespace_authority_joins() {
    let attacks: &[fn(&mut Fixture)] = &[
        |fixture| {
            Arc::make_mut(&mut fixture.census).files[1].digest = "xxh3-64:0000000000000000".into()
        },
        |fixture| fixture.export.target.path = "powerfmt::another_macro".into(),
        |fixture| fixture.export.target.stable_crate_id = "different compiled owner".into(),
        |fixture| fixture.export.target.definition_identity = "different definition".into(),
        |fixture| fixture.definition.span.start += 1,
        |fixture| fixture.cfgs.push("doc".into()),
    ];
    reject_attacks(attacks);
}

#[test]
fn receipt_rejects_changed_reexport_ownership_and_original_coordinates() {
    let attacks: &[fn(&mut Fixture)] = &[
        |fixture| fixture.namespaces.declarations[0].site.declaration.span.end -= 1,
        |fixture| {
            fixture.namespaces.declarations[0]
                .site
                .target
                .stable_crate_id = "another producer".into()
        },
        |fixture| {
            fixture.namespaces.declarations[0]
                .site
                .definition
                .definition_identity = "another reexport".into()
        },
        |fixture| fixture.namespaces.declarations.clear(),
        |fixture| fixture.export.route.clear(),
        |fixture| fixture.definition.digest = "xxh3-64:0000000000000000".into(),
        |fixture| fixture.namespaces.declarations[0].site.leaf.span.start += 1,
        |fixture| {
            fixture.namespaces.declarations[0]
                .site
                .owner
                .stable_crate_id = "wrong owner".into()
        },
        |fixture| fixture.namespaces.declarations[0].site.target_kind = "Struct".into(),
        |fixture| {
            fixture.export.route[0].declaration.stable_crate_id = "different route owner".into();
            fixture.namespaces.declarations[0]
                .site
                .definition
                .stable_crate_id = "different route owner".into();
        },
    ];
    reject_attacks(attacks);
}

fn reject_attacks(attacks: &[fn(&mut Fixture)]) {
    for (index, attack) in attacks.iter().enumerate() {
        let mut fixture = Fixture::original();
        attack(&mut fixture);
        assert!(
            verify(fixture.input()).is_err(),
            "Accepted source/namespace attack {index}"
        );
    }
}

#[test]
fn receipt_rejects_other_resolved_definitions_and_malformed_actual_invocations() {
    let fixture = Fixture::original();
    let receipt = verify(fixture.input()).unwrap();
    let invocation: syn::ExprMacro = syn::parse_str("smart_display::padded_width_of!(1)").unwrap();
    let attacks: &[fn(&mut Definition)] = &[
        |definition| definition.stable_crate_id = "different artifact".into(),
        |definition| definition.definition_identity = "different definition".into(),
        |definition| definition.path = "powerfmt::shadow".into(),
        |definition| definition.local = true,
    ];
    for attack in attacks {
        let mut resolved = fixture.export.target.clone();
        attack(&mut resolved);
        assert!(receipt.expressions(&invocation.mac, &resolved).is_err());
    }
    for tokens in ["@inner [] [0]", "1 => unknown(2)", "1 => width(2);", "1,,"] {
        let invocation: syn::ExprMacro =
            syn::parse_str(&format!("padded_width_of!({tokens})")).unwrap();
        assert!(
            receipt
                .expressions(&invocation.mac, &fixture.export.target)
                .is_err()
        );
    }
}

fn rewrite_module(fixture: &mut Fixture, from: &str, to: &str) {
    let census = Arc::make_mut(&mut fixture.census);
    let file = &mut census.files[1];
    let rewritten = file.bytes.replacen(from, to, 1);
    assert_ne!(rewritten, file.bytes.as_ref());
    file.bytes = rewritten.into();
    file.digest = file.bytes.checksum().into();
    fixture.definition.digest = file.digest.clone();
    let site = &mut fixture.namespaces.declarations[0].site;
    site.declaration.digest = file.digest.clone();
    site.leaf.digest = file.digest.clone();
}

#[test]
fn self_consistent_rechecksums_do_not_authorize_changed_definition_cfg_or_export() {
    for (from, to) in [
        (".with_width($e)", ".with_alternate($e)"),
        ("#[cfg(not(doc))]", "#[cfg(doc)]"),
        ("as padded_width_of;", "as another_macro;"),
    ] {
        let mut fixture = Fixture::original();
        rewrite_module(&mut fixture, from, to);
        assert!(verify(fixture.input()).is_err());
    }
}
