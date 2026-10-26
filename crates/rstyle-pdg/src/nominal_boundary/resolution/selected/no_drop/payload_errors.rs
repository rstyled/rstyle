//! Complete non-Copy error cleanup; source evidence grants no value or Copy authority.
use super::*;
use crate::concrete_types::{Field, VariantShape};

/// Select a complete error family and, when separately activated, its exact
/// success product. Every collected dependency receives its own proof.
pub(in crate::nominal_boundary::resolution) fn selection(
    facts: &HirFnFacts,
    resolution: &Resolution,
) -> Result<Option<super::super::Selection>, String> {
    if !supports(facts, crate::named_error_payloads::CAPABILITY)? {
        return Ok(None);
    }
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured { ty } = &interface.result else {
        return Ok(None);
    };
    let product = reader_product(facts, ty)?;
    let id_product = id_reader_product(facts, ty)?;
    let error = match ty {
        TypeTree::Result { ok, err }
            if product.is_some()
                || id_product.is_some()
                || matches!(ok.as_ref(), TypeTree::Scalar(value) if value == "usize")
                || supports(facts, crate::named_error_result_propagation::CAPABILITY)?
                    && crate::named_error_result_propagation::success(
                        &crate::data_types::TypeToken::from_tree(ok),
                    ) =>
        {
            err.as_ref()
        }
        TypeTree::Struct { fields, .. } if matches!(fields.as_slice(), [field] if matches!(field.ty, TypeTree::MixedEnum { .. })) => {
            ty
        }
        _ => return Ok(None),
    };
    if !matches!(error, TypeTree::Struct { fields, .. } if matches!(fields.as_slice(), [field] if field.name == "0" && matches!(field.ty, TypeTree::MixedEnum { .. })))
    {
        return Ok(None);
    }
    if !supported(error) {
        return Err("Named error requires its complete cleanup grammar".into());
    }
    verify(facts, resolution, error)?;

    Ok(Some(select_dependencies(
        facts, resolution, product, id_product,
    )?))
}

fn select_dependencies(
    facts: &HirFnFacts,
    resolution: &Resolution,
    product: Option<&TypeTree>,
    id_product: Option<&TypeTree>,
) -> Result<super::super::Selection, String> {
    let mut selection = super::super::Selection::default();
    for (key, tree) in &resolution.trees {
        if product == Some(tree) || id_dependency(id_product, tree) {
            let origins = facts.build.frontend_options.data::<Origins>()?;
            copy_products::verify(facts, tree, resolution, &origins)?;
            selection.copies.insert(key.clone());
        } else {
            verify(facts, resolution, tree)?;
        }
        selection.no_drop.insert(key.clone());
    }
    Ok(selection)
}

fn id_dependency(product: Option<&TypeTree>, tree: &TypeTree) -> bool {
    let Some(root @ TypeTree::Struct { fields, .. }) = product else {
        return false;
    };
    root == tree || matches!(fields.as_slice(), [field] if &field.ty == tree)
}

fn id_reader_product<'a>(
    facts: &HirFnFacts,
    result: &'a TypeTree,
) -> Result<Option<&'a TypeTree>, String> {
    if !supports(facts, crate::id_reader_types::CAPABILITY)? {
        return Ok(None);
    }
    let TypeTree::Result { ok, .. } = result else {
        return Ok(None);
    };
    Ok(crate::id_reader_types::product(ok))
}

pub(in crate::nominal_boundary::resolution) fn function_namespace(
    facts: &HirFnFacts,
    resolution: &Resolution,
    namespace: &[String],
) -> Result<bool, String> {
    if !supports(facts, crate::named_error_payloads::CAPABILITY)? {
        return Ok(false);
    }
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured { ty } = &interface.result else {
        return Ok(false);
    };
    let error = match ty {
        TypeTree::Result { err, .. } => err.as_ref(),
        error => error,
    };
    if !matches!(error, TypeTree::Struct { fields, .. } if matches!(fields.as_slice(), [field] if matches!(field.ty, TypeTree::MixedEnum { .. })))
    {
        return Ok(false);
    }

    let certificate = verify(facts, resolution, error)?;
    if let Some(product) = reader_product(facts, ty)? {
        return product_namespace(facts, resolution, namespace, product);
    }
    let mut owner: Namespace = certificate
        .nominal()
        .definition_path
        .split("::")
        .map(str::to_owned)
        .collect();
    owner.pop();
    if owner != namespace {
        return Err("Fresh named error callable is outside its checked cleanup namespace".into());
    }
    super::super::check_namespace_with_closure(
        facts,
        &resolution.environment,
        namespace,
        Some(&certificate),
    )?;
    Ok(true)
}

fn reader_product<'a>(
    facts: &HirFnFacts,
    result: &'a TypeTree,
) -> Result<Option<&'a TypeTree>, String> {
    if !supports(facts, crate::reader_products::CAPABILITY)? {
        return Ok(None);
    }
    let TypeTree::Result { ok, .. } = result else {
        return Ok(None);
    };
    if !crate::reader_products::shape(&crate::data_types::TypeToken::from_tree(ok)) {
        return Ok(None);
    }
    let TypeTree::Tuple(values) = ok.as_ref() else {
        return Ok(None);
    };

    Ok(match values.as_slice() {
        [product @ TypeTree::Struct { .. }, TypeTree::Scalar(size)] if size == "usize" => {
            Some(product)
        }
        _ => None,
    })
}

fn product_namespace(
    facts: &HirFnFacts,
    resolution: &Resolution,
    namespace: &[String],
    product: &TypeTree,
) -> Result<bool, String> {
    let observation = copy_owners::source::observation(facts)?
        .ok_or("Reader product requires its exact original inherent observation")?;
    let owner = copy_owners::source::resolve(facts, resolution, &observation)?
        .ok_or("Reader product requires its independently resolved inherent owner")?;
    if owner.root != *product || owner.namespace != namespace {
        return Err("Reader product callable differs from its exact original success owner".into());
    }

    let origins = facts.build.frontend_options.data::<Origins>()?;
    let certificate = copy_products::verify(facts, product, resolution, &origins)?;
    super::super::check_namespace_with_closure(
        facts,
        &resolution.environment,
        namespace,
        Some(&certificate),
    )?;
    Ok(true)
}

/// The caller separately gates this source family and proves every lexical field.
/// Shared strings require explicit static spelling, checked during verification.
pub(in crate::nominal_boundary::resolution) fn supported(tree: &TypeTree) -> bool {
    shape(tree, 0)
}

fn shape(tree: &TypeTree, depth: usize) -> bool {
    if depth > concrete_types::MAX_DEPTH {
        return false;
    }
    match tree {
        TypeTree::Struct { fields, .. } if scalar_errors::supported(tree) => !fields.is_empty(),
        TypeTree::Struct { fields, .. } => {
            matches!(fields.as_slice(), [field] if field.name == "0"
                && matches!(field.ty, TypeTree::MixedEnum { .. }) && shape(&field.ty, depth + 1))
        }
        TypeTree::MixedEnum { variants, .. } => {
            !variants.is_empty()
                && variants.len() <= concrete_types::MAX_VARIANTS
                && variants.iter().all(|variant| match &variant.shape {
                    VariantShape::Named { fields } => {
                        !fields.is_empty()
                            && fields.len() <= concrete_types::MAX_FIELDS
                            && fields
                                .iter()
                                .all(|(_, field)| payload(&field.ty, depth + 1))
                    }
                    _ => false,
                })
        }
        TypeTree::GeneratedStruct { fields, .. } => {
            matches!(fields.as_slice(), [field] if field.name == "0"
                && scalar_errors::supported(&field.ty) && shape(&field.ty, depth + 1))
        }
        _ => false,
    }
}

fn payload(tree: &TypeTree, depth: usize) -> bool {
    match tree {
        TypeTree::Scalar(token) => concrete_types::scalar(token) && token != "unit",
        TypeTree::Shared(referent) => referent.as_ref() == &TypeTree::Str,
        TypeTree::GeneratedStruct { .. } => shape(tree, depth),
        _ => false,
    }
}

/// `verify_declarations` must already have proved all original/generated source
/// and scoped field spelling against this exact collected interface. Rechecking
/// resolution here would recursively initialize permission selection.
pub(in crate::nominal_boundary::resolution) fn verify(
    facts: &HirFnFacts,
    resolution: &Resolution,
    tree: &TypeTree,
) -> Result<CheckedNoDrop, String> {
    concrete_types::validate(tree)?;
    if !supported(tree) {
        return Err("Payload error cleanup requires its entire bounded nominal tree".into());
    }

    resolution.environment.check_context(facts)?;
    pinned(facts)?;
    let mut checked = std::collections::BTreeSet::new();
    visit(facts, resolution, tree, &mut checked)?;

    let nominal = tree
        .nominal_identity()
        .ok_or("Payload error cleanup lacks exact nominal identity")?;
    Ok(CheckedNoDrop {
        nominal: nominal.clone(),
    })
}

fn pinned(facts: &HirFnFacts) -> Result<(), String> {
    crate::data_library::verify_sysroot_roots(
        facts,
        &["alloc", "core", "std"],
        "Payload error cleanup source",
    )?;
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Payload error cleanup requires pinned compiler/target")?;
    if !library_macros::pinned_native(registry) {
        return Err("Payload error cleanup macro pins are incomplete".into());
    }
    Ok(())
}

fn visit(
    facts: &HirFnFacts,
    resolution: &Resolution,
    tree: &TypeTree,
    checked: &mut std::collections::BTreeSet<Namespace>,
) -> Result<(), String> {
    let key = collected(resolution, tree)?;
    if checked.contains(&key) {
        return Ok(());
    }

    if matches!(tree, TypeTree::GeneratedStruct { .. }) {
        generated(facts, resolution, tree, &key)?;
    } else {
        original(facts, resolution, tree, &key)?;
    }
    for field in tree.nominal_fields() {
        static_string(field)?;
        if field.ty.nominal_identity().is_some() {
            visit(facts, resolution, &field.ty, checked)?;
        }
    }

    super::verify_source(facts, &resolution.environment, &key, true)?;
    checked.insert(key);
    Ok(())
}

fn collected(resolution: &Resolution, tree: &TypeTree) -> Result<Namespace, String> {
    let nominal = tree
        .nominal_identity()
        .ok_or("Payload error cleanup dependency lacks nominal identity")?;
    let key = nominal
        .definition_path
        .split("::")
        .map(str::to_owned)
        .collect();
    if resolution.trees.get(&key) != Some(tree) {
        return Err("Payload error cleanup differs from exact collected nominal tree".into());
    }
    Ok(key)
}

fn original(
    facts: &HirFnFacts,
    resolution: &Resolution,
    tree: &TypeTree,
    key: &Namespace,
) -> Result<(), String> {
    let environment = &resolution.environment;
    let declaration = environment
        .index
        .declarations
        .get(key)
        .ok_or("Payload error lacks an exact original declaration")?;
    let nominal = tree
        .nominal_identity()
        .ok_or("Missing payload error nominal")?;
    if !environment.declaration_matches(declaration, &nominal.declaration_digest)? {
        return Err("Payload error original declaration identity disagrees".into());
    }
    let item = &environment.modules[declaration.module_index].items[declaration.item_index];

    match (item, tree) {
        (syn::Item::Struct(item), TypeTree::Struct { .. })
            if item.generics.params.is_empty() && item.generics.where_clause.is_none() =>
        {
            super::super::structure(facts, item, false)
        }
        (syn::Item::Enum(item), TypeTree::MixedEnum { .. })
            if item.generics.params.is_empty() && item.generics.where_clause.is_none() =>
        {
            super::super::scalar_enum(facts, item)
        }
        _ => Err("Payload error original declaration has a different nominal kind".into()),
    }
}

fn generated(
    facts: &HirFnFacts,
    resolution: &Resolution,
    tree: &TypeTree,
    key: &Namespace,
) -> Result<(), String> {
    let declaration = resolution
        .generated_declaration(tree)
        .ok_or("Payload error generated child lacks independently checked source")?;
    if declaration.tree() != tree
        || declaration
            .namespace()
            .iter()
            .cloned()
            .chain([declaration.item().ident.to_string()])
            .collect::<Vec<_>>()
            != *key
    {
        return Err("Payload error generated child belongs to another source namespace".into());
    }
    super::super::structure(facts, declaration.item(), false)
}

fn static_string(field: &Field) -> Result<(), String> {
    if !matches!(&field.ty, TypeTree::Shared(referent) if referent.as_ref() == &TypeTree::Str) {
        return Ok(());
    }
    let source: syn::Type =
        syn::parse_str(&field.declaration).map_err(|error| error.to_string())?;
    if !matches!(source, syn::Type::Reference(reference)
        if reference.mutability.is_none()
            && reference.lifetime.as_ref().is_some_and(|lifetime| lifetime.ident == "static")
            && matches!(reference.elem.as_ref(), syn::Type::Path(path)
                if path.qself.is_none() && path.path.is_ident("str")))
    {
        return Err("Payload error shared strings require exact explicit static source".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
