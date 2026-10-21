//! Exact core authority and original receiver/default/callback arguments.

use super::{Predicate, verify_predicate};
use crate::{
    HirNode,
    concrete_types::{Observation, TypeTree},
    unit_store::EvidenceFacts,
};

pub struct Invocation<'a> {
    pub call: &'a HirNode,
    pub receiver: &'a HirNode,
    pub default: &'a HirNode,
    pub callback: &'a HirNode,
    pub predicate: Predicate<'a>,
}

impl<'a> Invocation<'a> {
    pub(crate) fn arguments(&self) -> Arguments<'a> {
        Arguments {
            call: self.call,
            receiver: self.receiver,
            default: self.default,
            callback: self.callback,
        }
    }
}

pub(crate) struct Arguments<'a> {
    pub call: &'a HirNode,
    pub receiver: &'a HirNode,
    pub default: &'a HirNode,
    pub callback: &'a HirNode,
}

/// This source contract does not certify a graph or activate a capability.
pub fn verify_map_or<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Invocation<'a>, String> {
    let Arguments {
        call,
        receiver,
        default,
        callback,
    } = verify_arguments(node, facts)?;
    let predicate = verify_predicate(callback, facts)?;
    Ok(Invocation {
        call,
        receiver,
        default,
        callback,
        predicate,
    })
}

pub(crate) fn verify_arguments<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Arguments<'a>, String> {
    let [receiver, default, callback] = node.children.as_slice() else {
        return Err("Immediate map_or requires receiver, eager default and callback".into());
    };
    if node.kind != "expr_method_call" || node.expansion.is_some() {
        return Err("Immediate map_or requires an original method invocation".into());
    }

    let callee = crate::data_library::callee(node)?;
    crate::data_library::verify_core_definition(
        &callee,
        facts,
        crate::data_library::CoreDefinition {
            paths: &[
                "std::option::Option::<T>::map_or",
                "core::option::Option::<T>::map_or",
            ],
            unavailable: "Unpinned callback compiler/target",
            mismatch: "Immediate map_or lacks the exact pinned core definition",
        },
    )?;
    crate::data_receivers::verify_dispatch(node, &callee.path, &["bool"])?;
    signature(node, facts)?;
    super::source::invocation(node, facts)?;

    Ok(Arguments {
        call: node,
        receiver,
        default,
        callback,
    })
}

fn signature(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let capture = crate::data_receivers::verify(facts)?;
    let record = capture
        .methods
        .iter()
        .find(|method| method.call == node.hir)
        .ok_or("Missing immediate map_or signature observation")?;
    let option = Observation::Captured {
        ty: TypeTree::Option(Box::new(TypeTree::Scalar("u8".into()))),
    };
    let boolean = Observation::Captured {
        ty: TypeTree::Scalar("bool".into()),
    };
    let [input, default, callback] = record.signature_inputs.as_slice() else {
        return Err("Immediate map_or signature loses an input".into());
    };

    if record.receiver_type != option
        || record.adjusted_type != option
        || !record.adjustments.is_empty()
        || *input != option
        || *default != boolean
        || !matches!(callback, Observation::Unsupported { reason } if !reason.is_empty())
        || record.signature_result != boolean
    {
        return Err(
            "Immediate map_or requires owned Option<u8>, bool and a separately checked callback"
                .into(),
        );
    }
    let option_token =
        crate::data_types::token(&TypeTree::Option(Box::new(TypeTree::Scalar("u8".into()))))?;
    for (child, expected) in node.children[..2]
        .iter()
        .zip([option_token.as_str(), "bool"])
    {
        if child.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(expected) {
            return Err("Immediate map_or operands disagree with its concrete signature".into());
        }
    }
    if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("bool") {
        return Err("Immediate map_or requires a bool result".into());
    }
    Ok(())
}
