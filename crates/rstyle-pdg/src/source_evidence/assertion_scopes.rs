//! Lexical closure through independently verified standard assertions.
use super::*;
use crate::unit_store::EvidenceFacts;

pub(crate) fn closed_assertion_body(
    body: &syn::Block,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let mut visitor = Scopes {
        facts,
        result: Ok(()),
    };
    visitor.visit_block(body);
    visitor.result
}

struct Scopes<'a> {
    facts: &'a dyn EvidenceFacts,
    result: Result<(), String>,
}

impl<'ast> Visit<'ast> for Scopes<'_> {
    fn visit_item(&mut self, _: &'ast syn::Item) {
        self.result = Err("Source body has unresolved block-local item/macro scopes".into());
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.result.is_err() {
            return;
        }
        match checked_arguments(mac, self.facts) {
            Ok(arguments) => {
                for argument in &arguments {
                    self.visit_expr(argument);
                }
            }
            Err(error) => self.result = Err(error),
        }
    }
}

fn checked_arguments(
    mac: &syn::Macro,
    facts: &dyn EvidenceFacts,
) -> Result<syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>, String> {
    use syn::parse::Parser;
    let span = syntax_span(&facts.source.bytes, mac);

    let nodes = crate::field_locations::selected_operations(facts, &["expr_assert"], |node, _| {
        node.span == span
    });
    let [node] = nodes.as_slice() else {
        return Err("Source macro lacks its unique retained assertion".into());
    };

    let source = byte_range(&facts.source.bytes, span)?;
    if node.snippet != source {
        return Err("Assertion lexical source differs from retained source".into());
    }

    crate::assertions::verify(
        node,
        source,
        facts.build.debug_assertions.value,
        crate::extraction_profile::ExtractionProfile::from_facts(facts)?,
    )?;
    syn::punctuated::Punctuated::parse_terminated
        .parse2(mac.tokens.clone())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExpansionSite, HirFnFacts, PathRes};

    fn fixture(condition: &str) -> (HirFnFacts, syn::ItemFn) {
        let mut facts: HirFnFacts = serde_json::from_str(include_str!(
            "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
        ))
        .unwrap();
        facts.source.bytes = format!("fn check() {{ assert!({condition}); }}").into();
        facts.build.frontend_options.insert(
            "extraction_profile".into(),
            serde_json::json!("hir-data-v3"),
        );
        let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes).unwrap();
        let syn::Stmt::Macro(statement) = &syntax.block.stmts[0] else {
            panic!()
        };
        let span = syntax_span(&facts.source.bytes, &statement.mac);
        install_assertion(&mut facts, span);
        (facts, syntax)
    }

    fn install_assertion(facts: &mut HirFnFacts, span: ByteSpan) {
        let source = byte_range(&facts.source.bytes, span).unwrap().to_owned();
        let mut condition_node = facts.body.clone();
        condition_node.children.clear();
        condition_node.kind = "expr_block".into();
        condition_node.span = crate::assertions::Assertion::parse(&source)
            .unwrap()
            .condition_span(&source, span.start);
        condition_node.ty = Some(crate::ResolvedTy {
            normalized: Some("bool".into()),
            observed: "bool".into(),
            spelling: String::new(),
        });
        facts.body.children = vec![condition_node];
        facts.body.kind = "expr_assert".into();
        facts.body.span = span;
        facts.body.snippet = source.clone();
        facts.body.ty = Some(crate::ResolvedTy {
            normalized: Some("unit".into()),
            observed: "()".into(),
            spelling: String::new(),
        });
        facts.body.lit = Some("true".into());
        facts.body.res = Some(PathRes {
            kind: "standard_assertion".into(),
            description: "core::assert".into(),
            target: None,
        });
        facts.body.expansion = Some(ExpansionSite {
            callsite: span,
            snippet: source,
        });
    }

    #[test]
    fn assertion_scope_checks_original_arguments_and_retained_authority() {
        for condition in ["true", "{ let saved = true; saved }"] {
            let (facts, syntax) = fixture(condition);
            assert!(closed_assertion_body(&syntax.block, &facts).is_ok());
            assert!(closed_body(&syntax.block).is_err());
            let mut missing = facts.clone();
            missing.body.res.as_mut().unwrap().description = "user::assert".into();
            assert!(closed_assertion_body(&syntax.block, &missing).is_err());
            missing.body = facts.body.children[0].clone();
            assert!(closed_assertion_body(&syntax.block, &missing).is_err());
        }
    }

    #[test]
    fn assertion_arguments_cannot_hide_items_imports_or_nested_macros() {
        for condition in [
            "{ fn hidden() {} true }",
            "{ use core::result::Result as Hidden; true }",
            "{ macro_rules! hidden { () => {true} } true }",
            "hidden!()",
        ] {
            let (facts, syntax) = fixture(condition);
            assert!(
                closed_assertion_body(&syntax.block, &facts).is_err(),
                "{condition}"
            );
        }
    }
}
