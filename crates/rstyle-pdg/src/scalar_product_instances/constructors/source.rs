//! Original callee spelling and ordered arguments checked through existing source resolution.
use super::*;
use crate::{concrete_types::TypeTree, data_types::TypeToken};
use syn::spanned::Spanned;

pub(super) fn verify<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
    context: &Context<'_>,
    observed: &Observation,
) -> Result<[&'a HirNode; 2], String> {
    if node.hir.krate != 0
        || node.kind != "expr_struct"
        || node.expansion.is_some()
        || node.dispatch.is_some()
        || node.binding.is_some()
        || node.binding_mut.is_some()
        || node.lit.is_some()
        || node.param_index.is_some()
        || node.is_unsafe_block
    {
        return Err(
            "Scalar constructor requires a plain original tuple-construction expression".into(),
        );
    }
    let source = crate::source_evidence::byte_range(&facts.source.bytes, node.span)?;
    if source != node.snippet {
        return Err("Scalar constructor source bytes disagree".into());
    }

    let expression = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::Call(call) = expression else {
        return Err("Scalar constructor requires an original call expression".into());
    };
    if !call.attrs.is_empty() || call.args.len() != 2 || node.children.len() != 2 {
        return Err(
            "Scalar constructor requires exactly two original and recorded arguments".into(),
        );
    }
    let proof = Source {
        node,
        facts,
        context,
        observed,
    };

    proof.callee(&call.func)?;

    let expected = TypeToken::from_tree(&TypeTree::ScalarProductInstance(Box::new(
        context.instance.clone(),
    )));
    typed(node, &expected)?;
    proof.fields(&call)
}

struct Source<'a, 'f, 'c> {
    node: &'a HirNode,
    facts: &'f dyn EvidenceFacts,
    context: &'f Context<'c>,
    observed: &'f Observation,
}

impl<'a> Source<'a, '_, '_> {
    fn callee(&self, source: &syn::Expr) -> Result<(), String> {
        let context = self.context;
        let syn::Expr::Path(path) = source else {
            return Err("Scalar constructor callee must be an original direct path".into());
        };
        if !path.attrs.is_empty()
            || path.qself.is_some()
            || crate::source_evidence::plain_relative_path(&path.path).is_none()
        {
            return Err(
                "Scalar constructor callee path is outside the bounded source grammar".into(),
            );
        }

        let source_path = syn::TypePath {
            qself: None,
            path: path.path.clone(),
        };
        let expected = TypeTree::ScalarProductInstance(Box::new(context.instance.clone()));
        if context
            .resolution
            .scalar_product_constructor_type(&source_path)?
            != expected
        {
            return Err(
                "Original scalar constructor path resolves to a different source instance".into(),
            );
        }

        self.callee_identity(path)
    }

    fn callee_identity(&self, path: &syn::ExprPath) -> Result<(), String> {
        let node = self.node;
        let facts = self.facts;
        let captured = &self.observed.callee;

        let root_bytes = crate::source_evidence::byte_range(&facts.source.bytes, node.span)?;
        let relative = crate::source_evidence::original_span(root_bytes, path.span());
        let expected_span = crate::ByteSpan {
            start: node.span.start + relative.start,
            end: node.span.start + relative.end,
        };
        if captured.span != expected_span
            || captured.hir.krate != node.hir.krate
            || captured.hir.owner != node.hir.owner
            || captured.hir == node.hir
            || crate::source_evidence::byte_range(&facts.source.bytes, captured.span)?
                != captured.snippet
            || node
                .children
                .iter()
                .any(|child| contains(child, &captured.hir))
        {
            return Err(
                "Scalar constructor actual callee identity or source span disagrees".into(),
            );
        }

        Ok(())
    }

    fn fields(&self, call: &syn::ExprCall) -> Result<[&'a HirNode; 2], String> {
        let Self {
            node,
            facts,
            context,
            observed,
        } = self;
        let expected = [
            TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
                "u8".into(),
            ))))),
            TypeToken::Scalar(context.instance.argument.as_str().into()),
        ];
        if observed.values[0] == observed.values[1]
            || observed.values.iter().any(|id| {
                id.krate != node.hir.krate || id.owner != node.hir.owner || *id == node.hir
            })
        {
            return Err("Scalar constructor argument identities are duplicate or foreign".into());
        }

        for (position, (syntax, child)) in call.args.iter().zip(&node.children).enumerate() {
            crate::source_evidence::verify_child_span(node, child, syntax.span(), *facts)?;
            if child.hir != observed.values[position]
                || crate::source_evidence::byte_range(&facts.source.bytes, child.span)?
                    != child.snippet
            {
                return Err(
                    "Scalar constructor ordered argument source or identity disagrees".into(),
                );
            }
            typed(child, &expected[position])?;
        }
        Ok([&node.children[0], &node.children[1]])
    }
}

fn typed(node: &HirNode, expected: &TypeToken) -> Result<(), String> {
    let normalized = node
        .ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_ref())
        .ok_or("Unknown scalar constructor expression type")?;
    if crate::data_types::parse(normalized)? != *expected {
        return Err(
            "Scalar constructor concrete expression type differs from its field/result".into(),
        );
    }
    Ok(())
}

fn contains(node: &HirNode, id: &crate::HirRef) -> bool {
    node.hir == *id || node.children.iter().any(|child| contains(child, id))
}
