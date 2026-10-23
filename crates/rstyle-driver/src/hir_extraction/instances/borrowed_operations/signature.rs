//! Structural signatures plus original late-bound receiver/result region identities.
use super::*;

#[derive(Serialize)]
pub(super) struct Signature {
    inputs: types::Arguments,
    output: types::Arguments,
    bound_variables: usize,
    receiver_region: Region,
    result_region: Region,
}

#[derive(Serialize)]
struct Region {
    binder_depth: u32,
    variable: u32,
}

pub(super) fn capture<'tcx>(
    tcx: TyCtxt<'tcx>,
    definition: DefId,
    arguments: ty::GenericArgsRef<'tcx>,
) -> Result<Signature, String> {
    let bound = tcx.fn_sig(definition).instantiate(tcx, arguments);
    let original = bound.skip_binder();
    let [receiver] = original.inputs() else {
        return Err("AsRef selected signature lacks its single receiver".into());
    };

    let receiver_region = region(*receiver)?;
    let result_region = region(original.output())?;

    let normalized = tcx
        .try_normalize_erasing_regions(ty::TypingEnv::fully_monomorphized(), bound)
        .map_err(|_| "AsRef selected signature normalization failed")?
        .skip_binder();
    let [receiver] = normalized.inputs() else {
        return Err("AsRef normalized signature lacks its single receiver".into());
    };

    Ok(Signature {
        inputs: endpoint(tcx, *receiver)?,
        output: endpoint(tcx, normalized.output())?,
        bound_variables: bound.skip_normalization().bound_vars().len(),
        receiver_region,
        result_region,
    })
}

fn region(value: Ty<'_>) -> Result<Region, String> {
    let ty::Ref(region, _, Mutability::Not) = value.kind() else {
        return Err("AsRef signature requires shared receiver and result references".into());
    };
    let ty::ReBound(ty::BoundVarIndexKind::Bound(depth), bound) = region.kind() else {
        return Err("AsRef signature lacks an original late-bound borrow region".into());
    };
    Ok(Region {
        binder_depth: depth.as_u32(),
        variable: bound.var.as_u32(),
    })
}
