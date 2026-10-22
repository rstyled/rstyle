//! Bounded structural observations; displays never identify nominal types.
use super::Definition;
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt};
use serde::Serialize;
use std::cell::Cell;

#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum Arguments {
    Listed(Vec<Argument>),
    Oversized {
        identity_status: &'static str,
        reason: &'static str,
        count: usize,
    },
}

impl Arguments {
    pub(super) fn complete(&self) -> bool {
        matches!(self, Self::Listed(arguments) if arguments.iter().all(|argument| matches!(argument.identity, Identity::Known { .. })))
    }
}

#[derive(Serialize)]
pub(super) struct Argument {
    kind: &'static str,
    value: String,
    display_only: bool,
    #[serde(flatten)]
    identity: Identity,
}

#[derive(Serialize)]
#[serde(tag = "identity_status", rename_all = "snake_case")]
enum Identity {
    Known { identity: Structural },
    Unsupported { reason: &'static str },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Structural {
    Builtin {
        name: String,
    },
    Str,
    Reference {
        region: &'static str,
        mutable: bool,
        referent: Box<Structural>,
    },
    Slice {
        element: Box<Structural>,
    },
    Tuple {
        elements: Vec<Structural>,
    },
    Adt {
        definition: Definition,
        arguments: Vec<Structural>,
    },
    ErasedRegion,
}

pub(super) fn arguments<'tcx>(tcx: TyCtxt<'tcx>, arguments: GenericArgsRef<'tcx>) -> Arguments {
    if arguments.len() > 256 {
        return Arguments::Oversized {
            identity_status: "unsupported",
            reason: "argument count exceeds observation limit",
            count: arguments.len(),
        };
    }

    let budget = Budget(Cell::new(256));
    Arguments::Listed(
        arguments
            .iter()
            .map(|argument| argument_record(tcx, argument, &budget))
            .collect(),
    )
}

fn argument_record<'tcx>(
    tcx: TyCtxt<'tcx>,
    argument: ty::GenericArg<'tcx>,
    budget: &Budget,
) -> Argument {
    let (kind, value, observed) = match argument.kind() {
        ty::GenericArgKind::Lifetime(_) => (
            "lifetime",
            "erased".into(),
            budget.charge(0).map(|_| Structural::ErasedRegion),
        ),
        ty::GenericArgKind::Type(value) => {
            ("type", value.to_string(), structural(tcx, value, 0, budget))
        }
        ty::GenericArgKind::Const(value) => (
            "const",
            value.to_string(),
            Err("const argument identity is outside this observation"),
        ),
    };
    let identity = match observed {
        Ok(identity) => Identity::Known { identity },
        Err(reason) => Identity::Unsupported { reason },
    };

    Argument {
        kind,
        value,
        display_only: true,
        identity,
    }
}

struct Budget(Cell<usize>);

impl Budget {
    fn charge(&self, depth: usize) -> Result<(), &'static str> {
        let remaining = self.0.get();
        if depth >= 32 || remaining == 0 {
            return Err("structural type observation limit exceeded");
        }

        self.0.set(remaining - 1);
        Ok(())
    }
}

fn structural<'tcx>(
    tcx: TyCtxt<'tcx>,
    value: Ty<'tcx>,
    depth: usize,
    budget: &Budget,
) -> Result<Structural, &'static str> {
    budget.charge(depth)?;
    match value.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Never => {
            Ok(Structural::Builtin {
                name: value.to_string(),
            })
        }
        ty::Str => Ok(Structural::Str),
        ty::Ref(_, inner, mutability) => Ok(Structural::Reference {
            region: "erased",
            mutable: mutability.is_mut(),
            referent: Box::new(structural(tcx, *inner, depth + 1, budget)?),
        }),
        ty::Slice(inner) => Ok(Structural::Slice {
            element: Box::new(structural(tcx, *inner, depth + 1, budget)?),
        }),
        ty::Tuple(elements) => Ok(Structural::Tuple {
            elements: elements
                .iter()
                .map(|element| structural(tcx, element, depth + 1, budget))
                .collect::<Result<_, _>>()?,
        }),
        ty::Adt(definition, arguments) => Ok(Structural::Adt {
            definition: super::definition_record(tcx, definition.did()),
            arguments: nested_arguments(tcx, arguments, depth + 1, budget)?,
        }),
        _ => Err("type form is outside structural instance identity"),
    }
}

fn nested_arguments<'tcx>(
    tcx: TyCtxt<'tcx>,
    arguments: GenericArgsRef<'tcx>,
    depth: usize,
    budget: &Budget,
) -> Result<Vec<Structural>, &'static str> {
    if arguments.len() > budget.0.get() {
        return Err("nested argument count exceeds observation limit");
    }

    arguments
        .iter()
        .map(|argument| match argument.kind() {
            ty::GenericArgKind::Lifetime(_) => {
                budget.charge(depth).map(|_| Structural::ErasedRegion)
            }
            ty::GenericArgKind::Type(value) => structural(tcx, value, depth, budget),
            ty::GenericArgKind::Const(_) => {
                Err("const argument identity is outside this observation")
            }
        })
        .collect()
}
