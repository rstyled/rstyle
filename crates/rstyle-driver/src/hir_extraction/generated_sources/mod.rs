//! Explicit diagnostic-only raw generated-source capture. No admission changes.
mod nodes;
mod provenance;
mod roots;
mod roster;

use super::Options;
use provenance::{Provenance, definition_record, identity};
use roots::Roots;
use rstyle_pdg::{HirFnFacts, generated_sources::capture::*};
use rustc_hir::def_id::LocalDefId;
use rustc_middle::ty::TyCtxt;
use rustc_span::{ExpnId, Span};
use std::{collections::BTreeSet, sync::Arc};

pub(super) fn capture(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    if !rstyle_pdg::diagnostics::switch("RSTYLE_DIAGNOSTIC_GENERATED_SOURCES") {
        return Ok(());
    }

    let mut roots = Roots::new(tcx);
    let selected = selected(tcx, facts, &mut roots);
    if selected.is_empty() {
        return Ok(());
    }
    let provenance = Provenance::new(tcx, &options.sources);
    let capture = unit_capture(provenance, facts, &selected, &mut roots);

    // One unit record: consumers join outcomes by their exact definition identity.
    facts[selected[0].index]
        .build
        .frontend_options
        .insert_data(capture);
    Ok(())
}

fn unit_capture<'tcx>(
    mut provenance: Provenance<'_, 'tcx>,
    facts: &[HirFnFacts],
    selected: &[Selected],
    roots: &mut Roots<'tcx>,
) -> Arc<UnitCapture> {
    let root_ids: BTreeSet<_> = selected
        .iter()
        .filter_map(|selection| selection.root.as_ref().ok().map(|root| identity(*root)))
        .collect();
    let (emitted, mut outcomes) = roster::capture(&mut provenance, roots, &root_ids);

    outcomes.extend(
        selected.iter().map(|selection| {
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
            tcx.inherent_impl_of_assoc(id.to_def_id())?;
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
    let implementation = tcx
        .inherent_impl_of_assoc(id.to_def_id())
        .ok_or("Generated callable is not an inherent implementation member")?;

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
