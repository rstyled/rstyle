//! Observations of unary struct constructors, never general callable permission.
use super::*;
use rustc_hir::{
    def::{CtorKind, CtorOf, DefKind},
    def_id::DefId,
};

#[derive(Serialize)]
pub(super) struct Observation {
    definition: Definition,
    owner: Definition,
    arguments: Vec<Structural>,
    signature: Signature,
}

#[derive(Serialize)]
struct Signature {
    input: Box<Structural>,
    output: Box<Structural>,
    bound_variables: usize,
    abi: String,
    safety: String,
    variadic: bool,
}

struct Constructor<'tcx> {
    definition: DefId,
    owner: DefId,
    arguments: GenericArgsRef<'tcx>,
    signature: ty::FnSig<'tcx>,
}

pub(super) fn capture<'tcx>(
    tcx: TyCtxt<'tcx>,
    value: Ty<'tcx>,
    depth: usize,
    budget: &Budget,
) -> Result<Structural, &'static str> {
    let constructor = selected(tcx, value)?;
    let arguments = nested_arguments(tcx, constructor.arguments, depth + 1, budget)?;
    let input = structural(tcx, constructor.signature.inputs()[0], depth + 1, budget)?;
    let output = structural(tcx, constructor.signature.output(), depth + 1, budget)?;

    Ok(Structural::TupleConstructor {
        observation: Box::new(Observation {
            definition: super::super::definition_record(tcx, constructor.definition),
            owner: super::super::definition_record(tcx, constructor.owner),
            arguments,
            signature: Signature {
                input: Box::new(input),
                output: Box::new(output),
                bound_variables: 0,
                abi: constructor.signature.abi().to_string(),
                safety: format!("{:?}", constructor.signature.safety()),
                variadic: constructor.signature.c_variadic(),
            },
        }),
    })
}

fn selected<'tcx>(tcx: TyCtxt<'tcx>, value: Ty<'tcx>) -> Result<Constructor<'tcx>, &'static str> {
    let ty::FnDef(definition, arguments) = value.kind() else {
        return Err("constructor observation requires a function definition");
    };
    if tcx.def_kind(*definition) != DefKind::Ctor(CtorOf::Struct, CtorKind::Fn) {
        return Err("function definition is not a struct tuple constructor");
    }
    let arguments = arguments.skip_binder();
    let bound = tcx.fn_sig(*definition).instantiate(tcx, arguments);

    if !bound.skip_normalization().bound_vars().is_empty() {
        return Err("tuple constructor signature has bound variables");
    }
    let signature = tcx
        .try_normalize_erasing_regions(ty::TypingEnv::fully_monomorphized(), bound)
        .map_err(|_| "tuple constructor signature normalization failed")?
        .skip_binder();
    let owner = exact_owner(tcx, *definition, signature)?;

    Ok(Constructor {
        definition: *definition,
        owner,
        arguments,
        signature,
    })
}

fn exact_owner<'tcx>(
    tcx: TyCtxt<'tcx>,
    definition: DefId,
    signature: ty::FnSig<'tcx>,
) -> Result<DefId, &'static str> {
    let [input] = signature.inputs() else {
        return Err("tuple constructor signature must be unary");
    };
    let ty::Adt(owner, arguments) = signature.output().kind() else {
        return Err("tuple constructor result is not an ADT");
    };
    if !owner.is_struct()
        || owner.did() != tcx.parent(definition)
        || owner.non_enum_variant().ctor_def_id() != Some(definition)
        || owner.non_enum_variant().fields.len() != 1
        || signature.c_variadic()
    {
        return Err("tuple constructor differs from its exact unary struct owner");
    }

    let field = owner.non_enum_variant().fields.iter().next().unwrap();
    let field = tcx
        .try_normalize_erasing_regions(
            ty::TypingEnv::fully_monomorphized(),
            field.ty(tcx, arguments),
        )
        .map_err(|_| "tuple constructor field normalization failed")?;
    if field != *input {
        return Err("tuple constructor input differs from its instantiated field");
    }
    Ok(owner.did())
}
