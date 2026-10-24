//! Original declaration tokens, effective modes and the lexical binding inventory.
use super::*;

impl Proof<'_, '_> {
    pub(super) fn binding(
        &self,
        syntax: &syn::Pat,
        node: &HirNode,
        rest: bool,
    ) -> Result<Observation, String> {
        let syn::Pat::Ident(ident) = syntax else {
            return Err(unsupported("Expected an original @ binding"));
        };
        if !ident.attrs.is_empty()
            || ident.by_ref.is_some()
            || ident.mutability.is_some()
            || ident.subpat.is_none()
        {
            return Err(unsupported("Only plain immutable @ bindings are in scope"));
        }

        self.source(node, syntax.span())?;
        let inner = if rest {
            TypeShape::Slice(Box::new(TypeShape::Byte))
        } else {
            TypeShape::Byte
        };
        let receipt = self.observed(node, &TypeShape::Shared(Box::new(inner)))?;

        let Details::Binding {
            binding,
            subpattern,
        } = &receipt.details
        else {
            return Err(unsupported("Missing raw binding descriptor"));
        };
        if node.kind != "pat_binding_at"
            || node.children.len() != 1
            || subpattern.as_ref() != Some(&node.children[0].hir)
            || !receipt.adjustments.is_empty()
            || binding.canonical != node.hir
        {
            return Err(unsupported(
                "Binding node, canonical identity or child roster disagrees",
            ));
        }

        self.binding_mode(binding)?;
        self.identifier(ident, node, binding)?;
        self.inventory(node, binding)?;

        Ok(receipt)
    }

    fn binding_mode(&self, binding: &Binding) -> Result<(), String> {
        let mode = binding
            .effective_mode
            .as_ref()
            .ok_or_else(|| unknown("Missing effective binding mode"))?;
        let mutable = binding
            .effective_mutable
            .ok_or_else(|| unknown("Missing effective binding mutability"))?;
        if binding.written_mode != BindingMode::Value
            || binding.written_mutable
            || *mode != BindingMode::Shared
            || mutable
        {
            return Err(unsupported(
                "Expected immutable shared match-ergonomic binding",
            ));
        }
        Ok(())
    }

    fn identifier(
        &self,
        syntax: &syn::PatIdent,
        node: &HirNode,
        binding: &Binding,
    ) -> Result<(), String> {
        let bytes = crate::source_evidence::byte_range(&self.facts.source.bytes, self.root.span)?;
        let local = crate::source_evidence::original_span(bytes, syntax.ident.span());
        let expected = crate::ByteSpan {
            start: self.root.span.start + local.start,
            end: self.root.span.start + local.end,
        };

        let symbol = format!("s:{}:{}", binding.canonical.owner, binding.canonical.local);
        if expected != binding.identifier_span
            || syntax.ident != binding.name
            || node.binding.as_ref() != Some(&symbol)
            || node.binding_mut != Some(false)
            || crate::source_evidence::byte_range(&self.facts.source.bytes, expected)?
                != binding.name
        {
            return Err(unsupported(
                "Original identifier token, symbol or mutability disagrees",
            ));
        }
        Ok(())
    }

    fn inventory(&self, node: &HirNode, binding: &Binding) -> Result<(), String> {
        let mut matches =
            self.facts.bindings.iter().filter(|entry| {
                entry.hir == node.hir || Some(&entry.symbol) == node.binding.as_ref()
            });
        let found = matches
            .next()
            .ok_or_else(|| unknown("Missing lexical binding inventory entry"))?;
        if matches.next().is_some()
            || Some(&found.symbol) != node.binding.as_ref()
            || found.hir != node.hir
            || found.name != binding.name
            || found.decl_span != binding.identifier_span
            || found.scope != self.scope
            || found.is_param
            || found.is_mut
            || node.ty.as_ref() != Some(&found.ty)
        {
            return Err(unsupported(
                "Lexical binding inventory does not match the original arm declaration",
            ));
        }
        Ok(())
    }
}

/// These binders share one lexical arm; Rust never permits duplicate declarations here.
pub(super) fn unique_names(syntax: &syn::PatSlice) -> Result<(), String> {
    let mut names = std::collections::BTreeSet::new();
    for pattern in &syntax.elems {
        let syn::Pat::Ident(binding) = pattern else {
            return Err(unsupported(
                "Each prefix/rest component requires an original named binding",
            ));
        };
        if !names.insert(binding.ident.to_string()) {
            return Err(unsupported(
                "Duplicate source bindings in the same match arm",
            ));
        }
    }
    Ok(())
}
