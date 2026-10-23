//! Complete own parameter, predicate and header rosters; no implicit prefix support.
use super::{Bounds, Result, Source};
use crate::ByteSpan;
use crate::reference_instances::provenance::model::{Anchor, Observation, Owner, Parameter};
use syn::spanned::Spanned;

pub(super) fn verify(
    record: &Observation,
    implementation: &syn::ItemImpl,
    function: &syn::ImplItemFn,
    source: &Source<'_>,
) -> Result<Bounds> {
    let from = implementation.trait_.is_some();
    let parent = &record.owners[0];
    let method = &record.owners[1];
    let expected_parent = if from {
        "<util::search::Input<'h> as core::convert::From<&'h H>>"
    } else {
        "util::search::Input<'h>"
    };
    if parent.definition.path != expected_parent {
        return Err("Reference generic owner changes its original source identity".into());
    }
    headers(record, implementation, function, source)?;
    let parent_count = if from { 2 } else { 1 };
    let method_count = usize::from(!from);
    if parent.parameters.len() != parent_count
        || method.parameters.len() != method_count
        || implementation.generics.params.len() != parent_count
        || function.sig.generics.params.len() != method_count
        || implementation.generics.where_clause.is_some()
        || function.sig.generics.where_clause.is_some()
    {
        return Err(
            "Reference own/inherited generic rosters differ or contain extra predicates".into(),
        );
    }

    lifetime(parent, &implementation.generics.params[0], source)?;
    let (owner_index, syntax) = if from {
        (0, &implementation.generics.params[1])
    } else {
        (1, &function.sig.generics.params[0])
    };
    let owner = &record.owners[owner_index];
    let clauses = type_parameter(owner, syntax, source)?;
    if !record.owners[1 - owner_index].source_predicates.is_empty() {
        return Err("Reference source provenance invents predicates on the other owner".into());
    }
    Ok(Bounds {
        owner: owner_index,
        clauses,
    })
}

fn headers(
    record: &Observation,
    implementation: &syn::ItemImpl,
    function: &syn::ImplItemFn,
    source: &Source<'_>,
) -> Result<()> {
    let mut expected = vec![source.anchor(implementation.self_ty.span())];
    if let Some((_, path, _)) = &implementation.trait_ {
        expected.push(source.anchor(path.span()));
    }
    expected.push(source.anchor(implementation.generics.span()));

    let signature = super::signature::anchor(function, source);
    let generics = if function.sig.generics.params.is_empty() {
        let end = source.anchor(function.sig.ident.span()).span.end;
        Anchor {
            source_file: source.file.into(),
            span: ByteSpan { start: end, end },
        }
    } else {
        source.anchor(function.sig.generics.span())
    };

    if record.owners[0].header != expected || record.owners[1].header != [signature, generics] {
        return Err("Reference provenance changes complete original impl/method headers".into());
    }
    Ok(())
}

fn lifetime(owner: &Owner, syntax: &syn::GenericParam, source: &Source<'_>) -> Result<()> {
    let syn::GenericParam::Lifetime(lifetime) = syntax else {
        return Err("Reference parent does not own its original lifetime0".into());
    };
    if lifetime.lifetime.ident != "h" || !lifetime.bounds.is_empty() || !lifetime.attrs.is_empty() {
        return Err("Reference lifetime has unsupported extra bounds/attributes".into());
    }
    parameter(
        &owner.parameters[0],
        owner,
        Expected {
            index: 0,
            name: "'h",
            kind: "lifetime",
            anchor: source.anchor(lifetime.lifetime.span()),
        },
    )
}

struct Expected<'a> {
    index: u32,
    name: &'a str,
    kind: &'a str,
    anchor: Anchor,
}

fn parameter(record: &Parameter, owner: &Owner, expected: Expected<'_>) -> Result<()> {
    let Expected {
        index,
        name,
        kind,
        anchor,
    } = expected;
    let path = format!(
        "{}::{name}",
        owner.definition.path.replace("::<'h>", "<'h>")
    );
    if record.index != index
        || record.name != name
        || record.kind != kind
        || record.declaration != anchor
        || record.name_anchor != anchor
        || !record.definition.same_crate(&owner.definition)
        || record.definition.path != path
    {
        return Err(
            "Reference generic declaration differs from its original owner/index/name anchor"
                .into(),
        );
    }
    Ok(())
}

fn type_parameter(
    owner: &Owner,
    syntax: &syn::GenericParam,
    source: &Source<'_>,
) -> Result<[Anchor; 2]> {
    let syn::GenericParam::Type(parameter_syntax) = syntax else {
        return Err("Reference constructor loses original type parameter H".into());
    };
    let position = owner.parameters.len() - 1;
    parameter(
        &owner.parameters[position],
        owner,
        Expected {
            index: 1,
            name: "H",
            kind: "type",
            anchor: source.anchor(parameter_syntax.ident.span()),
        },
    )?;
    if parameter_syntax.ident != "H"
        || !parameter_syntax.attrs.is_empty()
        || parameter_syntax.default.is_some()
        || parameter_syntax.bounds.len() != 2
    {
        return Err("Reference H parameter changes the bounded original declaration".into());
    }
    let colon = parameter_syntax
        .colon_token
        .ok_or("Reference H parameter lacks its original bound colon")?;
    let predicate = Anchor {
        source_file: source.file.into(),
        span: ByteSpan {
            start: source.anchor(colon.span).span.start,
            end: source.anchor(parameter_syntax.bounds.span()).span.end,
        },
    };
    if owner.source_predicates != [predicate] {
        return Err("Reference source predicate roster omits or invents original H bounds".into());
    }

    let sized = trait_bound(&parameter_syntax.bounds[0], true)?;
    let as_ref = trait_bound(&parameter_syntax.bounds[1], false)?;
    if !sized.path.is_ident("Sized") || !as_ref_path(&as_ref.path) {
        return Err("Reference source bounds must be exactly ?Sized + AsRef<[u8]>".into());
    }
    Ok([source.anchor(sized.span()), source.anchor(as_ref.span())])
}

fn trait_bound(bound: &syn::TypeParamBound, maybe: bool) -> Result<&syn::TraitBound> {
    let syn::TypeParamBound::Trait(bound) = bound else {
        return Err("Reference source bound is not an original trait predicate".into());
    };
    if bound.paren_token.is_some()
        || bound.lifetimes.is_some()
        || matches!(bound.modifier, syn::TraitBoundModifier::Maybe(_)) != maybe
    {
        return Err("Reference source bound has unsupported binder/modifier".into());
    }
    Ok(bound)
}

fn as_ref_path(path: &syn::Path) -> bool {
    if path.leading_colon.is_some() || path.segments.len() != 1 || path.segments[0].ident != "AsRef"
    {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &path.segments[0].arguments else {
        return false;
    };
    arguments.args.len() == 1
        && matches!(arguments.args.first(), Some(syn::GenericArgument::Type(syn::Type::Slice(slice)))
        if matches!(slice.elem.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("u8")))
}
