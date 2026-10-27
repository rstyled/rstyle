//! Local type-root shadowing in the original blocks enclosing one exact use.
use crate::ByteSpan;
use syn::{ext::IdentExt, spanned::Spanned, visit::Visit};

pub(crate) struct TypeRootUse<'a> {
    pub bytes: &'a str,
    pub name: &'a str,
    pub target: ByteSpan,
    pub parameter_error: &'static str,
    pub local_error: &'static str,
}

pub(crate) fn verify(
    signature: &syn::Signature,
    block: &syn::Block,
    site: TypeRootUse<'_>,
    shadows: impl Fn(&syn::Stmt, &str) -> bool,
) -> Result<(), String> {
    if signature.generics.params.iter().any(|parameter| {
        matches!(parameter, syn::GenericParam::Type(parameter) if parameter.ident.unraw() == site.name)
    }) {
        return Err(site.parameter_error.into());
    }
    let mut scope = Scope {
        site,
        shadows,
        found: false,
        shadowed: false,
    };
    scope.visit_block(block);
    if !scope.found || scope.shadowed {
        return Err(scope.site.local_error.into());
    }

    Ok(())
}

struct Scope<'a, F> {
    site: TypeRootUse<'a>,
    shadows: F,
    found: bool,
    shadowed: bool,
}

impl<'ast, F: Fn(&syn::Stmt, &str) -> bool> Visit<'ast> for Scope<'_, F> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let span = super::original_span(self.site.bytes, block.span());
        if span.start > self.site.target.start || span.end < self.site.target.end {
            return;
        }

        self.found = true;
        self.shadowed |= block
            .stmts
            .iter()
            .any(|statement| (self.shadows)(statement, self.site.name));
        syn::visit::visit_block(self, block);
    }
}
