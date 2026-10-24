//! Exact bounded declaration shapes and source-spelled static reference leaves.
use super::*;
use crate::concrete_types::{Field, VariantShape};

pub(super) fn verify(item: &syn::Item, tree: &TypeTree) -> Result<(), String> {
    match (item, tree) {
        (syn::Item::Struct(item), TypeTree::Struct { fields, .. }) => {
            nongeneric(&item.generics)?;
            fields_source(&item.fields, fields.iter())
        }
        (syn::Item::Enum(item), TypeTree::MixedEnum { variants, .. }) => {
            nongeneric(&item.generics)?;
            if item.variants.len() != variants.len() {
                return Err("Copy enum requires its complete original variant inventory".into());
            }
            for (position, (source, observed)) in item.variants.iter().zip(variants).enumerate() {
                if observed.position != position
                    || source.ident != observed.name
                    || source.discriminant.is_some()
                {
                    return Err("Copy enum variant identity/order/discriminant disagrees".into());
                }
                variant(&source.fields, &observed.shape)?;
            }
            Ok(())
        }
        _ => Err("Copy component declaration and observed nominal kinds disagree".into()),
    }
}

fn nongeneric(generics: &syn::Generics) -> Result<(), String> {
    if !generics.params.is_empty() || generics.where_clause.is_some() {
        return Err("Copy components require concrete nongeneric declarations".into());
    }
    Ok(())
}

fn variant(fields: &syn::Fields, shape: &VariantShape<Field>) -> Result<(), String> {
    match (fields, shape) {
        (syn::Fields::Unit, VariantShape::Unit {}) => Ok(()),
        (syn::Fields::Unnamed(source), VariantShape::SingleTuple { field })
            if source.unnamed.len() == 1 =>
        {
            fields_source(fields, [field].into_iter())
        }
        (syn::Fields::Named(_), VariantShape::Named { fields: observed }) => {
            if observed.iter().any(|(name, field)| name != &field.name) {
                return Err("Copy enum named component identity disagrees".into());
            }
            fields_source(fields, observed.iter().map(|(_, field)| field))
        }
        _ => Err("Copy enum original payload shape disagrees".into()),
    }
}

fn fields_source<'a>(
    source: &syn::Fields,
    observed: impl ExactSizeIterator<Item = &'a Field>,
) -> Result<(), String> {
    if source.len() != observed.len() {
        return Err("Copy component original field inventory disagrees".into());
    }
    for (position, (source, observed)) in source.iter().zip(observed).enumerate() {
        let name = source
            .ident
            .as_ref()
            .map_or_else(|| position.to_string(), ToString::to_string);
        if observed.position != position || observed.name != name {
            return Err("Copy component original field order/name disagrees".into());
        }
        field_type(&source.ty, &observed.ty)?;
    }
    Ok(())
}

fn field_type(source: &syn::Type, tree: &TypeTree) -> Result<(), String> {
    match (source, tree) {
        (syn::Type::Paren(source), _) => field_type(&source.elem, tree),
        (syn::Type::Group(source), _) => field_type(&source.elem, tree),
        (syn::Type::Reference(source), TypeTree::Shared(referent))
            if referent.as_ref() == &TypeTree::Str =>
        {
            static_str(source)
        }
        (syn::Type::Path(source), TypeTree::Scalar(token))
            if source.qself.is_none() && source.path.is_ident(token) =>
        {
            Ok(())
        }
        (syn::Type::Path(source), tree)
            if source.qself.is_none()
                && tree.nominal_parts().is_some()
                && source
                    .path
                    .segments
                    .iter()
                    .all(|segment| matches!(segment.arguments, syn::PathArguments::None)) =>
        {
            // Exact path-to-nominal correspondence is independently checked by
            // the supplied declaration resolution; this grants no alias proof.
            Ok(())
        }
        _ => Err("Copy component field lies outside its bounded source grammar".into()),
    }
}

pub(in crate::nominal_boundary::resolution::selected::no_drop) fn static_str(
    reference: &syn::TypeReference,
) -> Result<(), String> {
    if reference.mutability.is_some()
        || reference
            .lifetime
            .as_ref()
            .is_none_or(|lifetime| lifetime.ident != "static")
        || !matches!(reference.elem.as_ref(), syn::Type::Path(path)
            if path.qself.is_none() && path.path.is_ident("str"))
    {
        return Err(
            "Copy string component requires its explicit original static shared str".into(),
        );
    }
    Ok(())
}

pub(super) fn interface(tree: &TypeTree, spelling: &str) -> Result<(), String> {
    if matches!(tree, TypeTree::Shared(_)) {
        let source = syn::parse_str::<syn::Type>(spelling).map_err(|error| error.to_string())?;
        field_type(&source, tree)?;
    }
    Ok(())
}

/// This first activation takes supplied string backing only from explicitly
/// static formals. Erased Shared(Str) observations cannot prove that lifetime.
pub(super) fn parameters(
    function: &syn::ItemFn,
    interface: &InterfaceObservations,
) -> Result<(), String> {
    if function.sig.inputs.len() != interface.parameters.len() {
        return Err("Static product original parameter inventory disagrees".into());
    }
    for (parameter, observed) in function.sig.inputs.iter().zip(&interface.parameters) {
        let Observation::Captured { ty } = observed else {
            return Err("Static product requires complete original parameter types".into());
        };
        if matches!(ty, TypeTree::Shared(inner) if inner.as_ref() == &TypeTree::Str) {
            let syn::FnArg::Typed(parameter) = parameter else {
                return Err(
                    "Static string product requires an original typed string formal".into(),
                );
            };
            field_type(&parameter.ty, ty)?;
        } else if wrapped_string(ty) {
            return Err(
                "Static product string formals require direct static or checked nominal backing"
                    .into(),
            );
        }
    }
    Ok(())
}

fn wrapped_string(tree: &TypeTree) -> bool {
    match tree {
        // Nominal leaves have their own exact original field/Copy proof. A
        // receiver borrow does not change its independently proved static field.
        TypeTree::Shared(inner) => inner.as_ref() == &TypeTree::Str || wrapped_string(inner),
        TypeTree::Tuple(elements) => elements.iter().any(wrapped_string),
        TypeTree::Option(inner) | TypeTree::Slice(inner) => wrapped_string(inner),
        TypeTree::Array { element, .. } => wrapped_string(element),
        TypeTree::Result { ok, err } => wrapped_string(ok) || wrapped_string(err),
        _ => false,
    }
}
