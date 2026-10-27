//! Caller syntax and namespaces remain independent of the callee receipt.
use super::*;
use crate::concrete_types::{InterfaceObservations, Observation, source_proof};
use crate::copy_tagged_values::typed;
use source_proof::PathResolution;
use syn::spanned::Spanned;

#[cfg(test)]
mod tests;

pub(crate) fn verify(facts: &dyn EvidenceFacts, checked: &impl WholeCall) -> Result<(), String> {
    let resolution = facts.nominal_resolution()?;
    let source = crate::data_frontend::source_function(facts, Some(&resolution))?;
    shape(&source, checked.permits_const())?;
    let Some(syn::FnArg::Typed(parameter)) = source.sig.inputs.first() else {
        return Err("Tagged caller requires one ordinary formal".into());
    };
    let syn::Pat::Ident(binding) = parameter.pat.as_ref() else {
        return Err("Tagged caller requires one immutable identifier".into());
    };
    immutable_binding(binding)?;

    let resolver = Resolver {
        facts,
        resolution: &resolution,
    };
    source_proof::verify_top_syntax(&parameter.ty, checked.input(), &resolver)?;
    let syn::ReturnType::Type(_, output) = &source.sig.output else {
        return Err("Tagged caller requires its explicit complete return type".into());
    };
    source_proof::verify_top_syntax(output, checked.output(), &resolver)?;
    interface(facts, checked)?;
    syntax(facts, checked, &source)?;
    binding_source(facts, checked, binding)
}

pub(crate) fn shape(source: &syn::ItemFn, permits_const: bool) -> Result<(), String> {
    let signature = &source.sig;
    if !source.attrs.is_empty()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.unsafety.is_some()
        || signature.asyncness.is_some()
        || signature.constness.is_some() && !permits_const
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || signature.inputs.len() != 1
    {
        return Err("Tagged caller requires an ordinary nongeneric safe Rust function".into());
    }
    if !matches!(
        source.block.stmts.as_slice(),
        [syn::Stmt::Expr(syn::Expr::Call(_), None)]
    ) {
        return Err("Tagged caller requires one immediate whole-return call".into());
    }
    Ok(())
}

fn immutable_binding(binding: &syn::PatIdent) -> Result<(), String> {
    if !binding.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
    {
        return Err("Tagged caller requires an unadorned immutable formal".into());
    }
    Ok(())
}

fn interface(facts: &dyn EvidenceFacts, checked: &impl WholeCall) -> Result<(), String> {
    let observed = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let [Observation::Captured { ty: input }] = observed.parameters.as_slice() else {
        return Err("Tagged caller lacks its complete captured formal type".into());
    };
    if input != checked.input()
        || observed.result
            != (Observation::Captured {
                ty: checked.output().clone(),
            })
        || facts.signature.params.len() != 1
        || facts.signature.has_self
        || facts.signature.receiver.is_some()
    {
        return Err(
            "Tagged caller interface differs from the verified conversion endpoints".into(),
        );
    }

    typed(checked.call(), checked.output())?;
    typed(checked.actual(), checked.input())?;
    typed(checked.formal(), checked.input())?;

    let input = crate::data_types::TypeToken::from_tree(checked.input()).text()?;
    let output = crate::data_types::TypeToken::from_tree(checked.output()).text()?;
    if facts.signature.params[0].ty.normalized.as_deref() != Some(input.as_str())
        || facts.signature.return_ty.normalized.as_deref() != Some(output.as_str())
    {
        return Err("Tagged caller serialized signature changes its complete types".into());
    }
    Ok(())
}

fn syntax(
    facts: &dyn EvidenceFacts,
    checked: &impl WholeCall,
    source: &syn::ItemFn,
) -> Result<(), String> {
    let [syn::Stmt::Expr(syn::Expr::Call(call), None)] = source.block.stmts.as_slice() else {
        return Err("Missing exact tagged caller source operation".into());
    };

    if !call.attrs.is_empty()
        || call.args.len() != 1
        || crate::source_evidence::original_span(&facts.source.bytes, call.span())
            != checked.call().span
        || crate::source_evidence::original_span(&facts.source.bytes, source.block.span())
            != facts.body.span
    {
        return Err("Tagged caller source body/call correspondence disagrees".into());
    }
    let syn::Expr::Path(actual) = &call.args[0] else {
        return Err("Tagged caller actual must be its whole formal".into());
    };
    if !actual.attrs.is_empty()
        || actual.qself.is_some()
        || crate::source_evidence::original_span(&facts.source.bytes, actual.span())
            != checked.actual().span
        || crate::source_evidence::resolved_local_leaf(checked.actual())
            != Some(&checked.formal().hir)
    {
        return Err("Tagged caller must save its original formal exactly once".into());
    }
    Ok(())
}

fn binding_source(
    facts: &dyn EvidenceFacts,
    checked: &impl WholeCall,
    binding: &syn::PatIdent,
) -> Result<(), String> {
    let formal = &facts.signature.params[0];
    let expression = crate::source_evidence::original_expression(checked.actual(), facts)?;
    let syn::Expr::Path(actual) = expression else {
        return Err("Missing original actual identifier".into());
    };

    if crate::source_evidence::plain_relative_path(&actual.path)
        != Some(vec![binding.ident.to_string()])
        || checked.formal().kind != "pat_binding"
        || !checked.formal().children.is_empty()
        || formal.position != 0
        || formal.name_span
            != crate::source_evidence::original_span(&facts.source.bytes, binding.ident.span())
        || checked.formal().span
            != crate::source_evidence::original_span(&facts.source.bytes, binding.span())
    {
        return Err("Tagged caller formal source/binding correspondence disagrees".into());
    }
    Ok(())
}

struct Resolver<'a> {
    facts: &'a dyn EvidenceFacts,
    resolution: &'a crate::nominal_boundary::resolution::Resolution,
}

impl PathResolution for Resolver<'_> {
    fn resolve(&self, path: &syn::TypePath) -> Result<TypeTree, String> {
        self.resolution.resolve(path)
    }

    fn scalar_option(&self, path: &syn::TypePath, payload: &TypeTree) -> Result<(), String> {
        if matches!(payload, TypeTree::ScalarProductInstance(_)) {
            return self.resolution.verify_scalar_product_option_type(
                self.facts.facts(),
                path,
                payload,
            );
        }
        self.resolution
            .verify_product_option_type(self.facts.facts(), path, payload)
    }

    fn concrete_result(
        &self,
        path: &syn::TypePath,
        _: &TypeTree,
        _: &TypeTree,
    ) -> Result<(), String> {
        self.resolution
            .verify_result_type_source(self.facts.facts(), path)
    }
}
