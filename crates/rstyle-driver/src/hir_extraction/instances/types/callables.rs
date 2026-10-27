//! Callable identity observations grant no body, effect, or invocation permission.
use super::*;

#[derive(Serialize)]
pub(super) struct FunctionItem {
    definition: Definition,
    arguments: Vec<Structural>,
    signature: Signature,
}

#[derive(Serialize)]
pub(super) struct Closure {
    definition: Definition,
    parent_arguments: Vec<Structural>,
    closure_kind: String,
    capture_count: usize,
    upvars: Box<Structural>,
    signature: Signature,
}

#[derive(Serialize)]
struct Signature {
    inputs: Vec<Structural>,
    output: Box<Structural>,
    bound_variables: usize,
    abi: String,
    safety: String,
    variadic: bool,
}

pub(super) fn function_item<'tcx>(
    tcx: TyCtxt<'tcx>,
    value: Ty<'tcx>,
    depth: usize,
    budget: &Budget,
) -> Result<Structural, &'static str> {
    let ty::FnDef(definition, arguments) = value.kind() else {
        return Err("function item observation requires an exact definition");
    };
    if !matches!(
        tcx.def_kind(*definition),
        rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
    ) {
        return Err("callable is not an ordinary function item");
    }

    let arguments = arguments.skip_binder();
    let bound = tcx.fn_sig(*definition).instantiate(tcx, arguments);
    let signature = signature(tcx, bound, depth, budget)?;
    Ok(Structural::FunctionItem {
        observation: Box::new(FunctionItem {
            definition: super::super::definition_record(tcx, *definition),
            arguments: nested_arguments(tcx, arguments, depth + 1, budget)?,
            signature,
        }),
    })
}

pub(super) fn closure<'tcx>(
    tcx: TyCtxt<'tcx>,
    value: Ty<'tcx>,
    depth: usize,
    budget: &Budget,
) -> Result<Structural, &'static str> {
    let ty::Closure(definition, arguments) = value.kind() else {
        return Err("closure observation requires its exact definition");
    };
    let local = definition.as_local().ok_or("closure owner is not local")?;
    let closure = arguments.as_closure();
    if !tcx.closure_captures(local).is_empty() || !closure.upvar_tys().is_empty() {
        return Err("capturing closure is outside structural callable observation");
    }

    let parent = tcx.mk_args(&arguments[..tcx.generics_of(*definition).parent_count]);
    Ok(Structural::NoncapturingClosure {
        observation: Box::new(Closure {
            definition: super::super::definition_record(tcx, *definition),
            parent_arguments: nested_arguments(tcx, parent, depth + 1, budget)?,
            closure_kind: format!("{:?}", closure.kind()),
            capture_count: 0,
            upvars: Box::new(structural(
                tcx,
                closure.tupled_upvars_ty(),
                depth + 1,
                budget,
            )?),
            signature: signature(tcx, ty::Unnormalized::new_wip(closure.sig()), depth, budget)?,
        }),
    })
}

fn signature<'tcx>(
    tcx: TyCtxt<'tcx>,
    bound: ty::Unnormalized<'tcx, ty::PolyFnSig<'tcx>>,
    depth: usize,
    budget: &Budget,
) -> Result<Signature, &'static str> {
    if !bound.skip_normalization().bound_vars().is_empty() {
        return Err("callable signature retains bound variables");
    }
    let signature = tcx
        .try_normalize_erasing_regions(ty::TypingEnv::fully_monomorphized(), bound)
        .map_err(|_| "callable signature normalization failed")?
        .skip_binder();

    Ok(Signature {
        inputs: signature
            .inputs()
            .iter()
            .map(|input| structural(tcx, *input, depth + 1, budget))
            .collect::<Result<_, _>>()?,
        output: Box::new(structural(tcx, signature.output(), depth + 1, budget)?),
        bound_variables: 0,
        abi: signature.abi().to_string(),
        safety: format!("{:?}", signature.safety()),
        variadic: signature.c_variadic(),
    })
}
