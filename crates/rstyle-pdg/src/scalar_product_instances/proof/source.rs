//! Original concrete const-impl and instance declaration ownership.
use super::*;
use crate::nominal_boundary::resolution::FunctionSource;

pub(super) struct Method {
    pub(super) namespace: Namespace,
    digits: u8,
    pub(super) implementation: crate::ByteSpan,
}

impl Method {
    pub(super) fn digits(&self) -> u8 {
        self.digits
    }
}

pub(super) fn verify_method(
    facts: &HirFnFacts,
    environment: &Environment,
    argument: Argument,
) -> Result<Method, String> {
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing scalar product function source file")?;
    let mut candidates = Vec::new();
    for module in &environment.modules {
        if module.path != std::path::Path::new(file) {
            continue;
        }
        let source = environment.source(&module.path)?;
        for item in &module.items {
            if let syn::Item::Impl(item) = item {
                let search = MethodSearch {
                    facts,
                    module,
                    bytes: source.bytes(),
                    argument,
                };
                search.exact_methods(item, &mut candidates)?;
            }
        }
    }
    let [method] = candidates.as_slice() else {
        return Err("Scalar product requires a unique original concrete digit method".into());
    };
    unshadowed_byte(facts, environment, &method.namespace)?;

    Ok(Method {
        namespace: method.namespace.clone(),
        digits: method.digits,
        implementation: method.implementation,
    })
}

struct MethodSearch<'a> {
    facts: &'a HirFnFacts,
    module: &'a census::Module,
    bytes: &'a str,
    argument: Argument,
}

impl MethodSearch<'_> {
    fn exact_methods(
        &self,
        owner: &syn::ItemImpl,
        candidates: &mut Vec<Method>,
    ) -> Result<(), String> {
        let Self {
            facts,
            module,
            bytes,
            argument,
        } = self;
        for member in &owner.items {
            let syn::ImplItem::Fn(method) = member else {
                continue;
            };
            if !(FunctionSource {
                vis: &method.vis,
                sig: &method.sig,
                body: &method.block,
            })
            .exact(bytes, facts)
            {
                continue;
            }
            let digits = concrete_owner(owner, *argument)?;
            verify_owner_declaration(facts, module)?;
            super::super::super::function_attributes(facts, &owner.attrs)?;
            super::super::super::function_attributes(facts, &method.attrs)?;
            verify_signature(&method.sig)?;
            candidates.push(Method {
                namespace: module.namespace.clone(),
                digits,
                implementation: crate::nominal_boundary::resolution::implementation_source_span(
                    bytes, owner,
                ),
            });
        }

        Ok(())
    }
}

fn verify_signature(signature: &syn::Signature) -> Result<(), String> {
    if signature.ident != "parse"
        || signature.receiver().is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.constness.is_none()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
    {
        return Err("Scalar product requires the original static nongeneric parse method".into());
    }

    Ok(())
}

pub(super) fn concrete_owner(owner: &syn::ItemImpl, argument: Argument) -> Result<u8, String> {
    if owner.trait_.is_some()
        || owner.unsafety.is_some()
        || owner.defaultness.is_some()
        || !owner.generics.params.is_empty()
        || owner.generics.where_clause.is_some()
    {
        return Err("Digit source owner must be an original concrete inherent impl".into());
    }

    let syn::Type::Path(path) = owner.self_ty.as_ref() else {
        return Err("Digit source owner lacks its concrete nominal Self".into());
    };
    if path.qself.is_some() || path.path.leading_colon.is_some() || path.path.segments.len() != 1 {
        return Err("Digit source owner cannot use aliases or qualified Self".into());
    }
    let segment = &path.path.segments[0];
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err("Digit source owner requires one literal const argument".into());
    };
    if segment.ident != "ExactlyNDigits"
        || arguments.args.len() != 1
        || arguments.colon2_token.is_some()
    {
        return Err("Digit source owner differs from its exact literal impl".into());
    }
    literal_digits(arguments, argument)
}

fn literal_digits(
    arguments: &syn::AngleBracketedGenericArguments,
    argument: Argument,
) -> Result<u8, String> {
    let Some(syn::GenericArgument::Const(syn::Expr::Lit(literal))) = arguments.args.first() else {
        return Err("Digit source owner const argument is not an independent literal".into());
    };
    let syn::Lit::Int(literal) = &literal.lit else {
        return Err("Digit source owner const argument is not an integer".into());
    };
    let digits = literal
        .base10_parse::<u8>()
        .map_err(|error| error.to_string())?;
    if !literal.suffix().is_empty()
        || !matches!(
            (digits, argument),
            (1 | 2, Argument::U8) | (3, Argument::U16)
        )
    {
        return Err("Digit source owner literal and scalar instance disagree".into());
    }

    Ok(digits)
}

fn verify_owner_declaration(facts: &HirFnFacts, module: &census::Module) -> Result<(), String> {
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    let mut owners = Vec::new();
    for item in &module.items {
        if let syn::Item::Struct(item) = item
            && item.ident == "ExactlyNDigits"
            && configuration.active(&item.attrs)?
        {
            owners.push(item);
        }
    }
    let [owner] = owners.as_slice() else {
        return Err("Digit owner lacks one original active unit declaration".into());
    };
    let Some(syn::GenericParam::Const(parameter)) = owner.generics.params.first() else {
        return Err("Digit owner lacks its original const parameter".into());
    };
    if !matches!(owner.fields, syn::Fields::Unit)
        || owner.generics.params.len() != 1
        || owner.generics.where_clause.is_some()
        || parameter.default.is_some()
        || !parameter.attrs.is_empty()
        || !matches!(&parameter.ty, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("u8"))
    {
        return Err("Digit owner original unit/const declaration disagrees".into());
    }

    declaration_attributes(facts, &owner.attrs)
}

pub(super) struct DeclarationContext<'a> {
    pub(super) facts: &'a HirFnFacts,
    pub(super) resolution: &'a Resolution,
    pub(super) origins: &'a Origins,
}

pub(super) fn verify_declaration(
    context: &DeclarationContext<'_>,
    instance: &Instance,
    key: &Namespace,
) -> Result<syn::Ident, String> {
    let DeclarationContext {
        facts,
        resolution,
        origins,
    } = context;
    let environment = &resolution.environment;
    let declaration = environment
        .index
        .declarations
        .get(key)
        .ok_or("Missing scalar product source declaration")?;
    let census = environment.census();
    let origin = origins
        .declarations
        .iter()
        .find(|origin| origin.nominal == instance.nominal)
        .ok_or("Missing exact scalar product declaration origin")?;
    if origin.source_file != declaration.path.to_string_lossy()
        || origin.declaration_span != declaration.span
        || origin.file_bytes.as_ref()
            != crate::nominal_boundary::resolution::file_bytes(census, &declaration.path)?
        || instance.nominal.source_identity.as_deref() != Some(&census.context.package.package_id)
        || instance.nominal.package_name != census.context.package.name
        || instance.nominal.package_version != census.context.package.version
        || instance.nominal.manifest_digest != census.context.package.manifest_digest
    {
        return Err("Scalar product declaration source/context disagrees".into());
    }
    let syn::Item::Struct(item) =
        &environment.modules[declaration.module_index].items[declaration.item_index]
    else {
        return Err("Scalar product declaration is not an original struct".into());
    };
    declaration_attributes(facts, &item.attrs)?;
    unshadowed_byte(facts, environment, &declaration.namespace)?;
    let Some(syn::GenericParam::Type(parameter)) = item.generics.params.iter().nth(1) else {
        return Err("Scalar product original substitution parameter is missing".into());
    };

    Ok(parameter.ident.clone())
}

fn declaration_attributes(facts: &HirFnFacts, attributes: &[syn::Attribute]) -> Result<(), String> {
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    if !configuration.active(attributes)? {
        return Err("Scalar product selected declaration is inactive".into());
    }
    for attribute in configuration.effective_attributes(attributes)? {
        if attribute.path().is_ident("derive") {
            super::super::super::derive(&attribute)?;
        } else {
            super::super::super::ordinary(&attribute)?;
        }
    }

    Ok(())
}

fn unshadowed_byte(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
) -> Result<(), String> {
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    for module in &environment.modules {
        if !namespace.starts_with(&module.namespace) {
            continue;
        }
        for item in &module.items {
            if configuration.active(crate::conditional_source::item_attributes(item))? {
                byte_binding(item)?;
            }
        }
    }

    Ok(())
}

pub(super) fn byte_binding(item: &syn::Item) -> Result<(), String> {
    let name = match item {
        syn::Item::Struct(item) => Some(&item.ident),
        syn::Item::Enum(item) => Some(&item.ident),
        syn::Item::Union(item) => Some(&item.ident),
        syn::Item::Type(item) => Some(&item.ident),
        syn::Item::Trait(item) => Some(&item.ident),
        syn::Item::Mod(item) => Some(&item.ident),
        syn::Item::ExternCrate(item) => {
            Some(item.rename.as_ref().map_or(&item.ident, |(_, name)| name))
        }
        _ => None,
    };
    let primitive = |name: &str| matches!(name, "u8" | "u16");
    let import = matches!(item, syn::Item::Use(item)
        if source_imports::has_glob(&item.tree) || source_imports::bindings(&item.tree).iter().any(|(name, _)| primitive(name)));
    if name.is_some_and(|name| primitive(&name.to_string())) || import {
        return Err("Scalar product primitive has an unresolved lexical binding".into());
    }

    Ok(())
}
