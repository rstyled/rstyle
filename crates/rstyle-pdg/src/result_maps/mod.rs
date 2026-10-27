//! Bounded fresh Result transport by actual unary tuple constructors.
mod authority;
mod model;
mod ownership;
pub(crate) mod request;
mod shape;
pub(crate) mod source;
pub(crate) mod trait_leaf;
pub(crate) mod types;

use crate::{HirNode, concrete_types::TypeTree, unit_store::EvidenceFacts};
pub const CAPABILITY: &str = "rust.constructor-result-maps";
pub const RESULT: &str = "map.result-merge";
pub(crate) use request::TransportRequest;

pub(crate) struct CheckedTransport<'a> {
    pub stages: [CheckedMap<'a>; 2],
}

pub(crate) struct CheckedMap<'a> {
    pub source: &'a HirNode,
    pub receiver: &'a HirNode,
    pub callback: &'a HirNode,
    pub input: TypeTree,
    pub output: TypeTree,
    pub maps_error: bool,
}

impl CheckedMap<'_> {
    pub(crate) fn role(&self) -> &'static str {
        if self.maps_error {
            "result-map-error-stage"
        } else {
            "result-map-success-stage"
        }
    }

    pub(crate) fn payload_types(&self) -> [&TypeTree; 2] {
        let TypeTree::Result { ok, err } = &self.input else {
            unreachable!()
        };
        [ok, err]
    }
    pub(crate) fn constructed_type(&self) -> &TypeTree {
        let TypeTree::Result { ok, err } = &self.output else {
            unreachable!()
        };
        if self.maps_error { err } else { ok }
    }
}

pub fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    if crate::widening_readers::merge_candidate(node, facts)
        || crate::id_error_maps::candidate(node, facts)
    {
        return false;
    }
    crate::extraction_profile::gated_node(node, facts, CAPABILITY, |node| {
        node.kind == "expr_method_call"
            && node.res.as_ref().is_some_and(|resolution| {
                resolution.kind == "resolved_callee"
                    && serde_json::from_str::<crate::data_library::Callee>(&resolution.description)
                        .is_ok_and(|callee| {
                            matches!(
                                callee.path.as_str(),
                                "core::result::Result::<T, E>::map"
                                    | "core::result::Result::<T, E>::map_err"
                            )
                        })
            })
    })
}

/// Rejection-only source-routing filter; full transport proof remains mandatory.
pub(crate) fn tail_candidate(facts: &dyn EvidenceFacts) -> bool {
    let [root] = facts.body.children.as_slice() else {
        return false;
    };

    facts.body.kind == "expr_block" && candidate(root, facts)
}

pub(crate) fn verify_chain(
    facts: &dyn EvidenceFacts,
) -> Result<Option<CheckedTransport<'_>>, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || !facts.body.any(&|node| candidate(node, facts))
    {
        return Ok(None);
    }
    let resolution = facts.nominal_resolution()?;
    let Some(source) = resolution.transport_source() else {
        return Ok(None);
    };
    source.verify(facts.facts())?;
    let chain = shape::chain(facts.facts())?;
    let request = source.request();
    authority::verify(facts.facts(), request, [chain.map, chain.root])?;
    ownership::verify(facts, &resolution, request, &chain)?;
    let [input, intermediate, output] = request.result_types();

    Ok(Some(CheckedTransport {
        stages: [
            CheckedMap {
                source: chain.map,
                receiver: chain.leaf,
                callback: chain.callbacks[0],
                input: input.clone(),
                output: intermediate.clone(),
                maps_error: false,
            },
            CheckedMap {
                source: chain.root,
                receiver: chain.map,
                callback: chain.callbacks[1],
                input: intermediate.clone(),
                output: output.clone(),
                maps_error: true,
            },
        ],
    }))
}

/// Source coverage requires final ownership/helper authority and the complete
/// frontend boundary, not merely a generated source or substitution receipt.
pub(crate) fn verify_source(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if verify_chain(facts)?.is_none() {
        return Ok(false);
    }
    crate::data_frontend::verify(facts)?;
    Ok(true)
}

/// Cover only the exact driver finding at the checked generated signature.
/// All neighboring and unknown findings retain their ordinary interpretation.
pub(crate) fn covers_source_finding(
    facts: &dyn EvidenceFacts,
    finding: &crate::HirDiagnostic,
) -> bool {
    let span = facts.signature.sig_span;
    finding.capability == "rust.macros"
        && finding.construct == "macro_generated_function"
        && finding.span == span
        && facts
            .source
            .bytes
            .get(span.start as usize..span.end as usize)
            == Some(finding.snippet.as_str())
        && finding.severity == "error"
        && finding.reason == "Macro-generated functions are deferred to rust.macros."
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<CheckedMap<'a>, String> {
    verify_chain(facts)?
        .and_then(|checked| {
            checked
                .stages
                .into_iter()
                .find(|stage| stage.source.hir == node.hir)
        })
        .ok_or_else(|| "Constructor Result map lacks its complete checked fresh chain".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrelated_function_does_not_require_nominal_transport_source() {
        let mut facts: crate::HirFnFacts = serde_json::from_str(include_str!(
            "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
        ))
        .unwrap();
        facts
            .build
            .frontend_options
            .remove(crate::nominal_boundary::census::KEY);
        assert!(facts.nominal_resolution().is_err());
        assert!(!facts.body.any(&|node| candidate(node, &facts)));
        assert!(verify_chain(&facts).unwrap().is_none());
        assert!(!verify_source(&facts).unwrap());
    }

    #[test]
    fn generated_source_coverage_matches_the_complete_preserved_driver_finding() {
        let mut facts: crate::HirFnFacts = serde_json::from_str(include_str!(
            "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
        ))
        .unwrap();
        facts.signature.sig_span = crate::ByteSpan { start: 0, end: 1 };
        facts.source.bytes = "m".into();
        let finding = crate::HirDiagnostic {
            capability: "rust.macros".into(),
            construct: "macro_generated_function".into(),
            span: facts.signature.sig_span,
            snippet: "m".into(),
            severity: "error".into(),
            reason: "Macro-generated functions are deferred to rust.macros.".into(),
        };
        assert!(covers_source_finding(&facts, &finding));
        for mutation in 0..6 {
            let mut changed = finding.clone();
            match mutation {
                0 => changed.capability = "rust.unsafe".into(),
                1 => changed.construct = "unknown_generated_construct".into(),
                2 => changed.span.end = 0,
                3 => changed.snippet = "different".into(),
                4 => changed.severity = "warning".into(),
                _ => changed.reason = "unknown source problem".into(),
            }
            assert!(
                !covers_source_finding(&facts, &changed),
                "mutation {mutation}"
            );
        }
    }
}
