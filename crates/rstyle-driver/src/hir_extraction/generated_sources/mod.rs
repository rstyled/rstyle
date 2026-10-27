//! Explicit diagnostic-only raw generated-source capture. No admission changes.
mod nodes;
mod provenance;
mod roots;
mod roster;

use super::Options;
use provenance::{Provenance, definition_record, identity};
use roots::Roots;
use rstyle_pdg::{HirFnFacts, generated_sources::capture::*};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::ty::TyCtxt;
use rustc_span::{ExpnId, Span};
use std::{collections::BTreeSet, sync::Arc};

pub(super) fn declaration_origin(
    ctx: &super::hir::Context<'_>,
    id: DefId,
    span: Span,
) -> Result<(Definition, Origin, String), String> {
    let root = Roots::new(ctx.tcx)
        .definition(id.expect_local())?
        .ok_or("Generated declaration has no unique original local bang root")?;
    let mut provenance = Provenance::new(ctx.tcx, &ctx.sources);

    provenance.expansion(root)?;
    let origin = provenance.origin(span)?;
    Ok((definition_record(ctx.tcx, id), origin, identity(root)))
}

pub(super) fn capture(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    if !options
        .selected()
        .supports(rstyle_pdg::result_maps::CAPABILITY)
        && !rstyle_pdg::diagnostics::switch("RSTYLE_DIAGNOSTIC_GENERATED_SOURCES")
    {
        return Ok(());
    }

    let mut roots = Roots::new(tcx);
    let selected = Selection::capture(tcx, facts, &mut roots)?;
    if selected.functions.is_empty() && selected.roots.is_empty() {
        return Ok(());
    }
    let provenance = Provenance::new(tcx, &options.sources);
    let capture = unit_capture(provenance, facts, &selected, &mut roots);

    // One unit record: consumers join outcomes by their exact definition identity.
    facts[selected
        .functions
        .first()
        .map_or(0, |selection| selection.index)]
    .build
    .frontend_options
    .insert_data(capture);
    Ok(())
}

fn declaration_roots(facts: &[HirFnFacts]) -> Result<BTreeSet<String>, String> {
    let mut roots = BTreeSet::new();
    for facts in facts {
        if facts
            .build
            .frontend_options
            .contains_key(rstyle_pdg::declaration_origins::KEY)
        {
            let origins = facts
                .build
                .frontend_options
                .data::<rstyle_pdg::declaration_origins::Origins>()?;
            roots.extend(
                origins
                    .generated
                    .iter()
                    .map(|origin| origin.defining_expansion.clone()),
            );
        }
    }
    Ok(roots)
}

fn unit_capture<'tcx>(
    mut provenance: Provenance<'_, 'tcx>,
    facts: &[HirFnFacts],
    selected: &Selection,
    roots: &mut Roots<'tcx>,
) -> Arc<UnitCapture> {
    let (emitted, mut outcomes) = roster::capture(&mut provenance, roots, &selected.roots);

    outcomes.extend(
        selected.functions.iter().map(|selection| {
            observation_outcome(&mut provenance, selection, &facts[selection.index])
        }),
    );

    Arc::new(UnitCapture {
        identity: UNIT_IDENTITY.into(),
        complete: outcomes
            .iter()
            .all(|outcome| matches!(outcome, Outcome::Observed { .. })),
        scope: "selected-local-bang-expansion-closure".into(),
        outcomes,
        expansions: provenance.finish(),
        emitted,
    })
}

fn observation_outcome(
    provenance: &mut Provenance<'_, '_>,
    selection: &Selected,
    facts: &HirFnFacts,
) -> Outcome {
    let result = selection.root.clone().and_then(|root| {
        provenance.expansion(root)?;
        observe(provenance, selection.id, root, facts)
    });

    match result {
        Ok(observation) => Outcome::Observed {
            observation: Box::new(observation),
        },
        Err(reason) => Outcome::Unsupported {
            owner: definition_record(provenance.tcx, selection.id.to_def_id()),
            reason,
        },
    }
}

struct Selection {
    functions: Vec<Selected>,
    roots: BTreeSet<String>,
}

impl Selection {
    fn capture(
        tcx: TyCtxt<'_>,
        facts: &[HirFnFacts],
        roots: &mut Roots<'_>,
    ) -> Result<Self, String> {
        let functions = selected(tcx, facts, roots);
        let mut root_ids = declaration_roots(facts)?;
        root_ids.extend(
            functions
                .iter()
                .filter_map(|selection| selection.root.as_ref().ok().map(|root| identity(*root))),
        );
        Ok(Self {
            functions,
            roots: root_ids,
        })
    }
}

struct Selected {
    index: usize,
    id: LocalDefId,
    root: Result<ExpnId, String>,
}

fn selected(tcx: TyCtxt<'_>, facts: &[HirFnFacts], roots: &mut Roots<'_>) -> Vec<Selected> {
    facts
        .iter()
        .enumerate()
        .filter_map(|(index, facts)| {
            let id = LocalDefId {
                local_def_index: rustc_hir::def_id::DefIndex::from_u32(facts.body.hir.owner),
            };
            let node = tcx.hir_node_by_def_id(id);
            implementation(tcx, id)?;
            if !node
                .fn_sig()
                .is_some_and(|signature| signature.span.from_expansion())
            {
                return None;
            }
            let root = roots.definition(id).and_then(|root| {
                root.ok_or_else(|| {
                    "Expanded callable has no unique original local bang expansion".into()
                })
            });
            Some(Selected { index, id, root })
        })
        .collect()
}

fn observe(
    provenance: &mut Provenance<'_, '_>,
    id: LocalDefId,
    root: ExpnId,
    facts: &HirFnFacts,
) -> Result<Observation, String> {
    let tcx = provenance.tcx;
    let implementation = implementation(tcx, id)
        .ok_or("Generated callable lacks its selected concrete implementation")?;

    let node = tcx.hir_node_by_def_id(id);
    let signature = node
        .fn_sig()
        .ok_or("Generated callable signature is missing")?;
    let body = tcx.hir_body(node.body_id().ok_or("Generated callable body is missing")?);

    Ok(Observation {
        identity: IDENTITY.into(),
        owner: definition_record(tcx, id.to_def_id()),
        implementation: definition_record(tcx, implementation),
        defining_expansion: identity(root),
        signature: provenance.origin(signature.span)?,
        body: provenance.origin(body.value.span)?,
        nodes: nodes::capture(provenance, facts)?,
    })
}

// Capturing a trait member supplies no source, cleanup or callable permission.
fn implementation(tcx: TyCtxt<'_>, id: LocalDefId) -> Option<DefId> {
    if let Some(implementation) = tcx.inherent_impl_of_assoc(id.to_def_id()) {
        return Some(implementation);
    }
    let method = tcx
        .opt_associated_item(id.to_def_id())?
        .trait_item_def_id()?;
    if tcx.get_diagnostic_item(rustc_span::Symbol::intern("try_from_fn")) != Some(method) {
        return None;
    }
    tcx.impl_of_assoc(id.to_def_id())
}
