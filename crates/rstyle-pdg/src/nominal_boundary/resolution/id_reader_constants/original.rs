//! Original owner and size_of wrapper, selected from the checked module ASTs.
use super::*;
use crate::id_reader_constants::source;

pub(super) fn verify(authority: &SourceAuthority<'_>, record: &Observation) -> Result<(), String> {
    let size = declaration(
        authority.resolution,
        &record.size_argument,
        &record.size_argument_source,
    )?;
    source::transparent_u32(size)?;
    let owner = declaration(authority.resolution, &record.owner, &record.owner_source)?;
    wrapper(owner, terminal(&record.size_argument)?)?;

    let (implementation, constant) = original_constant(authority, record)?;
    let name = terminal(&record.size_argument)?;
    source::implementation(implementation, name)?;
    source::original(constant, name)?;
    let mut namespace = names(&record.size_argument)?;
    namespace.pop();
    let scoped = Scoped {
        resolution: authority.resolution,
        namespace: &namespace,
    };
    let syn::Type::Path(path) = implementation.self_ty.as_ref() else {
        return Err("ID size original implementation lacks its lexical owner".into());
    };
    nominal(scoped.resolve(path)?, &record.size_argument)?;
    core_source(authority, record, &scoped)
}

fn core_source(
    authority: &SourceAuthority<'_>,
    record: &Observation,
    scoped: &Scoped<'_>,
) -> Result<(), String> {
    let facts = authority.input.facts();
    let key = names(&record.size_argument)?;
    let tree = authority
        .resolution
        .trees
        .get(&key)
        .ok_or("ID size wrapper is outside its exact collected source closure")?;
    let origins = facts
        .build
        .frontend_options
        .data::<declaration_origins::Origins>()?;

    let certificate =
        selected::no_drop::copy_products::verify(facts, tree, authority.resolution, &origins)?;
    selected::source_library_root(facts, scoped, "core", &certificate)
}

pub(super) fn nominal(tree: TypeTree, definition: &Definition) -> Result<(), String> {
    let TypeTree::Struct { nominal, .. } = tree else {
        return Err("ID byte width requires its complete original struct owner".into());
    };
    if nominal.definition_path != definition.path {
        return Err("ID byte width selects another lexical nominal owner".into());
    }
    Ok(())
}

pub(super) fn declaration<'a>(
    resolution: &'a Resolution,
    definition: &Definition,
    site: &Site,
) -> Result<&'a syn::ItemStruct, String> {
    let key = names(definition)?;
    let observed = resolution
        .environment
        .index
        .declarations
        .get(&key)
        .ok_or("ID byte width lacks its checked original struct declaration")?;
    if observed.path != std::path::Path::new(&site.file) || observed.span != site.span {
        return Err("ID byte width declaration differs from its exact original owner span".into());
    }

    let tree = resolution
        .trees
        .get(&key)
        .ok_or("ID byte width owner is outside the complete nominal source closure")?;
    let nominal = tree
        .nominal_identity()
        .ok_or("ID byte width owner lacks nominal identity")?;
    if !resolution.checked_owned_copy(tree) || !resolution.checked_no_drop(nominal) {
        return Err(
            "ID byte width owner lacks complete checked Copy and no-drop source authority".into(),
        );
    }
    resolution.check_nominal(&key, nominal)?;
    let syn::Item::Struct(item) =
        &resolution.environment.modules[observed.module_index].items[observed.item_index]
    else {
        return Err("ID byte width original owner is not a struct".into());
    };
    Ok(item)
}

fn wrapper(item: &syn::ItemStruct, inner: &str) -> Result<(), String> {
    let syn::Fields::Unnamed(fields) = &item.fields else {
        return Err("ID byte width owner requires one original wrapper field".into());
    };
    if !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || fields.unnamed.len() != 1
        || !matches!(&fields.unnamed[0].ty, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident(inner))
    {
        return Err("ID byte width original wrapper payload disagrees".into());
    }
    Ok(())
}

fn original_constant<'a>(
    authority: &'a SourceAuthority<'_>,
    record: &Observation,
) -> Result<(&'a syn::ItemImpl, &'a syn::ImplItemConst), String> {
    let environment = &authority.resolution.environment;
    let mut found = Vec::new();
    for module in &environment.modules {
        if module.path != std::path::Path::new(&record.original_implementation_source.file) {
            continue;
        }
        let bytes = environment.source(&module.path)?.bytes();
        for item in &module.items {
            let syn::Item::Impl(item) = item else {
                continue;
            };
            let span = ByteSpan {
                start: source_span(bytes, item.impl_token.span).start,
                end: source_span(bytes, item.brace_token.span.close()).end,
            };
            if span != record.original_implementation_source.span {
                continue;
            }
            if record.original_source.file == record.original_implementation_source.file {
                found.push((item, constant(item, bytes, &record.original_source)?));
            }
        }
    }
    one(
        found.into_iter(),
        "exact original ID size constant and inherent impl",
    )
}

fn constant<'a>(
    implementation: &'a syn::ItemImpl,
    bytes: &str,
    site: &Site,
) -> Result<&'a syn::ImplItemConst, String> {
    one(
        implementation.items.iter().filter_map(|member| {
            let syn::ImplItem::Const(member) = member else {
                return None;
            };
            let start = if matches!(member.vis, syn::Visibility::Inherited) {
                member.const_token.span
            } else {
                member.vis.span()
            };
            let span = ByteSpan {
                start: source_span(bytes, start).start,
                end: source_span(bytes, member.semi_token.span).end,
            };
            (span == site.span).then_some(member)
        }),
        "original ID size member",
    )
}
