//! Exact reviewed total integer conversions and their immediate expect.
//! This contract gives no permission to construct, discard or unwrap other errors.
mod authority;
pub(crate) mod message;
mod scoped_u16;
mod scoped_u32;
mod u16_source;
pub(crate) use scoped_u16::verify as verify_scoped_u16_conversion;
mod selection;
pub(crate) use scoped_u32::verify as verify_scoped_u32_conversion;
mod source;
mod types;

use crate::{HirNode, call_summaries::Summary, concrete_calls::ConcreteCall};
use crate::{concrete_types::TypeTree, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.lossless-usize-u64-calls";
pub(crate) const U32_USIZE_CAPABILITY: &str = "rust.lossless-u32-usize-calls";
pub(super) const CONVERT: &str = "std::convert::TryFrom::try_from";
pub(super) const IMPLEMENTATION: &str =
    "std::convert::num::ptr_try_from_impls::<impl std::convert::TryFrom<usize> for u64>::try_from";
pub(super) const U32_USIZE_IMPLEMENTATION: &str =
    "std::convert::num::ptr_try_from_impls::<impl std::convert::TryFrom<u32> for usize>::try_from";
pub(super) const EXPECT: &str = "std::result::Result::<T, E>::expect";
pub(super) const ERROR: &str = "std::num::TryFromIntError";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Convert,
    Expect,
    ConvertU32Usize,
    ExpectUsize,
    ConvertU64Usize,
    ExpectU64Usize,
}

impl Kind {
    pub(crate) fn construct(self) -> &'static str {
        match self {
            Self::Convert => "data.integer.try-usize-u64",
            Self::Expect => "data.integer.expect-ok-u64",
            Self::ConvertU32Usize => "data.integer.lossless-try-u32-usize",
            Self::ExpectUsize => "data.integer.lossless-expect-ok-usize",
            Self::ConvertU64Usize => "data.integer.lossless-try-u64-usize",
            Self::ExpectU64Usize => "data.integer.lossless-expect-u64-usize",
        }
    }

    fn path(self) -> &'static str {
        match self {
            Self::Convert | Self::ConvertU32Usize | Self::ConvertU64Usize => CONVERT,
            Self::Expect | Self::ExpectUsize | Self::ExpectU64Usize => EXPECT,
        }
    }
}

impl ConcreteCall for Kind {
    fn trees(self) -> (Vec<TypeTree>, TypeTree) {
        let result = TypeTree::IntConversionResult(self.output().into());
        if self.is_conversion() {
            (vec![TypeTree::Scalar(self.input().into())], result)
        } else {
            (
                vec![result, TypeTree::Shared(Box::new(TypeTree::Str))],
                TypeTree::Scalar(self.output().into()),
            )
        }
    }
}

pub(crate) fn kind(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Kind> {
    let dispatch = node.dispatch.as_ref()?;
    [
        Kind::Convert,
        Kind::Expect,
        Kind::ConvertU32Usize,
        Kind::ExpectUsize,
        Kind::ConvertU64Usize,
        Kind::ExpectU64Usize,
    ]
    .into_iter()
    .find(|kind| {
        crate::extraction_profile::facts_support(facts, kind.capability())
            && authority::path(&dispatch.op, kind.path())
            && kind.matches(node)
    })
}

pub(crate) fn resolve(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Summary>, String> {
    let Some(kind) = kind(node, facts) else {
        return Ok(None);
    };
    kind.verify_types(node, facts, kind.is_conversion())?;
    let registry = authority::verify(node, kind, facts)?;
    selection::verify(node, kind, facts, registry)?;
    source::verify(node, kind, facts)?;

    if !kind.is_conversion() {
        let producer = node
            .children
            .first()
            .ok_or("Missing lossless expect receiver")?;
        if self::kind(producer, facts) != Some(kind.conversion()) {
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
        overflow_behavior: kind.overflow_behavior().into(),
    }))
}

#[cfg(test)]
mod tests;
