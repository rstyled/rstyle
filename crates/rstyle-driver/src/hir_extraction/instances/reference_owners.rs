//! Owning-compiler reference-instance provenance; diagnostic evidence only.
pub(super) mod original;
mod projection;

use super::{Definition, definition_record, types};
use crate::hir_extraction::hir::Context;
pub(super) use projection::Projection;
use rstyle_pdg::ByteSpan;
use rustc_hir::{self as hir, def_id::LocalDefId};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Observation {
    identity: &'static str,
    complete: bool,
    definition: Definition,
    arguments: types::Arguments,
    owners: Vec<Owner>,
    signature: Signature,
}

#[derive(Serialize)]
struct Anchor {
    source_file: String,
    span: ByteSpan,
}

#[derive(Serialize)]
struct Owner {
    definition: Definition,
    parent: Option<Definition>,
    declaration: Anchor,
    header: Vec<Anchor>,
    parameters: Vec<Parameter>,
    source_predicates: Vec<Anchor>,
    clauses: Vec<Clause>,
}

#[derive(Serialize)]
struct Parameter {
    definition: Definition,
    index: u32,
    name: String,
    kind: &'static str,
    declaration: Anchor,
    name_anchor: Anchor,
}

#[derive(Serialize)]
struct Clause {
    anchor: Anchor,
    #[serde(flatten)]
    evidence: original::Clause,
}

#[derive(Serialize)]
struct Signature {
    anchor: Anchor,
    inputs: Vec<original::Type>,
    output: original::Type,
}

pub(super) fn capture<'tcx>(
    context: &Context<'tcx>,
    owner: LocalDefId,
    projection: &Projection<'tcx>,
) -> Result<Observation, String> {
    let tcx = context.tcx;
    let mut chain = Vec::new();
    let mut next = Some(owner.to_def_id());
    while let Some(id) = next {
        if chain.len() >= 3 || chain.contains(&id) || !id.is_local() {
            return Err("Reference generic owner chain is nonlocal, cyclic or oversized".into());
        }
        chain.push(id);
        next = tcx.generics_of(id).parent;
    }

    chain.reverse();
    let owners = chain
        .into_iter()
        .map(|id| owner_record(context, id.expect_local(), projection))
        .collect::<Result<_, _>>()?;
    let node = tcx.hir_node_by_def_id(owner);
    let signature = tcx.fn_sig(owner).instantiate_identity().skip_binder();

    Ok(Observation {
        identity: "rstyle.diagnostic.reference-owner-provenance/1",
        complete: true,
        definition: definition_record(tcx, owner.to_def_id()),
        arguments: types::arguments(tcx, projection.arguments()),
        owners,
        signature: Signature {
            anchor: anchor(
                context,
                node.fn_sig()
                    .ok_or("Missing reference owner signature")?
                    .span,
            )?,
            inputs: signature
                .inputs()
                .iter()
                .map(|value| original::ty(tcx, *value))
                .collect::<Result<_, _>>()?,
            output: original::ty(tcx, signature.output())?,
        },
    })
}

fn owner_record<'tcx>(
    context: &Context<'tcx>,
    id: LocalDefId,
    projection: &Projection<'tcx>,
) -> Result<Owner, String> {
    let tcx = context.tcx;
    let generics = tcx.generics_of(id);
    let source = tcx
        .hir_get_generics(id)
        .ok_or("Missing reference source generic roster")?;
    if source.params.len() != generics.own_params.len() || source.params.len() > 8 {
        return Err(
            "Reference source/compiler own parameter rosters differ or exceed budget".into(),
        );
    }

    let parameters = source
        .params
        .iter()
        .zip(&generics.own_params)
        .map(|(source, compiler)| parameter(context, source, compiler))
        .collect::<Result<_, _>>()?;
    let (declaration, header) = owner_header(context, id, source)?;
    let source_predicates = source
        .predicates
        .iter()
        .map(|predicate| anchor(context, predicate.span))
        .collect::<Result<_, _>>()?;
    let clauses = clauses(context, id, projection)?;

    Ok(Owner {
        definition: definition_record(tcx, id.to_def_id()),
        parent: generics.parent.map(|parent| definition_record(tcx, parent)),
        declaration,
        header,
        parameters,
        source_predicates,
        clauses,
    })
}

fn owner_header(
    context: &Context<'_>,
    id: LocalDefId,
    source: &hir::Generics<'_>,
) -> Result<(Anchor, Vec<Anchor>), String> {
    let node = context.tcx.hir_node_by_def_id(id);
    let (declaration, mut header) = match node {
        hir::Node::Item(item) => {
            let hir::ItemKind::Impl(implementation) = item.kind else {
                return Err("Reference parent owner is not an original impl".into());
            };
            let mut header = vec![anchor(context, implementation.self_ty.span)?];
            if let Some(trait_header) = implementation.of_trait {
                header.push(anchor(context, trait_header.trait_ref.path.span)?);
            }
            (anchor(context, item.span)?, header)
        }
        _ => (
            anchor(context, context.tcx.def_span(id))?,
            vec![anchor(
                context,
                node.fn_sig().ok_or("Missing reference method header")?.span,
            )?],
        ),
    };
    if !source.span.is_dummy() {
        header.push(anchor(context, source.span)?);
    } else if !source.params.is_empty() || !source.predicates.is_empty() {
        return Err("Nonempty reference source generics lack an original anchor".into());
    }
    Ok((declaration, header))
}

fn clauses<'tcx>(
    context: &Context<'tcx>,
    id: LocalDefId,
    projection: &Projection<'tcx>,
) -> Result<Vec<Clause>, String> {
    let tcx = context.tcx;
    let generics = tcx.generics_of(id);
    let clauses = tcx.clauses_of(id);
    if clauses.parent != generics.parent || clauses.clauses.len() > 64 {
        return Err("Reference clauses omit or contradict their generic parent".into());
    }

    let clauses = clauses
        .clauses
        .iter()
        .map(|(clause, span)| {
            Ok(Clause {
                anchor: anchor(context, *span)?,
                evidence: original::clause(context.tcx, *clause, projection)?,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(clauses)
}

fn parameter(
    context: &Context<'_>,
    source: &hir::GenericParam<'_>,
    compiler: &ty::GenericParamDef,
) -> Result<Parameter, String> {
    if source.def_id.to_def_id() != compiler.def_id
        || source.pure_wrt_drop
        || compiler.pure_wrt_drop
    {
        return Err("Reference generic parameter source/compiler identity differs".into());
    }
    let kind = match compiler.kind {
        ty::GenericParamDefKind::Lifetime => "lifetime",
        ty::GenericParamDefKind::Type {
            has_default: false,
            synthetic: false,
        } => "type",
        _ => return Err("Reference generic parameter kind unsupported".into()),
    };
    Ok(Parameter {
        definition: definition_record(context.tcx, compiler.def_id),
        index: compiler.index,
        name: compiler.name.to_string(),
        kind,
        declaration: anchor(context, source.span)?,
        name_anchor: anchor(context, source.name.ident().span)?,
    })
}

fn anchor(context: &Context<'_>, span: Span) -> Result<Anchor, String> {
    if span.from_expansion() || span.is_dummy() {
        return Err("Reference owner source anchor is expanded or absent".into());
    }

    let tcx: TyCtxt<'_> = context.tcx;
    let file = tcx.sess.source_map().lookup_source_file(span.lo());
    let rustc_span::FileName::Real(real) = &file.name else {
        return Err("Reference source anchor has no original filesystem path".into());
    };
    let path = real
        .local_path()
        .ok_or("Reference source anchor lacks a local path")?;

    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    let (_, span) = context
        .sources
        .range(tcx, span)
        .ok_or("Reference source anchor crosses or lacks original file bytes")?;
    Ok(Anchor {
        source_file: path.to_string_lossy().into_owned(),
        span,
    })
}
