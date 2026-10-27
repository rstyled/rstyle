//! Exact local From<Range<usize>> owner; value consumption is proved elsewhere.
use super::*;

pub(super) fn conversion(
    facts: &HirFnFacts,
    resolution: &Resolution,
    module: &census::Module,
    item: &syn::ItemImpl,
) -> Result<Owner, String> {
    let bytes = resolution.environment.source(&module.path)?.bytes();
    let method = exact_method(item, bytes, facts).ok_or("Missing exact reverse Range method")?;
    let scope = Scope {
        resolution,
        namespace: &module.namespace,
    };
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    if !configuration.active(&item.attrs)? || !configuration.active(&method.attrs)? {
        return Err("Reverse Range conversion owner is inactive".into());
    }

    implementation(item)?;
    selected::function_attributes(facts, &item.attrs)?;
    selected::function_attributes(facts, &method.attrs)?;
    selected::check_namespace(facts, &resolution.environment, &module.namespace)?;
    let output = trait_owner(&scope, item)?;
    method_signature(&scope, method, &output)?;
    crate::source_evidence::closed_body(&method.block)?;
    let parameter = captured_interface(facts, resolution, &output)?;
    Ok(Owner {
        namespace: module.namespace.clone(),
        implementation: implementation_source_span(bytes, item),
        parameter,
    })
}

fn trait_owner(scope: &Scope<'_>, item: &syn::ItemImpl) -> Result<TypeTree, String> {
    let (input, output) = scope.conversion_paths(item)?;
    if !scope.range(input)? {
        return Err("Reverse Range conversion input is not exact Range<usize>".into());
    }

    let tree = resolve(scope, output)?;
    if !matches!(tree, TypeTree::Struct { .. }) {
        return Err("Reverse Range conversion requires a concrete local struct owner".into());
    }
    Ok(tree)
}

fn method_signature(
    scope: &Scope<'_>,
    method: &syn::ImplItemFn,
    output: &TypeTree,
) -> Result<(), String> {
    signature(&method.sig)?;
    if !matches!(method.vis, syn::Visibility::Inherited) || method.defaultness.is_some() {
        return Err("Reverse Range method has unsupported visibility/defaultness".into());
    }
    let Some(syn::FnArg::Typed(parameter)) = method.sig.inputs.first() else {
        return Err("Reverse Range method requires one value input".into());
    };
    if !parameter.attrs.is_empty()
        || !matches!(parameter.pat.as_ref(),
            syn::Pat::Ident(binding) if binding.by_ref.is_none() && binding.mutability.is_none()
                && binding.subpat.is_none() && binding.attrs.is_empty())
    {
        return Err("Reverse Range method requires one plain immutable value binding".into());
    }

    let syn::Type::Path(input) = parameter.ty.as_ref() else {
        return Err("Reverse Range method input is not a concrete path".into());
    };
    if !scope.range(input)? {
        return Err("Reverse Range method input differs from its exact trait input".into());
    }

    let syn::ReturnType::Type(_, result) = &method.sig.output else {
        return Err("Reverse Range method lacks an explicit output".into());
    };
    let syn::Type::Path(result) = result.as_ref() else {
        return Err("Reverse Range method result is not a concrete path".into());
    };
    if !(result.qself.is_none() && result.path.is_ident("Self"))
        && resolve(scope, result)? != *output
    {
        return Err("Reverse Range method result differs from its exact local owner".into());
    }
    Ok(())
}

fn captured_interface(
    facts: &HirFnFacts,
    resolution: &Resolution,
    output: &TypeTree,
) -> Result<TypeTree, String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let [Observation::Captured { ty: input }] = interface.parameters.as_slice() else {
        return Err("Reverse Range method requires its sole captured concrete input".into());
    };
    if !matches!(input, TypeTree::RangeUsize { .. })
        || resolution.range_value.as_ref() != Some(input)
        || !matches!(&interface.result, Observation::Captured { ty } if ty == output)
    {
        return Err("Reverse Range lexical owner differs from its captured interface".into());
    }
    Ok(input.clone())
}

fn resolve(scope: &Scope<'_>, path: &syn::TypePath) -> Result<TypeTree, String> {
    (Scoped {
        resolution: scope.resolution,
        namespace: scope.namespace,
    })
    .resolve(path)
}

#[cfg(test)]
mod tests;
