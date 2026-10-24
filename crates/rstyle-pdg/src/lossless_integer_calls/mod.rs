//! Exact reviewed total usize-to-u64 conversion and its immediate expect.
//! This contract gives no permission to construct, discard or unwrap other errors.
mod authority;
pub(crate) mod message;
mod selection;
mod source;

use crate::{HirNode, call_summaries::Summary, concrete_calls::ConcreteCall};
use crate::{concrete_types::TypeTree, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.lossless-usize-u64-calls";
pub(super) const CONVERT: &str = "std::convert::TryFrom::try_from";
pub(super) const IMPLEMENTATION: &str =
    "std::convert::num::ptr_try_from_impls::<impl std::convert::TryFrom<usize> for u64>::try_from";
pub(super) const EXPECT: &str = "std::result::Result::<T, E>::expect";
pub(super) const ERROR: &str = "std::num::TryFromIntError";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Convert,
    Expect,
}

impl Kind {
    pub(crate) fn construct(self) -> &'static str {
        match self {
            Self::Convert => "data.integer.try-usize-u64",
            Self::Expect => "data.integer.expect-ok-u64",
        }
    }

    fn path(self) -> &'static str {
        match self {
            Self::Convert => CONVERT,
            Self::Expect => EXPECT,
        }
    }
}

impl ConcreteCall for Kind {
    fn trees(self) -> (Vec<TypeTree>, TypeTree) {
        let result = TypeTree::IntConversionResult("u64".into());
        match self {
            Self::Convert => (vec![TypeTree::Scalar("usize".into())], result),
            Self::Expect => (
                vec![result, TypeTree::Shared(Box::new(TypeTree::Str))],
                TypeTree::Scalar("u64".into()),
            ),
        }
    }
}

pub(crate) fn kind(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Kind> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return None;
    }
    let dispatch = node.dispatch.as_ref()?;
    if authority::path(&dispatch.op, CONVERT)
        && dispatch.operand_tys == ["usize"]
        && matches!(crate::data_aggregates::parsed_type(node).ok()?,
            crate::data_types::TypeToken::IntConversionResult(value) if value == "u64")
    {
        Some(Kind::Convert)
    } else if authority::path(&dispatch.op, EXPECT)
        && matches!(crate::data_aggregates::parsed_type(node.children.first()?).ok()?,
            crate::data_types::TypeToken::IntConversionResult(value) if value == "u64")
    {
        Some(Kind::Expect)
    } else {
        None
    }
}

pub(crate) fn resolve(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Summary>, String> {
    let Some(kind) = kind(node, facts) else {
        return Ok(None);
    };
    kind.verify_types(node, facts, kind == Kind::Convert)?;
    let registry = authority::verify(node, kind, facts)?;
    selection::verify(node, kind, facts, registry)?;
    source::verify(node, kind, facts)?;

    if kind == Kind::Expect {
        let producer = node
            .children
            .first()
            .ok_or("Missing lossless expect receiver")?;
        if self::kind(producer, facts) != Some(Kind::Convert) {
            return Err("Lossless expect requires its immediate proved Ok producer".into());
        }
        resolve(producer, facts)?.ok_or("Missing lossless expect producer proof")?;
    }

    let (argument_types, result_type) = kind.signature()?;
    Ok(Some(Summary {
        id: format!("rstyle.{}", kind.construct()),
        version: "1.0.0".into(),
        callee_path: node.dispatch.as_ref().unwrap().op.clone(),
        callee_crate: "core".into(),
        argument_types,
        result_type,
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "reviewed-64-bit-usize-to-u64-always-ok".into(),
    }))
}

#[cfg(test)]
mod tests;
