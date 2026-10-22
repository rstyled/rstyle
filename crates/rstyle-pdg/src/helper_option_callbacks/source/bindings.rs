//! Declaration/use names come from original bytes, never mutable binding labels.
use crate::{HirNode, unit_store::EvidenceFacts};
use syn::{ext::IdentExt, visit::Visit};

pub(super) fn payload(used: &syn::Ident, declared: &syn::Ident) -> Result<(), String> {
    if used.unraw() != declared.unraw() {
        return Err(
            "Callback source binding argument differs from its original payload parameter".into(),
        );
    }
    Ok(())
}

pub(super) fn receiver(
    used: &syn::Ident,
    closure: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let formal = crate::option_callbacks::captures::verify_structure(closure, facts)?;
    let function: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    let position = formal.parameter as usize;
    let parameter = facts
        .signature
        .params
        .get(position)
        .ok_or("Missing callback formal position")?;
    if parameter.position != formal.parameter || parameter.pat.hir != formal.root {
        return Err("Callback source binding formal position disagrees with its root".into());
    }

    let declared = declaration(function.sig.inputs.iter().nth(position))?;
    let span = crate::ByteSpan {
        start: crate::source_evidence::byte_offset(&facts.source.bytes, declared.span().start())
            as u32,
        end: crate::source_evidence::byte_offset(&facts.source.bytes, declared.span().end()) as u32,
    };
    if used.unraw() != declared.unraw() || parameter.name_span != span {
        return Err("Callback source binding receiver disagrees with its original formal".into());
    }

    let mut shadow = Shadow {
        declared: &declared,
        found: false,
    };
    shadow.visit_block(&function.block);
    if shadow.found {
        return Err("Callback source binding requires an unshadowed shared formal".into());
    }
    Ok(())
}

fn declaration(argument: Option<&syn::FnArg>) -> Result<syn::Ident, String> {
    match argument {
        Some(syn::FnArg::Receiver(receiver)) => {
            Ok(syn::Ident::new("self", receiver.self_token.span))
        }
        Some(syn::FnArg::Typed(argument)) => match argument.pat.as_ref() {
            syn::Pat::Ident(binding)
                if binding.by_ref.is_none()
                    && binding.mutability.is_none()
                    && binding.subpat.is_none()
                    && binding.attrs.is_empty() =>
            {
                Ok(binding.ident.clone())
            }
            _ => Err("Callback source binding requires an immutable original formal".into()),
        },
        None => Err("Callback source binding has no original formal declaration".into()),
    }
}

struct Shadow<'a> {
    declared: &'a syn::Ident,
    found: bool,
}

impl<'ast> Visit<'ast> for Shadow<'_> {
    fn visit_pat_ident(&mut self, binding: &'ast syn::PatIdent) {
        self.found |= binding.ident.unraw() == self.declared.unraw();
        syn::visit::visit_pat_ident(self, binding);
    }
}
