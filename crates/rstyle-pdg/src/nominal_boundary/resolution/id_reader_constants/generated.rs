//! Generated associated constants use emitted ownership, never callable observations.
use super::*;
use crate::{
    id_reader_constants::source,
    nominal_boundary::census::generated_sources::{
        self as expansion, CheckedExpansion, CompilerInput,
    },
};

pub(super) fn verify(authority: &SourceAuthority<'_>, record: &Observation) -> Result<(), String> {
    let capture = authority
        .input
        .generated_source_capture()?
        .ok_or("Missing generated ID SIZE unit capture")?;
    if capture.identity != UNIT_IDENTITY || capture.scope != "selected-local-bang-expansion-closure"
    {
        return Err("ID SIZE requires its exact generated unit capture scope".into());
    }

    let root = one(
        capture
            .expansions
            .iter()
            .filter(|root| root.expansion == record.defining_expansion),
        "generated ID SIZE defining expansion",
    )?;
    authority::root_scope(authority.input.facts(), root)?;
    let environment = &authority.resolution.environment;
    let checked = expansion::verify_compiler_in(
        CompilerInput {
            census: environment.census(),
            cfgs: &authority.input.build.cfgs,
            definition_header: root
                .definition
                .as_ref()
                .ok_or("Missing ID SIZE macro definition")?,
            invocation_macro: &root.invocation.raw,
            preceding_expansions: crate::extraction_profile::facts_support(
                authority.input.facts(),
                "rust.ordered-local-macro-source",
            ),
        },
        environment,
    )?;
    let macro_definition = root
        .macro_definition
        .as_ref()
        .ok_or("Missing ID SIZE macro identity")?;
    if macro_definition.path != checked.macro_path().join("::")
        || macro_definition.crate_name != environment.census().context.crate_name
        || Some(root.name.as_str()) != checked.macro_path().last().map(String::as_str)
    {
        return Err("ID SIZE defining macro differs from checked original expansion".into());
    }
    roster(authority, record, &capture, &checked)?;
    member(authority, record, &checked)
}

fn roster(
    authority: &SourceAuthority<'_>,
    record: &Observation,
    capture: &UnitCapture,
    checked: &CheckedExpansion,
) -> Result<(), String> {
    let headers = headers(record, checked)?;
    let emitted = one(
        capture.emitted.iter().filter(|entry| {
            entry.definition.definition_identity == record.definition.definition_identity
        }),
        "exact emitted ID SIZE associated constant",
    )?;
    let implementation = one(
        capture.emitted.iter().filter(|entry| {
            entry.definition.definition_identity == record.implementation.definition_identity
        }),
        "exact emitted ID SIZE implementation",
    )?;
    if emitted.definition != record.definition
        || emitted.parent != record.implementation
        || emitted.origin.raw != headers.member
        || emitted.origin.marks != record.origin.marks
        || emitted.kind != "AssocConst"
        || implementation.definition != record.implementation
        || implementation.origin.raw != headers.implementation
        || implementation.origin.marks != record.implementation_origin.marks
        || implementation.kind != "Impl { of_trait: false }"
    {
        return Err("ID SIZE emitted constant and implementation ownership disagree".into());
    }

    authority::module_parent(
        authority.input.facts(),
        &implementation.parent,
        checked.namespace(),
    )?;
    let expected = vec![Mark {
        expansion: record.defining_expansion.clone(),
        transparency: Transparency::SemiOpaque,
    }];
    if record.origin.marks != expected || record.implementation_origin.marks != expected {
        return Err("ID SIZE lacks exact local defining expansion hygiene".into());
    }
    Ok(())
}

fn member(
    authority: &SourceAuthority<'_>,
    record: &Observation,
    checked: &CheckedExpansion,
) -> Result<(), String> {
    let (implementation, member) = selected_source(record, checked)?;
    source::implementation(implementation, terminal(&record.owner)?)?;
    source::member(member)?;
    let configuration = crate::conditional_source::Configuration::new(&authority.input.build.cfgs);
    if !configuration.active(&implementation.attrs)? || !configuration.active(&member.attrs)? {
        return Err("ID SIZE source member is inactive in selected build".into());
    }

    let namespace = checked.namespace().to_vec();
    let scoped = Scoped {
        resolution: authority.resolution,
        namespace: &namespace,
    };
    let syn::Type::Path(owner) = implementation.self_ty.as_ref() else {
        return Err("Generated ID SIZE implementation lost its original concrete owner".into());
    };
    original::nominal(scoped.resolve(owner)?, &record.owner)?;
    let original = source::owner_path(&member.expr)?;
    original::nominal(scoped.resolve(&original)?, &record.size_argument)
}

fn selected_source<'a>(
    record: &Observation,
    checked: &'a CheckedExpansion,
) -> Result<(&'a syn::ItemImpl, &'a syn::ImplItemConst), String> {
    let implementation = one(
        checked.items().iter().filter_map(|item| {
            let syn::Item::Impl(item) = item else {
                return None;
            };
            let span = item.impl_token.span.join(item.brace_token.span.close())?;
            (checked.source_span(span) == record.implementation_origin.raw).then_some(item)
        }),
        "complete generated ID SIZE source implementation",
    )?;
    let member = one(
        implementation.items.iter().filter_map(|item| {
            let syn::ImplItem::Const(item) = item else {
                return None;
            };
            let start = if matches!(item.vis, syn::Visibility::Inherited) {
                item.const_token.span
            } else {
                item.vis.span()
            };
            let span = start.join(item.semi_token.span)?;
            (checked.source_span(span) == record.origin.raw).then_some(item)
        }),
        "complete generated ID SIZE source member",
    )?;
    Ok((implementation, member))
}

struct Headers {
    implementation: census::expansions::FileSpan,
    member: census::expansions::FileSpan,
}

fn headers(record: &Observation, checked: &CheckedExpansion) -> Result<Headers, String> {
    let (source_impl, source_member) = selected_source(record, checked)?;
    // rustc's definition roster records declaration headers; HIR item spans
    // retain the complete initializer/body checked independently below.
    let implementation_header = source_impl
        .impl_token
        .span
        .join(source_impl.self_ty.span())
        .ok_or("Missing generated ID SIZE implementation header span")?;
    let start = if matches!(source_member.vis, syn::Visibility::Inherited) {
        source_member.const_token.span
    } else {
        source_member.vis.span()
    };
    let member_header = start
        .join(source_member.ty.span())
        .ok_or("Missing generated ID SIZE constant header span")?;

    Ok(Headers {
        implementation: checked.source_span(implementation_header),
        member: checked.source_span(member_header),
    })
}
