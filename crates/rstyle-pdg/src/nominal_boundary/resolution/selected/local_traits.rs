//! Original concrete local trait-member ownership, never trait-call authority.
use super::*;

struct Source<'a> {
    facts: &'a HirFnFacts,
    environment: &'a Environment,
    configuration: crate::conditional_source::Configuration<'a>,
}

/// Only exact ordinary source implementations with a shared concrete scalar
/// owner can use this route. Inherent helper inventories remain unchanged.
pub(in crate::nominal_boundary::resolution) fn implementation(
    facts: &HirFnFacts,
    environment: &Environment,
    module: &census::Module,
    item: &syn::ItemImpl,
) -> Result<bool, String> {
    if item.trait_.is_none() || !supports(facts, core_imports::CAPABILITY)? {
        return Ok(false);
    }
    let bytes = environment.source(&module.path)?.bytes();
    let Some(method) = exact_member(item, bytes, facts) else {
        return Ok(false);
    };
    let source = Source {
        facts,
        environment,
        configuration: crate::conditional_source::Configuration::new(&facts.build.cfgs),
    };

    source.implementation(module, item, method)?;
    Ok(true)
}

fn exact_member<'a>(
    item: &'a syn::ItemImpl,
    bytes: &str,
    facts: &HirFnFacts,
) -> Option<&'a syn::ImplItemFn> {
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

impl Source<'_> {
    fn implementation(
        &self,
        module: &census::Module,
        item: &syn::ItemImpl,
        method: &syn::ImplItemFn,
    ) -> Result<(), String> {
        if item.unsafety.is_some()
            || item.defaultness.is_some()
            || !item.generics.params.is_empty()
            || item.generics.where_clause.is_some()
            || method.defaultness.is_some()
        {
            return Err("Local trait member requires a concrete ordinary implementation".into());
        }

        self.active(&item.attrs)?;
        self.active(&method.attrs)?;
        member_signature(&method.sig)?;
        self.scalar_owner(module, &item.self_ty)?;
        let Some((None, path, _)) = &item.trait_ else {
            return Err("Negative local trait implementation is unsupported".into());
        };
        let key = self.resolve(&module.namespace, path)?;
        self.declaration(&key, &method.sig.ident.to_string())
    }
    fn active(&self, attributes: &[syn::Attribute]) -> Result<(), String> {
        if !self.configuration.active(attributes)? {
            return Err("Local trait source owner is inactive".into());
        }
        function_attributes(self.facts, attributes)
    }
    fn resolve(&self, namespace: &Namespace, path: &syn::Path) -> Result<Namespace, String> {
        if path.leading_colon.is_some()
            || path
                .segments
                .iter()
                .any(|part| !matches!(part.arguments, syn::PathArguments::None))
        {
            return Err("Local trait path requires a direct nongeneric source binding".into());
        }
        let names: Namespace = path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect();
        let first = names.first().ok_or("Empty local trait path")?;

        if let Some(target) = self
            .environment
            .index
            .imports
            .get(&(namespace.clone(), first.clone()))
        {
            return Ok(target.iter().chain(names[1..].iter()).cloned().collect());
        }
        anchored(names, namespace, false)
    }
    fn scalar_owner(&self, module: &census::Module, ty: &syn::Type) -> Result<(), String> {
        let syn::Type::Path(path) = ty else {
            return Err("Local trait owner requires an original nominal type".into());
        };
        if path.qself.is_some() {
            return Err("Associated local trait owner is unsupported".into());
        }
        let key = self.resolve(&module.namespace, &path.path)?;
        let declaration = self
            .environment
            .index
            .declarations
            .get(&key)
            .ok_or("Local trait owner lacks an original declaration")?;
        let syn::Item::Struct(item) =
            &self.environment.modules[declaration.module_index].items[declaration.item_index]
        else {
            return Err("Local trait owner must be an original struct".into());
        };

        if !self.configuration.active(&item.attrs)? {
            return Err("Local trait owner declaration is inactive".into());
        }
        nominal_attributes(
            self.facts,
            &item.generics,
            &item.attrs,
            AttributeOwner::Struct { transparent: false },
        )?;
        for field in &item.fields {
            self.active(&field.attrs)?;
        }
        scalar_structure(item)
    }
    fn declaration(&self, key: &[String], member: &str) -> Result<(), String> {
        let (name, namespace) = key.split_last().ok_or("Empty local trait identity")?;
        let modules: Vec<_> = self
            .environment
            .modules
            .iter()
            .filter(|module| module.namespace == namespace)
            .collect();
        let [module] = modules.as_slice() else {
            return Err("Local trait lacks one original source module".into());
        };
        let item = direct_trait(&module.items, name)?;

        self.active(&item.attrs)?;
        trait_declaration(item, member, &self.configuration, self.facts)?;
        self.bounds(module, item)
    }
    fn bounds(&self, module: &census::Module, item: &syn::ItemTrait) -> Result<(), String> {
        for bound in &item.supertraits {
            match bound {
                syn::TypeParamBound::Lifetime(lifetime) if lifetime.ident == "static" => {}
                syn::TypeParamBound::Trait(bound)
                    if matches!(bound.modifier, syn::TraitBoundModifier::None)
                        && bound.lifetimes.is_none() =>
                {
                    self.bound(module, &bound.path)?;
                }
                _ => return Err("Local trait has an unsupported source supertrait bound".into()),
            }
        }
        Ok(())
    }
    fn bound(&self, module: &census::Module, path: &syn::Path) -> Result<(), String> {
        let key = self.resolve(&module.namespace, path)?;
        if key == ["core", "fmt", "Debug"]
            && core_imports::debug(
                self.facts,
                self.environment,
                (&module.namespace, "Debug", &key),
            )?
        {
            return Ok(());
        }
        if key == ["core", "panic", "RefUnwindSafe"] || key == ["core", "panic", "UnwindSafe"] {
            return imports::library_root(self.facts, self.environment, &module.namespace, "core");
        }
        if path.is_ident("Send") || path.is_ident("Sync") {
            self.prelude_marker(module, path)?;
            return imports::library_root(self.facts, self.environment, &module.namespace, "core");
        }

        Err("Local trait supertrait lacks reviewed source identity".into())
    }
    fn prelude_marker(&self, module: &census::Module, path: &syn::Path) -> Result<(), String> {
        let name = path
            .get_ident()
            .ok_or("Missing marker trait name")?
            .to_string();
        for length in 0..=module.namespace.len() {
            let scope = &module.namespace[..length];
            self.marker_imports(scope, &name)?;
            self.marker_declarations(scope, &name)?;
        }
        Ok(())
    }
    fn marker_imports(&self, scope: &[String], name: &str) -> Result<(), String> {
        for ((namespace, binding), target) in &self.environment.index.imports {
            if namespace != scope {
                continue;
            }
            if binding == name {
                return Err("Local trait prelude marker has an unresolved import binding".into());
            }
            if binding.starts_with("*::") {
                imports::glob_excludes(self.environment, target, name, &self.configuration)?;
            }
        }
        Ok(())
    }

    fn marker_declarations(&self, scope: &[String], name: &str) -> Result<(), String> {
        for owner in self
            .environment
            .modules
            .iter()
            .filter(|owner| owner.namespace == scope)
        {
            for item in &owner.items {
                if super::super::builtin_options::type_name(item).as_deref() == Some(name) {
                    return Err("Local trait prelude marker is shadowed".into());
                }
            }
        }
        Ok(())
    }
}

fn scalar_structure(item: &syn::ItemStruct) -> Result<(), String> {
    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() || item.fields.is_empty()
        || item.fields.len() > concrete_types::MAX_FIELDS || !item.fields.iter().all(|field| {
            matches!(&field.ty, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("u8"))
        })
    { return Err("Local trait owner requires a complete nongeneric byte-field struct".into()); }
    Ok(())
}

fn member_signature(signature: &syn::Signature) -> Result<(), String> {
    if signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || !matches!(signature.inputs.first(), Some(syn::FnArg::Receiver(receiver)) if receiver.reference.is_some() && receiver.mutability.is_none() && receiver.colon_token.is_none())
    {
        return Err("Local trait member requires an ordinary nongeneric shared receiver".into());
    }
    for input in &signature.inputs {
        if let syn::FnArg::Typed(input) = input {
            declaration_type(&input.ty)?;
        }
    }
    if let syn::ReturnType::Type(_, result) = &signature.output {
        declaration_type(result)?;
    }
    Ok(())
}

fn declaration_type(ty: &syn::Type) -> Result<(), String> {
    match ty {
        syn::Type::Path(path) if path.qself.is_none() => {
            for segment in &path.path.segments {
                match &segment.arguments {
                    syn::PathArguments::None => {}
                    syn::PathArguments::AngleBracketed(arguments) => {
                        for argument in &arguments.args {
                            let syn::GenericArgument::Type(ty) = argument else {
                                return Err(
                                    "Local trait signature type arguments must be ordinary types"
                                        .into(),
                                );
                            };
                            declaration_type(ty)?;
                        }
                    }
                    _ => {
                        return Err(
                            "Local trait signature has an unsupported type application".into()
                        );
                    }
                }
            }
            Ok(())
        }
        syn::Type::Reference(reference) if reference.mutability.is_none() => {
            declaration_type(&reference.elem)
        }
        syn::Type::Slice(slice) => declaration_type(&slice.elem),
        syn::Type::Tuple(tuple) if tuple.elems.is_empty() => Ok(()),
        _ => Err("Local trait signature has opaque or executable type syntax".into()),
    }
}

fn trait_declaration(
    item: &syn::ItemTrait,
    member: &str,
    configuration: &crate::conditional_source::Configuration<'_>,
    facts: &HirFnFacts,
) -> Result<(), String> {
    trait_header(item)?;
    let mut selected = 0;
    for entry in &item.items {
        let syn::TraitItem::Fn(method) = entry else {
            return Err("Local trait associated nonfunction item is unsupported".into());
        };
        if !configuration.active(&method.attrs)? {
            continue;
        }
        required_method(method)?;
        function_attributes(facts, &method.attrs)?;
        selected += usize::from(method.sig.ident == member);
    }

    if selected != 1 {
        return Err("Local trait selected member is missing or ambiguous".into());
    }
    Ok(())
}

fn trait_header(item: &syn::ItemTrait) -> Result<(), String> {
    if item.unsafety.is_some()
        || item.auto_token.is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
    {
        return Err("Local trait declaration requires an ordinary nongeneric trait".into());
    }
    Ok(())
}

fn required_method(method: &syn::TraitItemFn) -> Result<(), String> {
    if method.default.is_some() {
        return Err("Local trait default method bodies require separate authority".into());
    }
    member_signature(&method.sig)
}

fn direct_trait<'a>(items: &'a [syn::Item], name: &str) -> Result<&'a syn::ItemTrait, String> {
    if items
        .iter()
        .filter(|item| super::super::builtin_options::type_name(item).as_deref() == Some(name))
        .count()
        != 1
    {
        return Err("Local trait name has ambiguous original source declarations".into());
    }
    items
        .iter()
        .find_map(|item| match item {
            syn::Item::Trait(item) if item.ident == name => Some(item),
            _ => None,
        })
        .ok_or_else(|| "Local trait lacks one direct original declaration".into())
}

#[cfg(test)]
mod tests;
