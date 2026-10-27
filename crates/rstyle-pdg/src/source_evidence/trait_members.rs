//! Original trait member source anchors and active attribute validation.
use crate::{ByteSpan, HirFnFacts, nominal_boundary::census::expansions::FileSpan};

/// One original source checksum is shared by every anchor in a member roster.
pub(crate) struct SourceAnchors<'a> {
    facts: &'a HirFnFacts,
    checksum: String,
    mismatch: &'static str,
}

impl<'a> SourceAnchors<'a> {
    pub(crate) fn new(facts: &'a HirFnFacts, bytes: &str, mismatch: &'static str) -> Self {
        Self {
            facts,
            checksum: crate::source_identity::checksum(bytes.as_bytes()),
            mismatch,
        }
    }

    pub(crate) fn verify(&self, anchor: &FileSpan, span: ByteSpan) -> Result<(), String> {
        if anchor.span != span
            || anchor.digest != self.checksum
            || self
                .facts
                .build
                .frontend_options
                .get("source_file")
                .and_then(serde_json::Value::as_str)
                != anchor.path.to_str()
        {
            return Err(self.mismatch.into());
        }
        Ok(())
    }
}

pub(crate) struct AttributePolicy {
    pub inactive: &'static str,
    pub member: fn(&syn::ImplItem) -> &[syn::Attribute],
}

pub(crate) fn attributes(
    facts: &HirFnFacts,
    item: &syn::ItemImpl,
    policy: AttributePolicy,
    verify: fn(&HirFnFacts, &[syn::Attribute]) -> Result<(), String>,
) -> Result<(), String> {
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    let attributes =
        std::iter::once(item.attrs.as_slice()).chain(item.items.iter().map(policy.member));

    for attributes in attributes {
        if !configuration.active(attributes)? {
            return Err(policy.inactive.into());
        }
        verify(facts, attributes)?;
    }
    Ok(())
}
