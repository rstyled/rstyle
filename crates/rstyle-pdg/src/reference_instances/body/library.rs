//! Exact existing slice-length evidence on the once-converted saved view.
use super::{Result, site};
use crate::{
    HirFnFacts, HirNode,
    concrete_types::{Observation, TypeTree},
    data_receivers::{Capture, Kind},
};
use std::collections::BTreeMap;

pub(super) fn verify(canonical: &HirFnFacts, length: &HirNode) -> Result<()> {
    let registry = crate::data_library::registry_for(canonical)
        .ok_or("Unpinned constructor length compiler/target")?;
    let operation = registry
        .operations
        .iter()
        .find(|operation| operation.id == "rstyle.data.slice.len")
        .ok_or("Missing reviewed slice length operation")?;
    let callee = crate::data_library::callee(length)?;
    let libraries: crate::data_library::Libraries =
        crate::data_frontend::evidence(canonical, crate::data_library::KEY)?;
    if libraries.identity != crate::data_library::IDENTITY
        || libraries.libraries.len() != 1
        || libraries.libraries[0].crate_name != "core"
        || libraries.libraries[0].crate_hash != registry.core_hash
        || callee.path != operation.path
        || callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || callee.definition_hash != operation.definition_hash
        || callee.implementation.is_some()
        || operation.result != "usize"
        || operation.referent != "slice"
    {
        return Err("Constructor length is not its exact pinned slice metadata operation".into());
    }

    let dispatch = length
        .dispatch
        .as_ref()
        .ok_or("Constructor length lacks dispatch evidence")?;
    if dispatch.kind != "resolved_method"
        || dispatch.op != operation.path
        || dispatch.operand_tys.len() != 1
        || !super::projection::shared_bytes_token(&dispatch.operand_tys[0])?
        || dispatch.result_ty != "usize"
    {
        return Err("Constructor length dispatch changes receiver/result semantics".into());
    }
    let capture = canonical.build.frontend_options.data::<Capture>()?;
    let records = records(canonical, &capture)?;
    let record = records
        .get(&site(&length.hir))
        .ok_or("Missing exact constructor length receiver record")?;
    signature(record)
}

fn records<'a>(
    canonical: &HirFnFacts,
    capture: &'a Capture,
) -> Result<BTreeMap<super::Site, &'a crate::data_receivers::Method>> {
    let mut methods = Vec::new();
    crate::frontend::collect_kind(&canonical.body, "expr_method_call", &mut methods);
    if capture.identity != crate::data_receivers::IDENTITY
        || capture.methods.len() != 2
        || methods.len() != 2
    {
        return Err(
            "Constructor requires its complete AsRef and length receiver inventories".into(),
        );
    }

    let mut remaining: BTreeMap<_, _> = methods
        .into_iter()
        .map(|node| (site(&node.hir), node))
        .collect();
    for record in &capture.methods {
        let node = remaining
            .remove(&site(&record.call))
            .ok_or("Constructor receiver duplicates or invents a method site")?;
        let [receiver] = node.children.as_slice() else {
            return Err("Constructor method receiver roster differs".into());
        };
        if record.source_span != node.span || record.receiver != receiver.hir {
            return Err(
                "Constructor receiver evidence is detached from its exact HIR/source site".into(),
            );
        }
    }
    Ok(capture
        .methods
        .iter()
        .map(|record| (site(&record.call), record))
        .collect())
}

fn signature(record: &crate::data_receivers::Method) -> Result<()> {
    let slice = TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into())));
    let shared = Observation::Captured {
        ty: TypeTree::Shared(Box::new(slice.clone())),
    };
    if record.receiver_type != shared
        || record.adjusted_type != shared
        || record.signature_inputs != [shared.clone()]
        || record.signature_result
            != (Observation::Captured {
                ty: TypeTree::Scalar("usize".into()),
            })
    {
        return Err("Constructor length lacks its exact concrete shared-slice signature".into());
    }

    let [dereference, borrow] = record.adjustments.as_slice() else {
        return Err(
            "Constructor length must retain complete builtin deref/shared-borrow adjustments"
                .into(),
        );
    };
    if dereference.kind != Kind::BuiltinDereference
        || dereference.target != (Observation::Captured { ty: slice })
        || borrow.kind != Kind::SharedBorrow
        || borrow.target != shared
    {
        return Err("Constructor length receiver changes adjustment types/order".into());
    }
    Ok(())
}
