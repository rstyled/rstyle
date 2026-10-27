//! Immediate Err-only callbacks move a checked generated error into its constructor.
mod authority;
pub mod capture;
mod source;

use crate::{HirNode, concrete_types::TypeTree, unit_store::EvidenceFacts};

pub const CAPABILITY: &str = "rust.named-error-id-reader-flow";
pub const ENVIRONMENT: &str = "id-map.static-environment";
pub const SELECTOR: &str = "id-map.result-is-err";
pub const PAYLOAD: &str = "id-map.error-payload";
pub const BINDING: &str = "id-map.error-binding";
pub const MESSAGE: &str = "id-map.captured-message";
pub const SUCCESS: &str = "id-map.success-payload";
pub const ERROR: &str = "id-map.wrap-error";
pub const OK: &str = "id-map.wrap-success";
pub const RESULT: &str = "id-map.result-merge";

pub(crate) struct Checked<'a> {
    pub source: &'a HirNode,
    pub receiver: &'a HirNode,
    pub closure: &'a HirNode,
    pub binding: &'a HirNode,
    pub constructor: &'a HirNode,
    pub error_read: &'a HirNode,
    pub message_read: &'a HirNode,
    pub message: &'a crate::ParamFact,
    pub input: TypeTree,
    pub output: TypeTree,
    pub storage: TypeTree,
    pub transfer: crate::named_error_payloads::owned::Transfer,
}

pub fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    if !matches!(node.children.as_slice(), [_, closure] if closure.kind == "expr_closure")
        || !crate::data_receivers::candidate(
            node,
            facts,
            crate::data_receivers::InvocationSyntax {
                capability: CAPABILITY,
                kind: "expr_method_call",
                paths: &["core::result::Result::<T, E>::map_err"],
            },
        )
    {
        return false;
    }

    facts.signature.return_ty.normalized.as_deref().is_some_and(|text|
        matches!(crate::data_types::parse(text), Ok(crate::data_types::TypeToken::Result { ok, .. })
            if crate::id_reader_types::shape(&ok)))
}

pub(crate) fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::helper_option_callbacks::method_sources(facts, candidate)
}

/// Replay the independent local callback and complete checked-wrapper contracts.
/// This does not replace complete reader verification or strict pair admission.
pub fn verify_documents<'a>(
    pair: impl Into<crate::producer::PairView<'a>>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let pair = pair.into();

    crate::data_documents::id_error_maps::verify(pair, facts)?;

    crate::data_documents::id_reader_checks::verify(pair, facts)?;

    Ok(())
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    let mut checked = source::check(node, facts)?;
    authority::verify(&checked, facts)?;
    capture::verify(&mut checked, facts)?;
    source::verify(&checked, facts)?;

    Ok(checked)
}

impl Checked<'_> {
    pub(crate) fn success(&self) -> &TypeTree {
        let TypeTree::Result { ok, .. } = &self.input else {
            unreachable!()
        };
        ok
    }

    pub(crate) fn error(&self) -> &TypeTree {
        let TypeTree::Result { err, .. } = &self.input else {
            unreachable!()
        };
        err
    }
}
