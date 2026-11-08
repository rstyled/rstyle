//! Parameter lists, signature shapes and mutated types (the profile's Pattern facts).

use super::model::{Callable, Function, Scope, Symbol, TargetKind, Type, TypeKind};
use super::source::Sources;
use super::tree::{Body, Tables};
use crate::facts::{ParamFacts, Receiver, Scalar, SigFacts, TyShape};
use std::collections::BTreeSet;

/// What a function's receiver, parameters and result are, in that order, whatever anything spells
/// them as: a spelling can hold ids local to one document. A type that can't be shaped has none.
pub fn signature_shapes(function: &Function, tables: &Tables<'_>) -> Vec<Option<TyShape>> {
    let signature = &function.signature;
    signature
        .receiver
        .iter()
        .map(|receiver| &receiver.ty)
        .chain(signature.parameters.iter().map(|parameter| &parameter.ty))
        .chain(std::iter::once(&signature.result))
        .map(|id| shape(tables, id).ok())
        .collect()
}

/// The structural shape of a type.
pub fn shape(tables: &Tables<'_>, id: &str) -> Result<TyShape, String> {
    let ty = tables.ty(id)?;
    if let Some((id, args)) = nominal(tables, ty)? {
        return Ok(TyShape::Adt { id, args });
    }

    let inner = |field: &Option<String>, name: &str| -> Result<Box<TyShape>, String> {
        Ok(Box::new(shape(tables, required(ty, field, name)?)?))
    };
    let primitive = |name: &str| Ok::<_, String>(TyShape::Prim(name.into()));
    match ty.kind {
        TypeKind::Reference => Ok(TyShape::Ref {
            mutable: ty.mode.as_deref() == Some("exclusive"),
            inner: inner(&ty.target, "target")?,
        }),
        TypeKind::Tuple => Ok(TyShape::Tuple(shapes(
            tables,
            ty.items.as_deref().unwrap_or_default(),
        )?)),
        TypeKind::Unit => Ok(TyShape::Tuple(vec![])),
        TypeKind::Sequence if ty.fixed == Some(true) => {
            Ok(TyShape::Array(inner(&ty.element, "element")?))
        }
        TypeKind::Sequence => Ok(TyShape::Slice(inner(&ty.element, "element")?)),
        TypeKind::Integer => primitive(&integer(tables, ty)?),
        TypeKind::Float => primitive(&format!("f{}", bits(ty)?)),
        TypeKind::Bool => primitive("bool"),
        TypeKind::Char => primitive("char"),
        TypeKind::String => primitive("str"),
        TypeKind::Bottom => primitive("!"),
        TypeKind::Parameter => {
            let index = ty
                .index
                .ok_or_else(|| format!("Type parameter {} has no index", ty.id))?;
            Ok(TyShape::Param(index as usize))
        }
        // The type of one named function (a function item) is not a pointer.
        TypeKind::Function if ty.definition.is_some() => Ok(TyShape::Other),
        TypeKind::Function => Ok(TyShape::FnPtr),
        TypeKind::Interface => Ok(TyShape::Dyn(match ty.definition.as_deref() {
            Some(principal) => type_identity(tables.definition(principal)?),
            None => String::new(),
        })),
        TypeKind::Opaque if ty.bounds.as_ref().is_some_and(|bounds| !bounds.is_empty()) => {
            Ok(TyShape::Opaque)
        }
        _ => Ok(TyShape::Other),
    }
}

fn shapes(tables: &Tables<'_>, ids: &[String]) -> Result<Vec<TyShape>, String> {
    ids.iter().map(|id| shape(tables, id)).collect()
}

/// A record, sum or owned pointer with a declaration: its identity and argument shapes.
fn nominal(tables: &Tables<'_>, ty: &Type) -> Result<Option<(String, Vec<TyShape>)>, String> {
    let Some(definition) = ty.definition.as_deref().filter(|_| is_nominal(ty)) else {
        return Ok(None);
    };
    let identity = type_identity(tables.definition(definition)?);
    let arguments = shapes(tables, ty.arguments.as_deref().unwrap_or_default())?;
    Ok(Some((identity, arguments)))
}

/// A type's identity: its definition's stable identity, without the marker that tells a
/// type apart from a value item of the same path, since a type is always a type.
fn type_identity(definition: &super::model::Definition) -> String {
    let identity = &definition.stable_identity;
    identity
        .strip_suffix(TYPE_NAMESPACE)
        .unwrap_or(identity)
        .to_owned()
}

/// The marker a stable identity carries when a type shares a value item's path.
const TYPE_NAMESPACE: &str = "#type";

fn is_nominal(ty: &Type) -> bool {
    matches!(ty.kind, TypeKind::Record | TypeKind::Sum) || is_owned_pointer(ty)
}

fn is_owned_pointer(ty: &Type) -> bool {
    ty.kind == TypeKind::Pointer && ty.mode.as_deref() == Some("owned")
}

fn required<'t>(ty: &Type, field: &'t Option<String>, name: &str) -> Result<&'t str, String> {
    field
        .as_deref()
        .ok_or_else(|| format!("Type {} has no {name}", ty.id))
}

fn bits(ty: &Type) -> Result<u32, String> {
    ty.bits
        .ok_or_else(|| format!("Type {} has no width", ty.id))
}

fn integer(tables: &Tables<'_>, ty: &Type) -> Result<String, String> {
    let sign = if ty.signed == Some(true) { "i" } else { "u" };
    Ok(match ty.pointer_sized {
        Some(true) => format!("{sign}size"),
        _ if ty.bits.is_none() && tables.unbounded_integers() => "int".into(),
        _ => format!("{sign}{}", bits(ty)?),
    })
}

/// What a parameter of this type holds when that's a plain value, however it's passed.
fn scalar_of(tables: &Tables<'_>, id: &str) -> Result<Option<Scalar>, String> {
    let mut ty = tables.ty(id)?;
    while ty.kind == TypeKind::Reference {
        ty = tables.ty(required(ty, &ty.target, "target")?)?;
    }

    Ok(match ty.kind {
        TypeKind::String => Some(Scalar::Text),
        TypeKind::Integer => Some(Scalar::Integer),
        TypeKind::Float => Some(Scalar::Float),
        TypeKind::Bool => Some(Scalar::Bool),
        TypeKind::Char => Some(Scalar::Char),
        _ => None,
    })
}

/// Name, written type, use count and scalar of each parameter, receiver excluded.
pub fn param_list(
    body: &Body<'_>,
    tables: &Tables<'_>,
    sources: &Sources<'_>,
) -> Result<Vec<ParamFacts>, String> {
    let function = body.function;
    function
        .signature
        .parameters
        .iter()
        .map(|parameter| {
            let ty = match &parameter.annotation {
                Some(span) => collapse_whitespace(sources.between(span, span)?),
                None => String::new(),
            };
            let uses = function
                .refs
                .iter()
                .filter_map(|reference| reference.target.as_ref())
                .filter(|target| {
                    target.kind == TargetKind::Symbol
                        && target.id.as_deref() == Some(parameter.symbol.as_str())
                })
                .count();
            Ok(ParamFacts {
                name: body.symbol(&parameter.symbol)?.name.clone(),
                ty,
                uses,
                scalar: scalar_of(tables, &parameter.ty)?,
            })
        })
        .collect()
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The shape of a function an implementation declares for a type; `None` otherwise.
pub fn signature_shape(body: &Body<'_>, tables: &Tables<'_>) -> Result<Option<SigFacts>, String> {
    let signature = &body.function.signature;
    let Some(owner) = signature.owner_type.as_deref() else {
        return Ok(None);
    };

    let owner = tables.ty(owner)?;
    let (self_ty, self_args) = match nominal(tables, owner)? {
        Some((id, args)) => (Some(id), args),
        None => (None, vec![]),
    };
    let receiver = match &signature.receiver {
        Some(receiver) => receiver_of(tables, tables.ty(&receiver.ty)?, owner)?,
        None => Receiver::None,
    };
    let params = signature
        .parameters
        .iter()
        .map(|parameter| shape(tables, &parameter.ty))
        .collect::<Result<_, _>>()?;
    let modifier = |name: &str| signature.modifiers.iter().any(|m| m == name);
    Ok(Some(SigFacts {
        self_ty,
        self_args,
        receiver,
        params,
        ret: shape(tables, &signature.result)?,
        is_async: modifier("async"),
        is_unsafe: modifier("unsafe"),
        is_const: modifier("const"),
        generics: signature.generics.len(),
    }))
}

/// The shape a call to a function or method sees; `None` for other definitions.
pub fn callable_shape(
    tables: &Tables<'_>,
    callable: Option<&Callable>,
) -> Result<Option<SigFacts>, String> {
    let Some(callable) = callable else {
        return Ok(None);
    };
    let owner = callable
        .owner_type
        .as_deref()
        .map(|id| tables.ty(id))
        .transpose()?;
    let nominal_owner = match owner {
        Some(owner) => nominal(tables, owner)?.map(|shape| (owner, shape)),
        None => None,
    };
    let receiver = match (&callable.receiver, &nominal_owner) {
        (None, _) => Receiver::None,
        (Some(receiver), Some((owner, _))) => receiver_of(tables, tables.ty(receiver)?, owner)?,
        (Some(receiver), None) => by_reference(tables.ty(receiver)?),
    };
    let (self_ty, self_args) = match nominal_owner {
        Some((_, (id, args))) => (Some(id), args),
        None => (None, vec![]),
    };
    let params = callable
        .parameters
        .iter()
        .map(|parameter| shape(tables, parameter))
        .collect::<Result<_, _>>()?;
    let modifier = |name: &str| callable.modifiers.iter().any(|m| m == name);

    Ok(Some(SigFacts {
        self_ty,
        self_args,
        receiver,
        params,
        ret: shape(tables, &callable.result)?,
        is_async: modifier("async"),
        is_unsafe: modifier("unsafe"),
        is_const: modifier("const"),
        generics: callable.generics,
    }))
}

/// A receiver with no nominal owner to compare with, by its own reference shape.
fn by_reference(ty: &Type) -> Receiver {
    match (ty.kind, ty.mode.as_deref()) {
        (TypeKind::Reference, Some("exclusive")) => Receiver::RefMut,
        (TypeKind::Reference, _) => Receiver::Ref,
        _ => Receiver::Value,
    }
}

/// How the receiver's type takes the owner: by value, by reference, or otherwise.
fn receiver_of(tables: &Tables<'_>, ty: &Type, owner: &Type) -> Result<Receiver, String> {
    let declared = owner.definition.as_deref().filter(|_| is_nominal(owner));
    let is_owner =
        |ty: &Type| declared.is_some() && is_nominal(ty) && ty.definition.as_deref() == declared;
    if ty.kind == TypeKind::Reference {
        let target = tables.ty(required(ty, &ty.target, "target")?)?;
        if is_owner(target) {
            return Ok(match ty.mode.as_deref() {
                Some("exclusive") => Receiver::RefMut,
                _ => Receiver::Ref,
            });
        }
    }
    Ok(match is_owner(ty) {
        true => Receiver::Value,
        false => Receiver::Other,
    })
}

/// Workspace records and sums written through the mutated symbols, by stable identity.
pub fn mutated_types<'a>(
    tables: &Tables<'_>,
    symbols: impl IntoIterator<Item = &'a Symbol>,
) -> Result<Vec<String>, String> {
    let mut types = BTreeSet::new();
    for symbol in symbols {
        let id = symbol
            .ty
            .as_deref()
            .ok_or_else(|| format!("Symbol {} is mutated but has no type", symbol.id))?;
        let mut ty = tables.ty(id)?;
        while ty.kind == TypeKind::Reference {
            ty = tables.ty(required(ty, &ty.target, "target")?)?;
        }
        if is_owned_pointer(ty) {
            ty = tables.ty(required(ty, &ty.target, "target")?)?;
        }
        let Some(definition) = ty
            .definition
            .as_deref()
            .filter(|_| matches!(ty.kind, TypeKind::Record | TypeKind::Sum))
        else {
            continue;
        };
        let definition = tables.definition(definition)?;
        if matches!(definition.scope, Scope::Unit | Scope::Workspace) {
            types.insert(type_identity(definition));
        }
    }
    Ok(types.into_iter().collect())
}
