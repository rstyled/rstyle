//! Type observations for local consumers; independent of strict extraction.
use super::hir::{Context, hir_id};
use rstyle_pdg::{HirFnFacts, consumer_facts::formal_types::*};
use rustc_hir::Mutability;
use rustc_middle::ty;

pub(super) fn capture(ctx: &Context<'_>, facts: &mut HirFnFacts) -> Result<(), String> {
    if facts.signature.params.len() > MAX_FORMALS {
        return Ok(());
    }
    let mutable_u32: Vec<_> = facts
        .signature
        .params
        .iter()
        .filter_map(|parameter| {
            let resolved = ctx.typeck.node_type_opt(hir_id(&parameter.pat.hir))?;
            let ty::Ref(_, referent, Mutability::Mut) = resolved.kind() else {
                return None;
            };
            matches!(referent.kind(), ty::Uint(ty::UintTy::U32)).then(|| Formal {
                parameter: parameter.position,
                binding: parameter.pat.hir.clone(),
            })
        })
        .collect();
    if !mutable_u32.is_empty() {
        facts.build.frontend_options.insert(
            KEY.into(),
            serde_json::to_value(Observations {
                identity: IDENTITY.into(),
                mutable_u32,
            })
            .map_err(|error| error.to_string())?,
        );
    }
    Ok(())
}
