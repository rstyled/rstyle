//! Contextual permissions derived from the complete original function receipt.
use super::*;
use crate::data_types::TypeToken;

/// Retain the raw scanner finding; cover only the exact independently checked
/// slice pattern, while all other source findings remain exclusions.
pub(crate) fn source_findings(
    facts: &dyn EvidenceFacts,
) -> Result<Option<Vec<crate::HirDiagnostic>>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    let checked = verify(facts)?;
    let pattern = checked.prefix().prefix().pattern();
    let mut covered = false;
    let mut remaining = Vec::new();
    for finding in &facts.source_diagnostics {
        if !covered && covers_pattern(finding, pattern) {
            covered = true;
        } else {
            remaining.push(finding.clone());
        }
    }

    Ok(Some(remaining))
}

pub(super) fn covers_pattern(finding: &crate::HirDiagnostic, pattern: &HirNode) -> bool {
    finding.capability == "rust.scalar-interface"
        && finding.construct == "binding_pattern"
        && finding.span == pattern.span
        && finding.snippet == pattern.snippet
        && finding.severity == "error"
        && finding.reason
            == "Source construct is deferred to rust.scalar-interface; remove it or assess it under a later policy."
}

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    if !crate::extraction_profile::facts_support(facts, VALUE_CAPABILITY)
        || facts.body.kind != "expr_block"
        || !matches!(facts.body.children.as_slice(), [matched] if matched.kind == "expr_match")
    {
        return false;
    }
    let Some(text) = facts.signature.return_ty.normalized.as_deref() else {
        return false;
    };
    let Ok(TypeToken::Option(payload)) = crate::data_types::parse(text) else {
        return false;
    };

    matches!(*payload, TypeToken::ScalarProductInstance { .. })
}

pub(crate) fn permits_type(text: &str, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !candidate(facts) || !text.contains("\"kind\":\"scalar_product_instance\"") {
        return Ok(false);
    }
    let checked = verify(facts)?;
    let product = TypeToken::from_tree(&TypeTree::ScalarProductInstance(Box::new(
        checked.instance().clone(),
    )));
    let ty = crate::data_types::parse(text)?;
    if ty != product && ty != TypeToken::Option(Box::new(product)) {
        return Err("Digit product type differs from its exact guarded function instance".into());
    }

    Ok(true)
}

/// Select only source-checked special forms; ordinary scalar operands still
/// undergo the existing frontend, eligibility and hazard checks.
pub(crate) fn operands<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Vec<&'a HirNode>>, String> {
    if !candidate(facts) || !special(node) {
        return Ok(None);
    }
    let checked = verify(facts)?;
    let selected = if node == checked.prefix().matched() {
        vec![
            &node.children[0],
            &node.children[1].children[1],
            &node.children[2].children[1],
        ]
    } else if node == checked.constructor().node() || node == checked.some() {
        node.children.iter().collect()
    } else if node == checked.prefix().failure() {
        Vec::new()
    } else {
        return Ok(None);
    };
    Ok(Some(selected))
}

pub(crate) fn construct(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<&'static str>, String> {
    if !candidate(facts) || node.kind == "expr_match" || !special(node) {
        return Ok(None);
    }
    let checked = verify(facts)?;
    Ok(if node == checked.constructor().node() {
        Some("data.scalar-product")
    } else if node == checked.some() {
        Some("data.digit-some")
    } else if node == checked.prefix().failure() {
        Some("data.digit-none")
    } else {
        None
    })
}

fn special(node: &HirNode) -> bool {
    node.kind == "expr_match"
        || node.res.as_ref().is_some_and(|resolution| {
            matches!(
                node.kind.as_str(),
                "expr_call" | "expr_struct" | "expr_path"
            ) && matches!(
                resolution.kind.as_str(),
                "resolved_option_variant" | super::constructors::RESOLUTION
            )
        })
}
