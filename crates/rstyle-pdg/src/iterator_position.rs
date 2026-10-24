//! One fresh pinned shared-byte iterator and an exact shared-capture predicate.
mod source;
mod types;

use crate::{HirNode, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.byte-iterator-position";
pub(crate) const PREFIX: &str = "iterator.byte-position-prefix";
pub(crate) const KEY: &str = "rstyle.data:byte-position";
pub(crate) const RESULT_TOKEN: &str =
    "{\"kind\":\"option\",\"value\":{\"kind\":\"scalar\",\"value\":\"usize\"}}";

pub struct Checked<'a> {
    pub call: &'a HirNode,
    pub iterator: &'a HirNode,
    pub receiver: &'a HirNode,
    pub callback: &'a HirNode,
    pub predicate: crate::option_callbacks::Predicate<'a>,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::data_receivers::candidate(
        node,
        facts,
        crate::data_receivers::InvocationSyntax {
            capability: CAPABILITY,
            kind: "expr_method_call",
            paths: &[
                "core::iter::Iterator::position",
                "std::iter::Iterator::position",
            ],
        },
    )
}

pub(crate) fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::selected_operations(facts, &["expr_method_call"], candidate)
}

pub(crate) fn iterator_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::data_receivers::candidate(
        node,
        facts,
        crate::data_receivers::InvocationSyntax {
            capability: CAPABILITY,
            kind: "expr_method_call",
            paths: &[
                "core::slice::<impl [T]>::iter",
                "std::slice::<impl [T]>::iter",
            ],
        },
    )
}

pub fn verify<'a>(node: &'a HirNode, facts: &'a dyn EvidenceFacts) -> Result<Checked<'a>, String> {
    if !candidate(node, facts) || node.expansion.is_some() {
        return Err("Byte position requires an original registered invocation".into());
    }
    let [iterator, callback] = node.children.as_slice() else {
        return Err("Byte position requires iterator then callback".into());
    };
    let [receiver] = iterator.children.as_slice() else {
        return Err("Byte position requires one fresh slice iterator receiver".into());
    };
    let [pattern, _] = callback.children.as_slice() else {
        return Err("Byte position requires a complete predicate".into());
    };
    if pattern.kind != "pat_shared_copy" {
        return Err("Byte position requires an explicit shared-byte copy pattern".into());
    }

    types::verify(node, iterator, receiver, facts)?;
    source::verify(node, iterator, facts)?;
    let predicate = crate::option_callbacks::verify_predicate(callback, facts)?;
    Ok(Checked {
        call: node,
        iterator,
        receiver,
        callback,
        predicate,
    })
}

pub(crate) fn result_type() -> String {
    RESULT_TOKEN.into()
}

pub(crate) fn source_resolved(
    node: &HirNode,
    hir: &crate::HirRef,
    facts: &dyn EvidenceFacts,
) -> bool {
    if node.hir != *hir || node.kind != "expr_method_call" {
        return false;
    }
    if candidate(node, facts) {
        return verify(node, facts).is_ok();
    }
    if !iterator_candidate(node, facts) {
        return false;
    }
    sources(facts).into_iter().any(|call| {
        verify(call, facts).is_ok_and(|checked| {
            node.hir == *hir && (node.hir == checked.call.hir || node.hir == checked.iterator.hir)
        })
    })
}

pub(crate) fn semantics() -> serde_json::Value {
    serde_json::from_str(r#"{
        "rule":"fresh-shared-byte-position", "receiver_operand":0, "captured_byte_operand":1,
        "direction":"ascending", "start_index":0, "read_prefix":"through-first-equal-else-entire-view",
        "empty":"none-no-element-read", "result_index":"relative-to-view",
        "callback":"copy-shared-u8-then-builtin-equality", "repeated_reads":true, "mutates_input":false
    }"#).expect("static iterator semantics")
}

pub(crate) fn locations(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Vec<crate::field_locations::derived::Located>, String> {
    let checked = verify(node, facts)?;
    let base = if crate::data_subslices::candidate(checked.receiver, facts) {
        crate::data_subslices::verify(checked.receiver, facts)?.base
    } else {
        checked.receiver
    };
    let roots = crate::data_origins::origins(base, facts)?;
    let mut locations = crate::field_locations::derived::referents(base, facts, roots)?;
    if locations.is_empty() {
        return Err("Byte position has no complete receiver origins".into());
    }

    for location in &mut locations {
        location
            .path
            .push(crate::field_locations::Projection::IteratorPrefix {
                operand_position: 0,
            });
        location.value = crate::concrete_types::TypeTree::Slice(Box::new(
            crate::concrete_types::TypeTree::Scalar("u8".into()),
        ));
    }
    Ok(locations)
}
