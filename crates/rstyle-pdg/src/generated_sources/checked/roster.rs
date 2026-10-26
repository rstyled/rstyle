//! Complete direct implementation-member roster, derived from original expansion syntax.
use super::*;
use crate::conditional_source::Configuration;

pub(super) struct Selection {
    pub implementation: usize,
    pub member: usize,
}
impl Selection {
    pub fn function<'a>(&self, expansion: &'a source::CheckedExpansion) -> &'a syn::ImplItemFn {
        let syn::Item::Impl(item) = &expansion.items()[self.implementation] else {
            unreachable!("Checked impl")
        };
        let syn::ImplItem::Fn(function) = &item.items[self.member] else {
            unreachable!("Checked member")
        };
        function
    }
}

pub(super) fn verify(
    input: &Input<'_>,
    observation: &Observation,
    expansion: &source::CheckedExpansion,
    conversion: bool,
) -> Result<Selection, String> {
    let emitted = unique_definition(input.capture, &observation.implementation)?;
    let owner = implementation_owner(input, observation, expansion, conversion)?;
    let (implementation, item) = selected_impl(observation, expansion, emitted, owner.as_deref())?;
    let configuration = Configuration::new(&input.facts.build.cfgs);
    let members = members(item, &configuration, conversion)?;
    let mut actual: Vec<_> = input
        .capture
        .emitted
        .iter()
        .filter(|member| member.parent == observation.implementation)
        .collect();
    actual.sort_by_key(|member| member.origin.raw.span.start);

    crate::source_evidence::exact_pairs(
        actual.into_iter(),
        members.iter(),
        "Generated implementation member roster is incomplete or has surplus definitions",
        |actual, expected| member(input, observation, expansion, (actual, expected)),
    )?;
    let selected: Vec<_> = members
        .iter()
        .filter(|member| {
            observation.owner.path
                == format!("{}::{}", observation.implementation.path, member.name)
        })
        .collect();
    if selected.len() != 1 || selected[0].kind != "AssocFn" {
        return Err("Generated function lacks a unique original direct member".into());
    }
    let selected_emitted = unique_definition(input.capture, &observation.owner)?;
    if selected_emitted.parent != observation.implementation {
        return Err("Generated function has another exact implementation parent".into());
    }
    Ok(Selection {
        implementation,
        member: selected[0].index,
    })
}

fn implementation_owner(
    input: &Input<'_>,
    observation: &Observation,
    expansion: &source::CheckedExpansion,
    conversion: bool,
) -> Result<Option<String>, String> {
    let emitted = unique_definition(input.capture, &observation.implementation)?;
    if emitted.kind
        != if conversion {
            "Impl { of_trait: true }"
        } else {
            "Impl { of_trait: false }"
        }
    {
        return Err(
            "Generated callable owner differs from the required implementation kind".into(),
        );
    }
    authority::module_parent(input.facts, &emitted.parent, expansion.namespace())?;

    let owner = if conversion {
        let (_, item) = original_impl(expansion, emitted)?;
        Some(super::conversion::verify(
            input,
            observation,
            expansion,
            item,
        )?)
    } else {
        None
    };
    Ok(owner)
}

fn unique_definition<'a>(
    capture: &'a UnitCapture,
    definition: &Definition,
) -> Result<&'a Emitted, String> {
    let mut entries = capture
        .emitted
        .iter()
        .filter(|entry| entry.definition.definition_identity == definition.definition_identity);
    let entry = entries.next().ok_or("Missing exact emitted definition")?;
    if entries.next().is_some() || entry.definition != *definition {
        return Err("Ambiguous or inconsistent emitted definition identity".into());
    }
    Ok(entry)
}

fn selected_impl<'a>(
    observation: &Observation,
    expansion: &'a source::CheckedExpansion,
    emitted: &Emitted,
    converted_owner: Option<&str>,
) -> Result<(usize, &'a syn::ItemImpl), String> {
    let (index, item) = original_impl(expansion, emitted)?;
    let owner = converted_owner
        .map(str::to_owned)
        .map_or_else(|| impl_owner(item, expansion), Ok)?;
    if observation.implementation.path != owner {
        return Err("Generated impl identity differs from substituted nominal syntax".into());
    }

    origin(
        &emitted.origin,
        &expansion.source_span(impl_header(item)?),
        &observation.defining_expansion,
    )?;
    Ok((index, item))
}

fn original_impl<'a>(
    expansion: &'a source::CheckedExpansion,
    emitted: &Emitted,
) -> Result<(usize, &'a syn::ItemImpl), String> {
    let mut matches = Vec::new();
    for (index, item) in expansion.items().iter().enumerate() {
        let syn::Item::Impl(item) = item else {
            continue;
        };
        if expansion.source_span(impl_header(item)?) == emitted.origin.raw {
            matches.push((index, item));
        }
    }
    if matches.len() != 1 {
        return Err("Generated impl lacks a unique exact template header".into());
    }
    Ok(matches.remove(0))
}

fn impl_header(item: &syn::ItemImpl) -> Result<proc_macro2::Span, String> {
    item.impl_token
        .span
        .join(item.self_ty.span())
        .ok_or_else(|| "Missing exact original implementation header".into())
}

fn impl_owner(
    item: &syn::ItemImpl,
    expansion: &source::CheckedExpansion,
) -> Result<String, String> {
    if item.trait_.is_some()
        || item.unsafety.is_some()
        || item.defaultness.is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
    {
        return Err("Generated implementation has unsupported ownership syntax".into());
    }
    let syn::Type::Path(path) = item.self_ty.as_ref() else {
        return Err("Generated impl has nonnominal owner".into());
    };
    let owner = path
        .path
        .get_ident()
        .filter(|_| path.qself.is_none())
        .ok_or("Generated impl requires a plain local nominal owner")?;

    Ok(expansion
        .namespace()
        .iter()
        .cloned()
        .chain([owner.to_string()])
        .collect::<Vec<_>>()
        .join("::"))
}

struct Member {
    index: usize,
    kind: &'static str,
    name: String,
    span: proc_macro2::Span,
}

fn members(
    item: &syn::ItemImpl,
    configuration: &Configuration<'_>,
    conversion: bool,
) -> Result<Vec<Member>, String> {
    let mut members = Vec::new();
    for (index, item) in item.items.iter().enumerate() {
        let (attrs, kind, name, span) = match item {
            syn::ImplItem::Fn(item) => (
                &item.attrs,
                "AssocFn",
                &item.sig.ident,
                visible_span(&item.vis, item.sig.span())?,
            ),
            syn::ImplItem::Const(item) => (
                &item.attrs,
                "AssocConst",
                &item.ident,
                visible_span(
                    &item.vis,
                    item.const_token
                        .span
                        .join(item.ty.span())
                        .ok_or("Missing constant header")?,
                )?,
            ),
            syn::ImplItem::Type(item) if conversion => (
                &item.attrs,
                "AssocTy",
                &item.ident,
                item.type_token
                    .span
                    .join(item.ident.span())
                    .ok_or("Missing associated type header")?,
            ),
            _ => return Err("Generated implementation has an unresolved member inventory".into()),
        };
        if !configuration.active(attrs)? {
            continue;
        }
        attributes(attrs, configuration)?;
        members.push(Member {
            index,
            kind,
            name: name.to_string(),
            span,
        });
    }
    Ok(members)
}

pub(super) fn attributes(
    attrs: &[syn::Attribute],
    configuration: &Configuration<'_>,
) -> Result<(), String> {
    for attribute in configuration.effective_attributes(attrs)? {
        if ![
            "doc", "inline", "must_use", "allow", "warn", "deny", "forbid", "cfg",
        ]
        .iter()
        .any(|name| attribute.path().is_ident(name))
        {
            return Err("Generated member has an unsupported expansion attribute".into());
        }
    }
    Ok(())
}

fn member(
    input: &Input<'_>,
    observation: &Observation,
    expansion: &source::CheckedExpansion,
    pair: (&Emitted, &Member),
) -> Result<(), String> {
    let (actual, expected) = pair;
    authority::local(input.facts, &actual.definition)?;
    unique_definition(input.capture, &actual.definition)?;

    if actual.kind != expected.kind
        || actual.definition.path
            != format!("{}::{}", observation.implementation.path, expected.name)
    {
        return Err("Generated emitted member differs from exact substituted source member".into());
    }
    origin(
        &actual.origin,
        &expansion.source_span(expected.span),
        &observation.defining_expansion,
    )
}
