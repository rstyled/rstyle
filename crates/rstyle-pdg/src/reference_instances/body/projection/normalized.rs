//! Independently check canonical enrichment instead of trusting raw type displays.
use super::{Nominals, Result, Role, Site};
use crate::{HirFnFacts, HirNode, ResolvedTy, data_types::TypeToken};
use std::collections::BTreeMap;

pub(super) use crate::reference_instances::shared_bytes;

pub(super) fn verify(
    canonical: &HirFnFacts,
    roles: &BTreeMap<Site, Role>,
    nominals: &Nominals<'_>,
    haystack: &str,
) -> Result<()> {
    node(&canonical.signature.params[0].pat, roles, nominals)?;
    node(&canonical.body, roles, nominals)?;
    check(&canonical.signature.params[0].ty, Role::Formal, nominals)?;
    check(&canonical.signature.return_ty, Role::Input, nominals)?;

    for binding in &canonical.bindings {
        let role = roles
            .get(&super::site(&binding.hir))
            .ok_or("Constructor normalized binding has no original source role")?;
        check(&binding.ty, *role, nominals)?;
    }
    if !matches!(haystack, "str" | "[u8]") {
        return Err(
            "Constructor normalization requires exact concrete haystack substitution".into(),
        );
    }
    Ok(())
}

fn node(node: &HirNode, roles: &BTreeMap<Site, Role>, nominals: &Nominals<'_>) -> Result<()> {
    if let Some(ty) = &node.ty {
        let role = roles
            .get(&super::site(&node.hir))
            .ok_or("Constructor normalized value has no complete source role")?;
        check(ty, *role, nominals)?;
    }
    for child in &node.children {
        self::node(child, roles, nominals)?;
    }
    Ok(())
}

fn check(ty: &ResolvedTy, role: Role, nominals: &Nominals<'_>) -> Result<()> {
    if role == Role::Formal {
        return if ty.normalized.is_none() && ty.observed == "&H" {
            Ok(())
        } else {
            Err("Original generic formal must retain unsupported H normalization".into())
        };
    }

    let text = ty
        .normalized
        .as_deref()
        .ok_or("Missing canonical concrete normalization for a supported constructor node")?;
    let expected = match role {
        Role::Bytes => shared_bytes(),
        Role::Input => TypeToken::from_tree(nominals.input_type),
        Role::Span => TypeToken::from_tree(nominals.span_type),
        Role::Anchored => TypeToken::from_tree(nominals.anchored_type),
        Role::Usize => TypeToken::Scalar("usize".into()),
        Role::Bool => TypeToken::Scalar("bool".into()),
        Role::Formal => return Err("Unexpected generic normalization role".into()),
    };
    let actual = if matches!(text, "usize" | "bool") {
        TypeToken::Scalar(text.into())
    } else {
        serde_json::from_str(text).map_err(|error| error.to_string())?
    };
    if actual != expected {
        return Err(
            "Constructor canonical normalized token differs from its source-selected component"
                .into(),
        );
    }
    Ok(())
}
