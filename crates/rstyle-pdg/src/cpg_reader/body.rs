//! The body tree: each source node of the document as the driver's construct record.

use super::{Context, hir_ref, symbol_name};
use crate::frontend::{ByteSpan, ExpansionSite, HirNode};
use rstyle_core::cpg::model::{Node, NodeKind, Origin};

impl<'a> Context<'_, 'a> {
    pub(super) fn tree(&self, node: &'a Node) -> Result<HirNode, String> {
        let span = node
            .span
            .as_ref()
            .map(|span| self.span(span))
            .unwrap_or(ByteSpan { start: 0, end: 0 });
        let children = self
            .body
            .source_children(node)?
            .into_iter()
            .map(|child| self.tree(child))
            .collect::<Result<_, _>>()?;
        let ty = node
            .ty
            .as_deref()
            .map(|ty| self.reader.resolved(ty, String::new()))
            .transpose()?;
        let symbol = self.declared(node);

        Ok(HirNode {
            hir: hir_ref(&node.id),
            kind: node.original_kind.clone(),
            span,
            snippet: self.snippet(span),
            ty,
            res: None,
            dispatch: None,
            expansion: self.expansion(node, span),
            binding: symbol.map(|(symbol, _)| symbol_name(symbol)),
            binding_mut: symbol.map(|(_, mutable)| mutable),
            lit: (node.kind == NodeKind::Literal).then(|| self.snippet(span)),
            param_index: None,
            is_unsafe_block: node.kind == NodeKind::Block
                && node.modifiers.iter().any(|modifier| modifier == "unsafe"),
            children,
        })
    }

    /// The symbol a binding pattern declares, with its mutability.
    fn declared(&self, node: &Node) -> Option<(&'a str, bool)> {
        self.body
            .function
            .symbols
            .iter()
            .find(|symbol| symbol.node.as_deref() == Some(node.id.as_str()))
            .map(|symbol| (symbol.id.as_str(), symbol.mutable))
    }

    fn expansion(&self, node: &Node, span: ByteSpan) -> Option<ExpansionSite> {
        if node.provenance.origin != Origin::MacroExpansion {
            return None;
        }
        let callsite = node.provenance.call_site.as_ref()?;
        Some(ExpansionSite {
            callsite: self.span(callsite),
            snippet: self.snippet(span),
        })
    }
}
