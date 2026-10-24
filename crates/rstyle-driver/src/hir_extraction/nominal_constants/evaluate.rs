//! One typed structural product evaluation; never decode allocation bytes.
use super::{CaptureFailure, Context};
use rstyle_pdg::{concrete_types::TypeTree, nominal_constants::ScalarValue};
use rustc_hir::def_id::DefId;
use rustc_middle::{
    mir::interpret::GlobalId,
    ty::{self, Ty, consts::ConstExt},
};

pub(super) fn owner<'tcx>(
    context: &Context<'tcx>,
    definition: DefId,
    observed: &TypeTree,
) -> Result<Ty<'tcx>, CaptureFailure> {
    let tcx = context.tcx;
    let environment = ty::TypingEnv::post_analysis(tcx, definition);
    let value = tcx
        .try_normalize_erasing_regions(environment, tcx.type_of(definition).instantiate_identity())
        .map_err(|_| CaptureFailure::Unsupported("product constant type is not normalized"))?;
    let ty::Adt(adt, arguments) = value.kind() else {
        return Err(CaptureFailure::Unsupported(
            "constant is not a nominal product",
        ));
    };

    let TypeTree::Struct {
        nominal, fields, ..
    } = observed
    else {
        return Err(CaptureFailure::Unsupported(
            "missing complete product observation",
        ));
    };
    if !adt.is_struct()
        || !adt.did().is_local()
        || !arguments.is_empty()
        || adt.has_dtor(tcx)
        || !tcx.type_is_copy_modulo_regions(environment, value)
        || nominal.definition_path != tcx.def_path_str(adt.did())
        || fields.len() != 1
        || fields[0].position != 0
        || fields[0].name != "0"
        || fields[0].ty != TypeTree::Scalar("u32".into())
        || adt.non_enum_variant().fields.len() != 1
        || adt
            .non_enum_variant()
            .fields
            .iter()
            .next()
            .is_none_or(|field| field.ty(tcx, arguments).skip_norm_wip() != tcx.types.u32)
    {
        return Err(CaptureFailure::Unsupported(
            "product constant requires one concrete Copy u32 tuple field",
        ));
    }

    Ok(value)
}

pub(super) fn field<'tcx>(
    context: &Context<'tcx>,
    definition: DefId,
    value: Ty<'tcx>,
) -> Result<ScalarValue, CaptureFailure> {
    let tcx = context.tcx;
    let environment = ty::TypingEnv::post_analysis(tcx, definition);
    let instance = ty::Instance::new_raw(
        definition,
        ty::GenericArgs::identity_for_item(tcx, definition),
    );
    let tree = tcx
        .const_eval_global_id_for_typeck(
            environment,
            GlobalId {
                instance,
                promoted: None,
            },
            tcx.def_span(definition),
        )
        .map_err(|error| CaptureFailure::Evaluation(format!("{error:?}")))?
        .map_err(|_| {
            CaptureFailure::Unsupported("product constant has no structural value tree")
        })?;
    let product = ty::Value {
        ty: value,
        valtree: tree,
    };
    let components = product.try_to_branch().ok_or(CaptureFailure::Unsupported(
        "product constant lacks structural fields",
    ))?;
    let [component] = components else {
        return Err(CaptureFailure::Unsupported(
            "product constant structural arity differs",
        ));
    };
    let component = component.try_to_value().ok_or(CaptureFailure::Unsupported(
        "product component lacks a typed value",
    ))?;
    if component.ty != tcx.types.u32 {
        return Err(CaptureFailure::Unsupported("product component is not u32"));
    }
    let scalar = component.try_to_leaf().ok_or(CaptureFailure::Unsupported(
        "product component is not a scalar leaf",
    ))?;

    Ok(ScalarValue {
        ty: "u32".into(),
        value: scalar.to_u32().to_string(),
    })
}
