//! Symbolic original types retain generic/lifetime relationships before erasure.
use super::super::diagnostic_support::ArgumentsProjection;
use super::super::{Definition, comparisons, definition_record, types};
use super::Projection;
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt};
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(in crate::hir_extraction::instances) enum Type {
    Parameter {
        index: u32,
        name: String,
    },
    Builtin {
        name: String,
    },
    Str,
    Slice {
        element: Box<Type>,
    },
    Reference {
        lifetime: Region,
        mutable: bool,
        referent: Box<Type>,
    },
    Adt {
        definition: Definition,
        arguments: Vec<Argument>,
    },
    Tuple {
        elements: Vec<Type>,
    },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(in crate::hir_extraction::instances) enum Region {
    Parameter { index: u32, name: String },
    Static,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub(in crate::hir_extraction::instances) enum Argument {
    Type(Type),
    Lifetime(Region),
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(in crate::hir_extraction::instances) enum Clause {
    Trait {
        trait_definition: comparisons::CompilerDefinition,
        positive: bool,
        original_arguments: Vec<Argument>,
        substituted_arguments: types::Arguments,
    },
    TypeOutlives {
        original_type: Type,
        lifetime: Region,
        substituted_type: types::Arguments,
    },
    RegionOutlives {
        longer: Region,
        shorter: Region,
    },
}

pub(super) fn clause<'tcx>(
    tcx: TyCtxt<'tcx>,
    original: ty::Clause<'tcx>,
    projection: &Projection<'tcx>,
) -> Result<Clause, String> {
    let binder = original.kind();
    if !binder.bound_vars().is_empty() {
        return Err("Higher-ranked reference owner clause unsupported".into());
    }
    match binder.skip_binder() {
        ty::ClauseKind::Trait(bound) => {
            if bound.trait_ref.def_id.is_local() {
                return Err(
                    "Local trait provenance is unsupported before metadata encoding".into(),
                );
            }
            Ok(Clause::Trait {
                trait_definition: comparisons::definition(tcx, bound.trait_ref.def_id),
                positive: bound.polarity == ty::ClausePolarity::Positive,
                original_arguments: arguments(tcx, bound.trait_ref.args, 0)?,
                substituted_arguments: types::arguments(
                    tcx,
                    projection.args(tcx, bound.trait_ref.args)?,
                ),
            })
        }
        ty::ClauseKind::TypeOutlives(predicate) => {
            let (value, lifetime) = (predicate.0, predicate.1);
            Ok(Clause::TypeOutlives {
                original_type: ty(tcx, value)?,
                lifetime: region(lifetime)?,
                substituted_type: types::arguments(
                    tcx,
                    tcx.mk_args(&[projection.ty(tcx, value)?.into()]),
                ),
            })
        }
        ty::ClauseKind::RegionOutlives(predicate) => {
            let (longer, shorter) = (predicate.0, predicate.1);
            Ok(Clause::RegionOutlives {
                longer: region(longer)?,
                shorter: region(shorter)?,
            })
        }
        _ => Err("Reference owner clause kind is outside the complete diagnostic grammar".into()),
    }
}

pub(super) fn ty<'tcx>(tcx: TyCtxt<'tcx>, value: Ty<'tcx>) -> Result<Type, String> {
    tree(tcx, value, 0)
}

fn tree<'tcx>(tcx: TyCtxt<'tcx>, value: Ty<'tcx>, depth: usize) -> Result<Type, String> {
    if depth >= 16 {
        return Err("Original reference type exceeds structural depth budget".into());
    }
    Ok(match value.kind() {
        ty::Param(parameter) => Type::Parameter {
            index: parameter.index,
            name: parameter.name.to_string(),
        },
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Never => Type::Builtin {
            name: value.to_string(),
        },
        ty::Str => Type::Str,
        ty::Slice(element) => Type::Slice {
            element: Box::new(tree(tcx, *element, depth + 1)?),
        },
        ty::Ref(lifetime, referent, mutability) => Type::Reference {
            lifetime: region(*lifetime)?,
            mutable: mutability.is_mut(),
            referent: Box::new(tree(tcx, *referent, depth + 1)?),
        },
        ty::Adt(definition, args) => Type::Adt {
            definition: definition_record(tcx, definition.did()),
            arguments: arguments(tcx, args, depth + 1)?,
        },
        ty::Tuple(elements) if elements.len() <= 16 => Type::Tuple {
            elements: elements
                .iter()
                .map(|element| tree(tcx, element, depth + 1))
                .collect::<Result<_, _>>()?,
        },
        _ => return Err("Original reference type is outside symbolic structural grammar".into()),
    })
}

pub(in crate::hir_extraction::instances) fn arguments<'tcx>(
    tcx: TyCtxt<'tcx>,
    original: GenericArgsRef<'tcx>,
    depth: usize,
) -> Result<Vec<Argument>, String> {
    if original.len() > 16 || depth >= 16 {
        return Err("Original reference argument roster exceeds structural budget".into());
    }
    original
        .iter()
        .map(|argument| match argument.kind() {
            ty::GenericArgKind::Type(value) => tree(tcx, value, depth + 1).map(Argument::Type),
            ty::GenericArgKind::Lifetime(value) => region(value).map(Argument::Lifetime),
            _ => Err("Original const argument unsupported in reference owner".into()),
        })
        .collect()
}

fn region(region: ty::Region<'_>) -> Result<Region, String> {
    match region.kind() {
        ty::ReEarlyParam(parameter) => Ok(Region::Parameter {
            index: parameter.index,
            name: parameter.name.to_string(),
        }),
        ty::ReStatic => Ok(Region::Static),
        _ => Err("Original lifetime lacks an early source parameter/static identity".into()),
    }
}
