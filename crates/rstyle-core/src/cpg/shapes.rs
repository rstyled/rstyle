//! Parameter lists, signature shapes and mutated types (the profile's Pattern facts).

use super::model::{Scope, Symbol, TargetKind, Type, TypeKind};
use super::source::Sources;
use super::tree::{Body, Tables};
use crate::facts::{ParamFacts, Receiver, SigFacts, TyShape};
use std::collections::BTreeSet;

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
        TypeKind::Integer => primitive(&integer(ty)?),
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
        TypeKind::Function => Ok(TyShape::FnPtr),
        TypeKind::Interface => Ok(TyShape::Dyn(match ty.definition.as_deref() {
            Some(principal) => tables.definition(principal)?.stable_identity.clone(),
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
    let identity = tables.definition(definition)?.stable_identity.clone();
    let arguments = shapes(tables, ty.arguments.as_deref().unwrap_or_default())?;
    Ok(Some((identity, arguments)))
}

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

fn integer(ty: &Type) -> Result<String, String> {
    let sign = if ty.signed == Some(true) { "i" } else { "u" };
    Ok(match ty.pointer_sized {
        Some(true) => format!("{sign}size"),
        _ => format!("{sign}{}", bits(ty)?),
    })
}

/// Name, written type and use count of each parameter, receiver excluded.
pub fn param_list(body: &Body<'_>, sources: &Sources<'_>) -> Result<Vec<ParamFacts>, String> {
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
            types.insert(definition.stable_identity.clone());
        }
    }
    Ok(types.into_iter().collect())
}
