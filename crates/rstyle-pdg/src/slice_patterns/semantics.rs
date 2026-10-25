//! Saved-input prefix semantics; source proof never substitutes for graph replay.
mod contract;
mod matched;
#[cfg(test)]
mod tests;
#[cfg(test)]
use contract::Destination;
mod values;

use super::source::CheckedPrefix;
use crate::{HirNode, unit_store::EvidenceFacts};
pub(crate) use contract::{Contract, Region, SelectorKind};

pub(crate) struct Checked<'a> {
    matched: &'a HirNode,
    scrutinee: &'a HirNode,
    formal: &'a HirNode,
    prefix: CheckedPrefix<'a>,
    success: &'a HirNode,
    failure: &'a HirNode,
    digits: Vec<Digit<'a>>,
    remainder: &'a HirNode,
}

pub(crate) struct Digit<'a> {
    binding: &'a HirNode,
    range: &'a HirNode,
    pointer: &'a HirNode,
    read: &'a HirNode,
}

impl<'a> Checked<'a> {
    pub(crate) fn matched(&self) -> &'a HirNode {
        self.matched
    }
    pub(crate) fn scrutinee(&self) -> &'a HirNode {
        self.scrutinee
    }
    pub(crate) fn formal(&self) -> &'a HirNode {
        self.formal
    }
    pub(crate) fn prefix(&self) -> &CheckedPrefix<'a> {
        &self.prefix
    }
    pub(crate) fn success(&self) -> &'a HirNode {
        self.success
    }
    pub(crate) fn failure(&self) -> &'a HirNode {
        self.failure
    }
    pub(crate) fn digits(&self) -> &[Digit<'a>] {
        &self.digits
    }
    pub(crate) fn remainder(&self) -> &'a HirNode {
        self.remainder
    }
    pub(crate) fn consumed(&self) -> u8 {
        self.digits.len() as u8
    }
    pub(crate) fn contract(&self) -> Contract {
        contract::derive(self)
    }
}

impl<'a> Digit<'a> {
    pub(crate) fn binding(&self) -> &'a HirNode {
        self.binding
    }
    pub(crate) fn range(&self) -> &'a HirNode {
        self.range
    }
    pub(crate) fn pointer(&self) -> &'a HirNode {
        self.pointer
    }
    pub(crate) fn read(&self) -> &'a HirNode {
        self.read
    }
}

/// Call only after selecting this registered whole-function frontier. This
/// checks source semantics, not product ownership, arithmetic or serialized CFG.
pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    if !crate::extraction_profile::facts_support(facts, super::CAPABILITY) {
        return Err("Prefix semantics require their registered source capability".into());
    }
    let (scrutinee, success, failure) = matched::verify(node, facts)?;
    let pattern = &node.children[1].children[0];
    let prefix = super::source::verify(pattern, facts)?;
    let digits = values::digits(&prefix, success, facts)?;
    let remainder = values::remainder(&prefix, success, facts)?;

    Ok(Checked {
        matched: node,
        scrutinee,
        formal: &facts.signature.params[0].pat,
        prefix,
        success,
        failure,
        digits,
        remainder,
    })
}
