//! Copy dependencies may occur only inside a distinct shared owner. No value of
//! the dependency is loaded, constructed, returned, dispatched on or dropped.
//! Exact source/field resolution and pinned builtin Copy are checked by callers.
use super::*;
use crate::{HirNode, ResolvedTy, data_types::TypeToken};

pub(super) const CAPABILITY: &str = "rust.passive-copy-dependencies";

pub(super) fn verify(facts: &HirFnFacts, tree: &TypeTree) -> Result<bool, String> {
    if !supports(facts, CAPABILITY)? || !concrete_types::closed_payload(tree) {
        return Ok(false);
    }
    let Some((nominal, _)) = tree.nominal_parts() else {
        return Ok(false);
    };
    let uses = Uses { nominal };
    let mut seen = false;

    for ty in facts
        .signature
        .params
        .iter()
        .map(|parameter| &parameter.ty)
        .chain([&facts.signature.return_ty])
    {
        let Some(occurs) = uses.ty(ty)? else {
            return Ok(false);
        };
        seen |= occurs;
    }
    Ok(uses
        .body(&facts.body, facts)?
        .is_some_and(|occurs| occurs || seen))
}

struct Uses<'a> {
    nominal: &'a concrete_types::Nominal,
}

impl Uses<'_> {
    // None denies permission; Some records whether the dependency occurs.
    fn ty(&self, ty: &ResolvedTy) -> Result<Option<bool>, String> {
        let Some(text) = &ty.normalized else {
            return Ok(None);
        };
        Ok(self.token(&crate::data_types::parse(text)?, false))
    }

    fn body(&self, node: &HirNode, facts: &HirFnFacts) -> Result<Option<bool>, String> {
        let observation = if node.kind == "expr_closure" {
            crate::option_callbacks::captures::verify_structure(node, facts)
                .ok()
                .and_then(|capture| crate::data_types::parse(&capture.token).ok())
                .and_then(|ty| self.token(&ty, false))
        } else {
            node.ty.as_ref().map_or(Ok(Some(false)), |ty| self.ty(ty))?
        };
        let Some(mut seen) = observation else {
            return Ok(None);
        };
        for child in &node.children {
            let Some(occurs) = self.body(child, facts)? else {
                return Ok(None);
            };
            seen |= occurs;
        }
        Ok(Some(seen))
    }

    fn token(&self, ty: &TypeToken, borrowed: bool) -> Option<bool> {
        match ty {
            TypeToken::Struct { nominal, fields } => {
                if nominal == self.nominal {
                    return borrowed.then_some(true);
                }
                self.children(fields.iter().map(|(_, field)| field), borrowed)
            }
            TypeToken::MixedEnum { variants, .. } => self.children(
                variants.iter().filter_map(|(_, shape)| shape.field()), borrowed),
            TypeToken::Enum { variants, .. } => self.children(
                variants.iter().map(|(_, field)| field), borrowed),
            TypeToken::Shared(inner) => self.token(inner,
                borrowed || matches!(inner.as_ref(), TypeToken::Struct { nominal, .. } if nominal != self.nominal)),
            TypeToken::Tuple(values) => self.children(values.iter(), borrowed),
            TypeToken::Slice(inner) | TypeToken::Option(inner)
            | TypeToken::Array { element: inner, .. } => self.token(inner, borrowed),
            _ => Some(false),
        }
    }

    fn children<'a>(
        &self,
        mut fields: impl Iterator<Item = &'a TypeToken>,
        borrowed: bool,
    ) -> Option<bool> {
        fields.try_fold(false, |seen, field| {
            Some(self.token(field, borrowed)? || seen)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nominal(name: &str) -> concrete_types::Nominal {
        concrete_types::Nominal {
            source_identity: Some("original-source".into()),
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: concrete_types::digest(b"manifest"),
            definition_path: format!("sample::{name}"),
            declaration_digest: concrete_types::digest(name.as_bytes()),
        }
    }

    fn dependency() -> TypeToken {
        TypeToken::Struct {
            nominal: nominal("Dependency"),
            fields: vec![("0".into(), TypeToken::Scalar("u32".into()))],
        }
    }

    #[test]
    fn only_a_distinct_shared_struct_protects_a_dependency_value() {
        let dependency = dependency();
        let owner = TypeToken::Struct {
            nominal: nominal("Owner"),
            fields: vec![("unused".into(), dependency.clone())],
        };
        for (ty, expected) in [
            (dependency.clone(), false),
            (TypeToken::Shared(Box::new(dependency.clone())), false),
            (owner.clone(), false),
            (TypeToken::Shared(Box::new(owner.clone())), true),
            (TypeToken::Tuple(vec![owner.clone()]), false),
            (
                TypeToken::Tuple(vec![
                    TypeToken::Shared(Box::new(owner.clone())),
                    dependency.clone(),
                ]),
                false,
            ),
            (
                TypeToken::Tuple(vec![dependency.clone(), TypeToken::Shared(Box::new(owner))]),
                false,
            ),
            (TypeToken::Slice(Box::new(dependency.clone())), false),
            (TypeToken::Option(Box::new(dependency.clone())), false),
            (
                TypeToken::Array {
                    element: Box::new(dependency),
                    length: 1,
                },
                false,
            ),
        ] {
            let target = nominal("Dependency");
            let uses = Uses { nominal: &target };
            assert_eq!(uses.token(&ty, false), expected.then_some(true), "{ty:?}");
        }
    }

    #[test]
    fn missing_type_is_no_permission_and_unrelated_type_is_no_dependency() {
        let target = nominal("Dependency");
        let uses = Uses { nominal: &target };
        let ty = ResolvedTy {
            spelling: "unknown".into(),
            observed: "unknown".into(),
            normalized: None,
        };
        assert_eq!(uses.ty(&ty).unwrap(), None);
        assert_eq!(
            uses.token(&TypeToken::Scalar("u32".into()), false),
            Some(false)
        );
    }
}
