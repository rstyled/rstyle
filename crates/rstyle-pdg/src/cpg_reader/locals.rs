//! The data profile's local type observations: `&mut u32` formals, never-typed control
//! constructs, and how mutable `usize` formals are read and written.

use super::Context;
use crate::consumer_facts::{control_types, formal_types};
use crate::field_locations::FieldObservations;
use crate::field_locations::scalar_formals::{
    Formal, FormalObservation, ScalarObservation, observe_accesses,
};
use crate::frontend::{HirFnFacts, HirRef};
use rstyle_core::cpg::model::TypeKind;

impl Context<'_, '_> {
    pub(super) fn local_observations(&self, facts: &mut HirFnFacts) -> Result<(), String> {
        let types = Types(self.hir_types());
        self.formal_types(facts, &types)?;
        self.control_types(facts, &types)?;
        self.scalar_formals(facts, &types)
    }

    fn formal_types(&self, facts: &mut HirFnFacts, ty: &Types<'_>) -> Result<(), String> {
        if facts.signature.params.len() > formal_types::MAX_FORMALS {
            return Ok(());
        }
        let mutable_u32: Vec<_> = facts
            .signature
            .params
            .iter()
            .filter(|parameter| {
                ty.of(&parameter.pat.hir)
                    .is_some_and(|id| self.mutable_reference_to(id, "u32"))
            })
            .map(|parameter| formal_types::Formal {
                parameter: parameter.position,
                binding: parameter.pat.hir.clone(),
            })
            .collect();
        if mutable_u32.is_empty() {
            return Ok(());
        }
        let observations = formal_types::Observations {
            identity: formal_types::IDENTITY.into(),
            mutable_u32,
        };
        let value = serde_json::to_value(observations).map_err(|error| error.to_string())?;
        facts
            .build
            .frontend_options
            .insert(formal_types::KEY.into(), value);

        Ok(())
    }

    fn control_types(&self, facts: &mut HirFnFacts, ty: &Types<'_>) -> Result<(), String> {
        let mut never = Vec::new();
        let mut pending = vec![&facts.body];
        while let Some(node) = pending.pop() {
            let bottom = ty
                .of(&node.hir)
                .and_then(|id| self.reader.ty(id).ok())
                .is_some_and(|ty| ty.kind == TypeKind::Bottom);
            if control_types::control_kind(&node.kind)
                && node.expansion.is_none()
                && node.ty.as_ref().is_some_and(|ty| ty.normalized.is_none())
                && bottom
            {
                never.push(control_types::Control {
                    hir: node.hir.clone(),
                    span: node.span,
                    kind: node.kind.clone(),
                });
            }
            pending.extend(&node.children);
        }
        if never.is_empty() {
            return Ok(());
        }
        let observations = control_types::Observations {
            identity: control_types::IDENTITY.into(),
            never,
        };
        let value = serde_json::to_value(observations).map_err(|error| error.to_string())?;
        facts
            .build
            .frontend_options
            .insert(control_types::KEY.into(), value);

        Ok(())
    }

    fn scalar_formals(&self, facts: &mut HirFnFacts, ty: &Types<'_>) -> Result<(), String> {
        if !facts
            .build
            .frontend_options
            .contains_key(crate::field_locations::KEY)
        {
            return Ok(());
        }
        let formals: Vec<_> = facts
            .signature
            .params
            .iter()
            .map(|parameter| Formal {
                parameter: parameter.position,
                binding: parameter.pat.hir.clone(),
                observation: self.formal_observation(ty.of(&parameter.pat.hir)),
            })
            .collect();
        if !formals.iter().any(Formal::mutable_usize) {
            return Ok(());
        }
        let evidence = observe_accesses(facts, formals);
        let mut fields = facts
            .build
            .frontend_options
            .data::<FieldObservations>()?
            .into_owned();
        fields.scalar_formal_accesses = Some(evidence);
        facts.build.frontend_options.insert_data(fields);

        Ok(())
    }

    fn formal_observation(&self, ty: Option<&str>) -> FormalObservation {
        let Some(ty) = ty.and_then(|ty| self.reader.ty(ty).ok()) else {
            return FormalObservation::Unknown {
                reason: "Missing compiler formal type".into(),
            };
        };
        let referent = || {
            if self.reader.integer(ty.target.as_deref(), "usize") {
                ScalarObservation::Captured {
                    scalar: "usize".into(),
                }
            } else {
                ScalarObservation::Unsupported {
                    reason: "Referent is outside the observed usize location family".into(),
                }
            }
        };
        match (ty.kind, ty.mode.as_deref()) {
            (TypeKind::Reference, Some("shared")) => FormalObservation::Shared {
                referent: referent(),
            },
            (TypeKind::Reference, Some("exclusive")) => FormalObservation::Mutable {
                referent: referent(),
            },
            _ => FormalObservation::NonReference,
        }
    }

    fn mutable_reference_to(&self, id: &str, scalar: &str) -> bool {
        self.reader.ty(id).is_ok_and(|ty| {
            ty.kind == TypeKind::Reference
                && ty.mode.as_deref() == Some("exclusive")
                && self.reader.integer(ty.target.as_deref(), scalar)
        })
    }
}

/// Each HIR identity's type ID.
struct Types<'a>(std::collections::BTreeMap<(u32, u32), &'a str>);

impl Types<'_> {
    fn of(&self, reference: &HirRef) -> Option<&str> {
        self.0.get(&(reference.owner, reference.local)).copied()
    }
}
