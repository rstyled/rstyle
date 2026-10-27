//! Exact source ownership and lexical spellings for concrete Range conversions.
//! This proves neither pinned library identity nor move/drop/type authority.
use super::*;

mod reverse;

pub(super) struct Owner {
    pub namespace: Namespace,
    pub implementation: ByteSpan,
    pub parameter: TypeTree,
}

pub(super) fn owner(facts: &HirFnFacts, resolution: &Resolution) -> Result<Option<Owner>, String> {
    find_owner(facts, resolution, conversion)
}

/// Reverse lexical ownership only; consumes no Range and grants no Copy proof.
pub(super) fn reverse_owner(
    facts: &HirFnFacts,
    resolution: &Resolution,
) -> Result<Option<Owner>, String> {
    find_owner(facts, resolution, reverse::conversion)
}

fn find_owner(
    facts: &HirFnFacts,
    resolution: &Resolution,
    check: impl Fn(&HirFnFacts, &Resolution, &census::Module, &syn::ItemImpl) -> Result<Owner, String>,
) -> Result<Option<Owner>, String> {
    resolution.environment.check_context(facts)?;
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing Range conversion source file")?;
    let mut found = None;

    for module in resolution
        .environment
        .modules
        .iter()
        .filter(|module| module.path.to_string_lossy() == file)
    {
        let bytes = resolution.environment.source(&module.path)?.bytes();
        for item in &module.items {
            if let syn::Item::Impl(item) = item
                && exact_method(item, bytes, facts).is_some()
            {
                let candidate = check(facts, resolution, module, item)?;
                if found.replace(candidate).is_some() {
                    return Err("Ambiguous exact Range conversion owner".into());
                }
            }
        }
    }
    Ok(found)
}

pub(super) fn observed<'a>(
    facts: &HirFnFacts,
    observations: impl Iterator<Item = &'a Observation>,
) -> Result<Option<TypeTree>, String> {
    if !supports(facts, "rust.fresh-range-results")? {
        return Ok(None);
    }
    let mut range = None;

    for observation in observations {
        if let Observation::Captured {
            ty: ty @ TypeTree::RangeUsize { .. },
        } = observation
        {
            if range.as_ref().is_some_and(|known| known != ty) {
                return Err("Conflicting concrete Range observations".into());
            }
            range = Some(ty.clone());
        }
    }
    Ok(range)
}

pub(super) fn bind(facts: &HirFnFacts, resolution: &mut Resolution) -> Result<bool, String> {
    if bind_reverse(facts, resolution)? {
        return Ok(true);
    }
    if facts.name.rsplit("::").next() != Some("from") {
        return Ok(false);
    }
    let Some(range) = resolution.range_value.clone() else {
        return Ok(false);
    };
    let Some(owner) = owner(facts, resolution)? else {
        return Ok(false);
    };
    if !resolution.checked_owned_copy(&owner.parameter) {
        return Err("Range conversion input lacks checked local Copy ownership".into());
    }

    install_owner(resolution, owner, range);
    Ok(true)
}

/// Bind only the gated, captured reverse interface after exact lexical proof.
fn bind_reverse(facts: &HirFnFacts, resolution: &mut Resolution) -> Result<bool, String> {
    if !supports(facts, "rust.consuming-range-conversions")?
        || facts.name.rsplit("::").next() != Some("from")
    {
        return Ok(false);
    }
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let [
        Observation::Captured {
            ty: TypeTree::RangeUsize { .. },
        },
    ] = interface.parameters.as_slice()
    else {
        return Ok(false);
    };
    let Observation::Captured {
        ty: result @ TypeTree::Struct { .. },
    } = &interface.result
    else {
        return Ok(false);
    };
    let Some(owner) = reverse_owner(facts, resolution)? else {
        return Ok(false);
    };
    if !resolution.checked_owned_copy(result) {
        return Err("Reverse Range result lacks checked local Copy ownership".into());
    }

    install_owner(resolution, owner, result.clone());
    Ok(true)
}

pub(super) fn resolve(
    path: &syn::TypePath,
    scoped: &Scoped<'_>,
) -> Result<Option<TypeTree>, String> {
    let Some(tree) = &scoped.resolution.range_value else {
        return Ok(None);
    };
    let is_self = path.qself.is_none()
        && path.path.is_ident("Self")
        && matches!(
            scoped.resolution.receiver,
            Some(TypeTree::RangeUsize { .. })
        );
    if is_self
        || (Scope {
            resolution: scoped.resolution,
            namespace: scoped.namespace,
        })
        .range(path)?
    {
        return Ok(Some(tree.clone()));
    }
    Ok(None)
}

pub(super) fn call(
    node: &crate::HirNode,
    caller: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if !supports(caller.facts(), "rust.fresh-range-results")?
        || node.kind != "expr_call"
        || node.expansion.is_some()
    {
        return Err("Range From call requires its original registered expression".into());
    }
    let source = caller
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .filter(|source| *source == node.snippet)
        .ok_or("Range call differs from original source range")?;
    let parsed: syn::ExprCall = syn::parse_str(source).map_err(|error| error.to_string())?;
    let syn::Expr::Path(path) = parsed.func.as_ref() else {
        return Err("Range From call requires a direct source path".into());
    };
    if !parsed.attrs.is_empty() || !path.attrs.is_empty() || parsed.args.len() != 1 {
        return Err("Range From call requires one original value argument".into());
    }

    let function: syn::ItemFn =
        syn::parse_str(&caller.source.bytes).map_err(|error| error.to_string())?;
    crate::source_evidence::closed_body(&function.block)?;
    let resolution = caller.nominal_resolution()?;
    Scope {
        resolution: &resolution,
        namespace: &resolution.namespace,
    }
    .call_path(path)
}

fn exact_method<'a>(
    item: &'a syn::ItemImpl,
    bytes: &str,
    facts: &HirFnFacts,
) -> Option<&'a syn::ImplItemFn> {
    item.trait_.as_ref()?;
    item.items.iter().find_map(|member| {
        let syn::ImplItem::Fn(method) = member else {
            return None;
        };
        (FunctionSource {
            vis: &method.vis,
            sig: &method.sig,
            body: &method.block,
        })
        .exact(bytes, facts)
        .then_some(method)
    })
}

fn conversion(
    facts: &HirFnFacts,
    resolution: &Resolution,
    module: &census::Module,
    item: &syn::ItemImpl,
) -> Result<Owner, String> {
    let bytes = resolution.environment.source(&module.path)?.bytes();
    let method = exact_method(item, bytes, facts).ok_or("Missing exact Range method")?;
    let scope = Scope {
        resolution,
        namespace: &module.namespace,
    };
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    if !configuration.active(&item.attrs)? || !configuration.active(&method.attrs)? {
        return Err("Range conversion owner is inactive".into());
    }

    implementation(item)?;
    selected::function_attributes(facts, &item.attrs)?;
    selected::function_attributes(facts, &method.attrs)?;
    selected::check_namespace(facts, &resolution.environment, &module.namespace)?;

    let input = scope.trait_input(item)?;
    let parameter = (Scoped {
        resolution,
        namespace: &module.namespace,
    })
    .resolve(input)?;
    if !matches!(parameter, TypeTree::Struct { .. }) {
        return Err("Range conversion requires a concrete local struct input".into());
    }

    scope.method(method, input)?;
    crate::source_evidence::closed_body(&method.block)?;
    Ok(Owner {
        namespace: module.namespace.clone(),
        implementation: implementation_source_span(bytes, item),
        parameter,
    })
}

fn implementation(item: &syn::ItemImpl) -> Result<(), String> {
    if item.unsafety.is_some()
        || item.defaultness.is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || item.items.len() != 1
    {
        return Err("Range conversion requires one safe nongeneric implementation member".into());
    }
    Ok(())
}

/// Module-level spelling only; callers must reject or prove block-local items.
/// Pinned Range type and move/drop evidence remain independent obligations.
#[cfg(test)]
fn range_path(path: &syn::TypePath, resolution: &Resolution) -> Result<bool, String> {
    if path.qself.is_none() && path.path.is_ident("Self") {
        return Ok(matches!(
            resolution.receiver,
            Some(TypeTree::RangeUsize { .. })
        ));
    }
    Scope {
        resolution,
        namespace: &resolution.namespace,
    }
    .range(path)
}

struct Scope<'a> {
    resolution: &'a Resolution,
    namespace: &'a Namespace,
}

impl Scope<'_> {
    fn call_path(&self, expression: &syn::ExprPath) -> Result<(), String> {
        let mut path = expression.path.clone();
        let method = path
            .segments
            .pop()
            .ok_or("Empty Range From call path")?
            .into_value();
        if method.ident != "from" || !matches!(method.arguments, syn::PathArguments::None) {
            return Err("Range From call has a different method or generic arguments".into());
        }
        if let Some(qualification) = &expression.qself {
            return self.qualified_call(qualification, &path);
        }

        let owner = path
            .segments
            .last_mut()
            .ok_or("Range From call lacks an owner")?;
        if matches!(owner.arguments, syn::PathArguments::None) {
            owner.arguments = syn::PathArguments::AngleBracketed(syn::parse_quote!(<usize>));
        }
        if self.range(&syn::TypePath { qself: None, path })? {
            return Ok(());
        }
        Err("Range From call lacks an exact lexical Range owner".into())
    }

    fn qualified_call(&self, qualification: &syn::QSelf, path: &syn::Path) -> Result<(), String> {
        let syn::Type::Path(owner) = qualification.ty.as_ref() else {
            return Err("Qualified Range From call lacks a concrete owner".into());
        };
        if qualification.as_token.is_none()
            || qualification.position != path.segments.len()
            || !self.range(owner)?
            || !self.library(path, &["convert", "From"], true)?
        {
            return Err("Qualified Range From call lacks its exact owner/trait".into());
        }

        let syn::Type::Path(input) = argument(path)? else {
            return Err("Qualified Range From input lacks a local concrete type".into());
        };
        let parameter = (Scoped {
            resolution: self.resolution,
            namespace: self.namespace,
        })
        .resolve(input)?;
        if !matches!(parameter, TypeTree::Struct { .. })
            || !self.resolution.checked_owned_copy(&parameter)
        {
            return Err("Qualified Range From input lacks checked local Copy ownership".into());
        }
        Ok(())
    }

    fn trait_input<'a>(&self, item: &'a syn::ItemImpl) -> Result<&'a syn::TypePath, String> {
        let (input, output) = self.conversion_paths(item)?;
        if !self.range(output)? {
            return Err("Range conversion owner is not exact Range<usize>".into());
        }
        Ok(input)
    }

    fn conversion_paths<'a>(
        &self,
        item: &'a syn::ItemImpl,
    ) -> Result<(&'a syn::TypePath, &'a syn::TypePath), String> {
        let Some((negative, path, _)) = &item.trait_ else {
            return Err("Range conversion lacks a trait owner".into());
        };
        if negative.is_some() || !self.library(path, &["convert", "From"], true)? {
            return Err("Range conversion lacks exact core/std From ownership".into());
        }

        let syn::Type::Path(input) = argument(path)? else {
            return Err("Range From input is not a concrete path".into());
        };
        let syn::Type::Path(output) = item.self_ty.as_ref() else {
            return Err("Range conversion owner is not a concrete type path".into());
        };
        Ok((input, output))
    }

    fn method(&self, method: &syn::ImplItemFn, input: &syn::TypePath) -> Result<(), String> {
        signature(&method.sig)?;
        if !matches!(method.vis, syn::Visibility::Inherited) || method.defaultness.is_some() {
            return Err("Range conversion has unsupported member visibility/defaultness".into());
        }
        let Some(syn::FnArg::Typed(parameter)) = method.sig.inputs.first() else {
            return Err("Range conversion requires one value input".into());
        };
        if !parameter.attrs.is_empty()
            || !matches!(parameter.pat.as_ref(),
            syn::Pat::Ident(binding) if binding.by_ref.is_none() && binding.mutability.is_none()
                && binding.subpat.is_none() && binding.attrs.is_empty())
        {
            return Err("Range conversion requires one plain immutable value binding".into());
        }

        let syn::Type::Path(parameter_type) = parameter.ty.as_ref() else {
            return Err("Range conversion input is not a concrete path".into());
        };
        let resolver = Scoped {
            resolution: self.resolution,
            namespace: self.namespace,
        };
        if resolver.resolve(parameter_type)? != resolver.resolve(input)? {
            return Err("Range conversion method input differs from trait input".into());
        }
        self.output(&method.sig.output)
    }

    fn output(&self, output: &syn::ReturnType) -> Result<(), String> {
        let syn::ReturnType::Type(_, output) = output else {
            return Err("Range conversion lacks an explicit output".into());
        };
        let syn::Type::Path(path) = output.as_ref() else {
            return Err("Range conversion output is not a type path".into());
        };
        if path.qself.is_none() && path.path.is_ident("Self") || self.range(path)? {
            return Ok(());
        }
        Err("Range conversion output differs from its exact owner".into())
    }

    fn range(&self, path: &syn::TypePath) -> Result<bool, String> {
        if path.qself.is_some()
            || !self.range_candidate(&path.path)
            || !self.library(&path.path, &["ops", "Range"], false)?
        {
            return Ok(false);
        }
        let syn::Type::Path(element) = argument(&path.path)? else {
            return Err("Range element is not exact usize".into());
        };
        if element.qself.is_some()
            || !element.path.is_ident("usize")
            || self.lookup("usize")?.is_some()
        {
            return Err("Range requires an unshadowed usize element".into());
        }
        Ok(true)
    }

    fn range_candidate(&self, path: &syn::Path) -> bool {
        let Some(first) = path.segments.first() else {
            return false;
        };
        path.segments
            .last()
            .is_some_and(|part| part.ident == "Range")
            || self
                .resolution
                .environment
                .index
                .imports
                .get(&(self.namespace.clone(), first.ident.to_string()))
                .is_some_and(|binding| binding.last().is_some_and(|name| name == "Range"))
    }

    fn library(&self, path: &syn::Path, tail: &[&str], prelude: bool) -> Result<bool, String> {
        let mut resolved = self.library_name(path, tail, prelude)?;
        if resolved.first().is_some_and(|name| name == "<external>") {
            resolved.remove(0);
        }
        if resolved.len() != 3
            || !matches!(resolved[0].as_str(), "core" | "std")
            || resolved[1] != tail[0]
            || resolved[2] != tail[1]
        {
            return Ok(false);
        }

        self.library_root(&resolved[0])?;
        if path
            .segments
            .iter()
            .take(path.segments.len().saturating_sub(1))
            .any(|part| !matches!(part.arguments, syn::PathArguments::None))
        {
            return Err("Range library qualifier has unsupported generic arguments".into());
        }
        Ok(true)
    }

    fn library_name(
        &self,
        path: &syn::Path,
        tail: &[&str],
        prelude: bool,
    ) -> Result<Namespace, String> {
        let names: Namespace = path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect();
        let first = names.first().ok_or("Empty Range library path")?;
        Ok(if path.leading_colon.is_none() {
            match self.lookup(first)? {
                Some(binding) => binding
                    .into_iter()
                    .chain(names[1..].iter().cloned())
                    .collect(),
                None if prelude && names == [tail[1]] => {
                    vec!["core".into(), tail[0].into(), tail[1].into()]
                }
                None => names,
            }
        } else {
            names
        })
    }

    fn library_root(&self, root: &str) -> Result<(), String> {
        if self.lookup(root)?.is_some_and(|binding| binding != [root]) {
            return Err("Range library root is shadowed".into());
        }
        Ok(())
    }

    fn lookup(&self, name: &str) -> Result<Option<Namespace>, String> {
        let modules: Vec<_> = self
            .resolution
            .environment
            .modules
            .iter()
            .filter(|module| module.namespace == *self.namespace)
            .collect();
        let [module] = modules.as_slice() else {
            return Err("Range spelling lacks one exact source module".into());
        };
        for item in &module.items {
            if builtin_options::type_name(item).as_deref() == Some(name) {
                if let syn::Item::ExternCrate(item) = item
                    && item.ident == name
                    && item.rename.is_none()
                    && matches!(name, "core" | "std")
                {
                    return Ok(Some(vec![name.into()]));
                }
                return Err("Range spelling is shadowed by a local declaration".into());
            }
        }

        let imports = &self.resolution.environment.index.imports;
        if imports
            .keys()
            .any(|(scope, key)| scope == self.namespace && key.starts_with("*::"))
        {
            return Err("Range spelling has unresolved glob imports".into());
        }
        Ok(imports.get(&(self.namespace.clone(), name.into())).cloned())
    }
}

fn argument(path: &syn::Path) -> Result<&syn::Type, String> {
    let Some(last) = path.segments.last() else {
        return Err("Empty Range type path".into());
    };
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return Err("Range/From requires one explicit concrete type argument".into());
    };
    let mut arguments = arguments.args.iter();
    match (arguments.next(), arguments.next()) {
        (Some(syn::GenericArgument::Type(ty)), None) => Ok(ty),
        _ => Err("Range/From requires exactly one concrete type argument".into()),
    }
}

fn signature(signature: &syn::Signature) -> Result<(), String> {
    if signature.ident != "from"
        || signature.inputs.len() != 1
        || !matches!(signature.inputs.first(), Some(syn::FnArg::Typed(_)))
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
    {
        return Err("Range conversion requires the exact concrete from signature".into());
    }
    Ok(())
}

fn install_owner(resolution: &mut Resolution, owner: Owner, receiver: TypeTree) {
    resolution.namespace = owner.namespace;
    resolution.receiver = Some(receiver);
    resolution.range_owner = Some((owner.implementation, owner.parameter));
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn lexical(bytes: &str) -> Resolution {
        let mut census = (*environment::tests::census()).clone();
        census.files[0].bytes = bytes.into();
        census.files[0].digest = concrete_types::digest(bytes.as_bytes());
        let environment =
            Rc::new(Environment::with_options(Arc::new(census), false, Some(&[]), false).unwrap());
        Resolution {
            namespace: environment
                .modules
                .iter()
                .find(|module| module.namespace.is_empty())
                .expect("The source fixture has one exact crate-root module")
                .namespace
                .clone(),
            receiver: None,
            owner_path: None,
            range_value: None,
            range_owner: None,
            trees: BTreeMap::new(),
            environment,
            copy_trees: Default::default(),
            no_drop_trees: Default::default(),
            fresh_owner_source: None,
            array_views: false,
            scalar_aliases: false,
            local_import_chains: false,
            source_requirements: Default::default(),
            copy_owner_source: None,
            scalar_result_source: None,
            copy_conversion_source: None,
            scalar_product_source: None,
            nonzero_product_source: None,
            transport_sources: None,
            generated_declarations: Vec::new(),
            id_reader_constant_chains: Default::default(),
        }
    }

    fn path(text: &str) -> syn::TypePath {
        syn::parse_str(text).unwrap()
    }

    #[test]
    fn concrete_range_paths_require_exact_library_import_and_usize() {
        let resolution = lexical("use core::ops::Range; use core::ops::Range as Window;");
        for text in [
            "Range<usize>",
            "Window<usize>",
            "core::ops::Range<usize>",
            "::core::ops::Range<usize>",
            "std::ops::Range<usize>",
        ] {
            assert!(range_path(&path(text), &resolution).unwrap(), "{text}");
        }
        for text in [
            "Range<u8>",
            "Range<usize, usize>",
            "Range",
            "core::ops::<u8>::Range<usize>",
        ] {
            assert!(range_path(&path(text), &resolution).is_err(), "{text}");
        }
        assert!(!range_path(&path("Span"), &resolution).unwrap());
        assert!(!range_path(&path("Self"), &resolution).unwrap());
    }

    #[test]
    fn local_shadowing_and_globs_never_supply_range_authority() {
        for bytes in [
            "struct Range;",
            "use core::ops::Range; type usize = u8;",
            "mod core {}",
            "use core::ops::*;",
        ] {
            let resolution = lexical(bytes);
            let spelling = if bytes == "mod core {}" {
                "core::ops::Range<usize>"
            } else {
                "Range<usize>"
            };
            assert!(range_path(&path(spelling), &resolution).is_err(), "{bytes}");
        }
        let resolution = lexical("use other::Range;");
        assert!(!range_path(&path("Range<usize>"), &resolution).unwrap());
    }

    #[test]
    fn conversion_signature_does_not_enable_generic_trait_members() {
        assert!(
            signature(&syn::parse_str::<syn::Signature>("fn from(span: Span) -> Self").unwrap())
                .is_ok()
        );
        for text in [
            "fn from<T>(span: T) -> Self",
            "unsafe fn from(span: Span) -> Self",
            "fn from(&self) -> Self",
            "fn other(span: Span) -> Self",
            "fn from(a: Span,b: Span) -> Self",
        ] {
            let parsed: syn::Signature = syn::parse_str(text).unwrap();
            assert!(signature(&parsed).is_err(), "{text}");
        }
    }
}
