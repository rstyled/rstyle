//! Caller receipts must be exact members of the retained original owning unit.
use super::{Result, site, transport::*, types};
use crate::{HirFnFacts, HirNode, HirRef, generated_sources::capture::Definition};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
mod tests;

pub(super) fn verify(
    original: &HirFnFacts,
    projection: &Projection,
    receipts: &[Receipt],
    callers: &[HirFnFacts],
) -> Result<()> {
    if receipts.is_empty() || receipts.len() > 4096 || callers.len() > 65536 {
        return Err("Slice instance caller receipts are empty or exceed budget".into());
    }
    let mut seen = BTreeSet::new();
    for receipt in receipts {
        let matches: Vec<_> = callers
            .iter()
            .filter(|caller| {
                caller
                    .build
                    .frontend_options
                    .get("definition_identity")
                    .and_then(Value::as_str)
                    == Some(receipt.caller.as_str())
            })
            .collect();
        let [caller] = matches.as_slice() else {
            return Err("Slice instance receipt caller is absent or ambiguous".into());
        };

        build(original, caller)?;
        if receipt.caller_source_digest != caller.source.digest
            || receipt.caller_fn_span != caller.fn_span
        {
            return Err("Slice instance receipt changes its original caller source".into());
        }
        let hir: HirRef =
            serde_json::from_value(receipt.call["hir"].clone()).map_err(|e| e.to_string())?;
        if !seen.insert((receipt.caller.clone(), site(&hir)))
            || inventory_call(caller, &hir)? != &receipt.call
        {
            return Err(
                "Slice instance receipt is duplicated or differs from the original call inventory"
                    .into(),
            );
        }
        inventory_member(caller, &receipt.call, &projection.definition)?;
    }

    if seen != expected(original, &projection.definition, callers)? {
        return Err("Slice instance envelope omits or invents a concrete caller receipt".into());
    }
    Ok(())
}

fn expected(
    original: &HirFnFacts,
    definition: &Definition,
    callers: &[HirFnFacts],
) -> Result<BTreeSet<(String, super::Site)>> {
    let mut expected = BTreeSet::new();
    for caller in callers {
        for observed in inventory(caller)? {
            if observed["status"] != "resolved"
                || observed["requested"]["definition_identity"] != definition.definition_identity
            {
                continue;
            }
            let arguments: Vec<Argument> =
                serde_json::from_value(observed["requested_arguments"].clone())
                    .map_err(|e| e.to_string())?;
            if types::arguments_match(&arguments, &[types::builtin("u8")]).is_err() {
                continue;
            }
            let hir: HirRef =
                serde_json::from_value(observed["hir"].clone()).map_err(|e| e.to_string())?;
            inventory_member(caller, observed, definition)?;
            build(original, caller)?;
            let identity = caller
                .build
                .frontend_options
                .get("definition_identity")
                .and_then(Value::as_str)
                .ok_or("Missing selected slice caller identity")?;
            if inventory_call(caller, &hir)? != observed
                || !expected.insert((identity.into(), site(&hir)))
            {
                return Err("Selected slice call inventory is duplicated or inconsistent".into());
            }
        }
    }
    Ok(expected)
}

pub(super) fn inventory_call<'a>(facts: &'a HirFnFacts, hir: &HirRef) -> Result<&'a Value> {
    let calls = inventory(facts)?;
    let mut matches = calls.iter().filter(|call| {
        serde_json::from_value::<HirRef>(call["hir"].clone())
            .ok()
            .as_ref()
            == Some(hir)
    });
    let call = matches
        .next()
        .ok_or("Original call-instance receipt is missing")?;
    if matches.next().is_some() {
        return Err("Original call-instance receipt is duplicated".into());
    }

    Ok(call)
}

fn inventory(facts: &HirFnFacts) -> Result<&[Value]> {
    let inventory = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Missing original call-instance inventory")?;
    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
    {
        return Err("Original call-instance inventory is incomplete".into());
    }

    inventory["calls"]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "Missing original call-instance roster".into())
}

/// Inventory membership does not grant authority to request or link a call.
fn inventory_member(caller: &HirFnFacts, observed: &Value, definition: &Definition) -> Result<()> {
    let (hir, span) = observation(observed, definition)?;
    if let Some(node) = find(&caller.body, &hir) {
        return retained_call(node, span);
    }
    unavailable_for(caller, &hir, span)
}

fn call(node: &HirNode, observed: &Value, definition: &Definition) -> Result<()> {
    let (hir, span) = observation(observed, definition)?;
    if hir != node.hir {
        return Err("Slice instance receipt changes its exact original call identity".into());
    }
    retained_call(node, span)
}

fn retained_call(node: &HirNode, span: crate::ByteSpan) -> Result<()> {
    if node.kind != "expr_call"
        || node.expansion.is_some()
        || node.children.len() != 3
        || span != node.span
    {
        return Err("Slice instance receipt lacks its complete original local call".into());
    }
    Ok(())
}

fn observation(observed: &Value, definition: &Definition) -> Result<(HirRef, crate::ByteSpan)> {
    let requested: Definition =
        serde_json::from_value(observed["requested"].clone()).map_err(|e| e.to_string())?;
    let selected: Definition =
        serde_json::from_value(observed["selected"].clone()).map_err(|e| e.to_string())?;
    if observed["expanded"] != false
        || observed["status"] != "resolved"
        || requested != *definition
        || selected != *definition
        || observed["signature"]["display_only"] != true
    {
        return Err(
            "Slice instance receipt does not select the exact original local function".into(),
        );
    }

    for key in ["requested_arguments", "selected_arguments"] {
        let arguments: Vec<Argument> =
            serde_json::from_value(observed[key].clone()).map_err(|e| e.to_string())?;
        types::arguments_match(&arguments, &[types::builtin("u8")])?;
    }
    let hir = serde_json::from_value(observed["hir"].clone()).map_err(|e| e.to_string())?;
    let span = serde_json::from_value(observed["span"].clone()).map_err(|e| e.to_string())?;
    Ok((hir, span))
}

/// The opaque retained for-desugaring accounts for an unavailable observation;
/// it certifies neither its hidden call nor any supported prefix of the caller.
fn unavailable_for(caller: &HirFnFacts, hir: &HirRef, span: crate::ByteSpan) -> Result<()> {
    if hir.krate != caller.body.hir.krate
        || hir.owner != caller.body.hir.owner
        || !caller.source.bytes.matches_identity(&caller.source.digest)
    {
        return Err("Missing slice call changes its original HIR owner".into());
    }
    let source = crate::source_evidence::byte_range(&caller.source.bytes, span)?;
    if !matches!(syn::parse_str::<syn::Expr>(source), Ok(syn::Expr::Call(call)) if call.args.len() == 3)
    {
        return Err(
            "Unavailable slice observation is not an exact original three-argument call".into(),
        );
    }

    let mut enclosing = Vec::new();
    opaque_for_nodes(&caller.body, span, &mut enclosing);
    let [node] = enclosing.as_slice() else {
        return Err("Missing slice call lacks one explicit enclosing opaque for boundary".into());
    };
    deferred_for(caller, node)
}

fn opaque_for_nodes<'a>(node: &'a HirNode, span: crate::ByteSpan, found: &mut Vec<&'a HirNode>) {
    if node.kind == "expr_match"
        && node.children.is_empty()
        && node.span.start < span.start
        && span.start < span.end
        && span.end < node.span.end
        && node.res.is_none()
        && node.dispatch.is_none()
        && !node.is_unsafe_block
    {
        found.push(node);
    }
    for child in &node.children {
        opaque_for_nodes(child, span, found);
    }
}

fn deferred_for(caller: &HirFnFacts, node: &HirNode) -> Result<()> {
    let source = crate::source_evidence::byte_range(&caller.source.bytes, node.span)?;
    if node.hir.krate != caller.body.hir.krate
        || node.hir.owner != caller.body.hir.owner
        || node.snippet != source
        || !node
            .expansion
            .as_ref()
            .is_some_and(|expansion| expansion.callsite == node.span && expansion.snippet == source)
        || !matches!(
            syn::parse_str::<syn::Expr>(source),
            Ok(syn::Expr::ForLoop(_))
        )
    {
        return Err("Unavailable slice boundary lacks its exact original for desugaring".into());
    }

    let findings: Vec<_> = caller
        .source_diagnostics
        .iter()
        .filter(|finding| finding.span == node.span && finding.construct == "source_deferred")
        .collect();
    let [finding] = findings.as_slice() else {
        return Err("Unavailable slice boundary lacks one exact deferred source finding".into());
    };
    if finding.capability != "rust.additional-types"
        || finding.severity != "error"
        || finding.snippet != source
        || finding.reason
            != "Original owned source contains deferred rust.additional-types; cfg removal cannot certify a supported remainder."
    {
        return Err("Unavailable slice boundary changes its original deferred finding".into());
    }
    Ok(())
}

pub(super) fn find<'a>(node: &'a HirNode, hir: &HirRef) -> Option<&'a HirNode> {
    if node.hir == *hir {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, hir))
}

fn build(original: &HirFnFacts, caller: &HirFnFacts) -> Result<()> {
    let a = &original.build;
    let b = &caller.build;
    if a.producer_revision != b.producer_revision
        || a.rustc_version != b.rustc_version
        || a.rustc_commit != b.rustc_commit
        || a.edition != b.edition
        || a.target_triple != b.target_triple
        || a.pointer_width != b.pointer_width
        || a.data_layout != b.data_layout
        || a.cfgs != b.cfgs
        || a.features != b.features
        || a.overflow_checks != b.overflow_checks
        || a.debug_assertions != b.debug_assertions
        || a.panic_strategy != b.panic_strategy
        || a.opt_level != b.opt_level
        || a.frontend_options.get("rstyle.rust.compiled-crate")
            != b.frontend_options.get("rstyle.rust.compiled-crate")
    {
        return Err("Slice instance caller and original owner builds differ".into());
    }
    Ok(())
}

pub(super) struct Requests<'a> {
    pub definition: &'a Definition,
    pub receipts: &'a [Receipt],
}

impl Requests<'_> {
    pub fn requested(
        &self,
        original: &HirFnFacts,
        caller: &HirFnFacts,
        node: &HirNode,
    ) -> Result<bool> {
        let caller_identity = caller
            .build
            .frontend_options
            .get("definition_identity")
            .and_then(Value::as_str)
            .ok_or("Missing original caller definition")?;
        let matches: Vec<_> = self
            .receipts
            .iter()
            .filter(|receipt| {
                receipt.caller == caller_identity
                    && serde_json::from_value::<HirRef>(receipt.call["hir"].clone())
                        .ok()
                        .as_ref()
                        == Some(&node.hir)
            })
            .collect();
        if matches.is_empty() {
            return Ok(false);
        }
        let [receipt] = matches.as_slice() else {
            return Err("Duplicated selected slice caller receipt".into());
        };

        build(original, caller)?;
        if receipt.caller_source_digest != caller.source.digest
            || receipt.caller_fn_span != caller.fn_span
            || find(&caller.body, &node.hir) != Some(node)
            || inventory_call(caller, &node.hir)? != &receipt.call
        {
            return Err(
                "Selected slice call loses its original owning source/build/inventory".into(),
            );
        }

        call(node, &receipt.call, self.definition)?;
        Ok(true)
    }
}
