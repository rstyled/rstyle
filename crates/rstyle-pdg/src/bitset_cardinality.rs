//! Bounded primitive integer conversions and checked scalar operations.
mod authority;
mod source;
use crate::concrete_calls::ConcreteCall;
use crate::{
    HirNode, call_summaries::Summary, concrete_types::TypeTree, unit_store::EvidenceFacts,
};

pub(crate) const CAPABILITY: &str = "rust.bitset-cardinality";
pub(crate) const EXTENSION: &str = "rust.end-of-input";
pub(crate) const COUNT: &str = "core::num::<impl u32>::count_ones";
pub(crate) const CONVERT: &str = "std::convert::TryFrom::try_from";
pub(crate) const UNWRAP: &str = "std::result::Result::<T, E>::unwrap";
pub(crate) const IMPLEMENTATION: &str =
    "std::convert::num::ptr_try_from_impls::<impl std::convert::TryFrom<u32> for usize>::try_from";
pub(crate) const NARROW_IMPLEMENTATION: &str =
    "std::convert::num::ptr_try_from_impls::<impl std::convert::TryFrom<usize> for u16>::try_from";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Count,
    Convert,
    Unwrap,
    Narrow,
    UnwrapNarrow,
    CheckedSub,
    UnwrapOption,
}
impl Kind {
    pub fn construct(self) -> &'static str {
        match self {
            Self::Count => "data.bitset.count-u32",
            Self::Convert => "data.integer.try-u32-usize",
            Self::Unwrap => "data.integer.unwrap-ok-usize",
            Self::Narrow => "data.integer.try-usize-u16",
            Self::UnwrapNarrow => "data.integer.unwrap-u16",
            Self::CheckedSub => "data.integer.checked-sub-usize",
            Self::UnwrapOption => "data.integer.unwrap-option-usize",
        }
    }
    fn path(self) -> &'static str {
        match self {
            Self::Count => COUNT,
            Self::Convert | Self::Narrow => CONVERT,
            Self::Unwrap | Self::UnwrapNarrow => UNWRAP,
            Self::CheckedSub => "core::num::<impl usize>::checked_sub",
            Self::UnwrapOption => "std::option::Option::<T>::unwrap",
        }
    }
    fn implementation(self) -> Option<&'static str> {
        match self {
            Self::Convert => Some(IMPLEMENTATION),
            Self::Narrow => Some(NARROW_IMPLEMENTATION),
            _ => None,
        }
    }

    fn producer(self) -> Option<Self> {
        match self {
            Self::Unwrap => Some(Self::Convert),
            Self::UnwrapNarrow => Some(Self::Narrow),
            Self::UnwrapOption => Some(Self::CheckedSub),
            _ => None,
        }
    }
    fn overflow(self) -> &'static str {
        match self {
            Self::Narrow | Self::UnwrapNarrow => "checked-usize-to-u16-err-above-u16-max",
            Self::CheckedSub | Self::UnwrapOption => "checked-usize-sub-none-on-underflow",
            _ => "exact-u32-population-or-lossless-64-bit-conversion",
        }
    }
}

impl ConcreteCall for Kind {
    fn trees(self) -> (Vec<TypeTree>, TypeTree) {
        let scalar = |name: &str| TypeTree::Scalar(name.into());
        let result = |name: &str| TypeTree::IntConversionResult(name.into());
        let option = || TypeTree::Option(Box::new(scalar("usize")));
        match self {
            Self::Count => (vec![scalar("u32")], scalar("u32")),
            Self::Convert => (vec![scalar("u32")], result("usize")),
            Self::Unwrap => (vec![result("usize")], scalar("usize")),
            Self::Narrow => (vec![scalar("usize")], result("u16")),
            Self::UnwrapNarrow => (vec![result("u16")], scalar("u16")),
            Self::CheckedSub => (vec![scalar("usize"), scalar("usize")], option()),
            Self::UnwrapOption => (vec![option()], scalar("usize")),
        }
    }
}

pub(crate) fn kind(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Kind> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return None;
    }
    let dispatch = node.dispatch.as_ref()?;
    match dispatch.op.as_str() {
        COUNT => Some(Kind::Count),
        CONVERT | "core::convert::TryFrom::try_from"
            if dispatch.operand_tys == ["u32"] && conversion_result(node, "usize") =>
        {
            Some(Kind::Convert)
        }
        UNWRAP | "core::result::Result::<T, E>::unwrap" => {
            let operand = node.children.first()?;
            match crate::data_aggregates::parsed_type(operand).ok()? {
                crate::data_types::TypeToken::IntConversionResult(value) if value == "usize" => {
                    Some(Kind::Unwrap)
                }
                crate::data_types::TypeToken::IntConversionResult(value)
                    if value == "u16"
                        && crate::extraction_profile::facts_support(facts, EXTENSION) =>
                {
                    Some(Kind::UnwrapNarrow)
                }
                _ => None,
            }
        }
        _ => extended_kind(node, facts, &dispatch.op),
    }
}

fn extended_kind(node: &HirNode, facts: &dyn EvidenceFacts, path: &str) -> Option<Kind> {
    if !crate::extraction_profile::facts_support(facts, EXTENSION) {
        return None;
    }
    let dispatch = node.dispatch.as_ref()?;
    match path {
        CONVERT | "core::convert::TryFrom::try_from"
            if dispatch.operand_tys == ["usize"] && conversion_result(node, "u16") =>
        {
            Some(Kind::Narrow)
        }
        "core::num::<impl usize>::checked_sub" => Some(Kind::CheckedSub),
        "std::option::Option::<T>::unwrap" | "core::option::Option::<T>::unwrap" => {
            match crate::data_aggregates::parsed_type(node.children.first()?).ok()? {
                crate::data_types::TypeToken::Option(value) if matches!(value.as_ref(), crate::data_types::TypeToken::Scalar(name) if name == "usize") => {
                    Some(Kind::UnwrapOption)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn conversion_result(node: &HirNode, expected: &str) -> bool {
    matches!(crate::data_aggregates::parsed_type(node),
        Ok(crate::data_types::TypeToken::IntConversionResult(value)) if value == expected)
}

pub(crate) fn resolve(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Summary>, String> {
    let Some(kind) = kind(node, facts) else {
        return Ok(None);
    };
    kind.verify_types(node, facts, kind.implementation().is_some())?;
    authority::verify(node, kind, facts)?;
    source::verify(node, kind, facts)?;
    if let Some(producer) = kind.producer() {
        let operand = node
            .children
            .first()
            .ok_or("Missing checked unwrap operand")?;
        if self::kind(operand, facts) != Some(producer) {
            return Err("Integer unwrap requires its immediate checked producer".into());
        }
        resolve(operand, facts)?.ok_or("Missing checked integer producer")?;
    }

    let (inputs, output) = kind.signature()?;
    Ok(Some(Summary {
        id: format!("rstyle.{}", kind.construct()),
        version: "1.0.0".into(),
        callee_path: node.dispatch.as_ref().unwrap().op.clone(),
        callee_crate: "core".into(),
        argument_types: inputs,
        result_type: output,
        memory_effects: false,
        mutates_arguments: false,
        may_panic: matches!(kind, Kind::UnwrapNarrow | Kind::UnwrapOption),
        always_returns: !matches!(kind, Kind::UnwrapNarrow | Kind::UnwrapOption),
        overflow_behavior: kind.overflow().into(),
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn primitive_tryfrom_candidates_preserve_the_concrete_result_boundary() {
        let mut facts: crate::HirFnFacts = serde_json::from_str(include_str!(
            "nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
        ))
        .unwrap();
        facts
            .build
            .frontend_options
            .insert("extraction_profile".into(), "hir-data-v3".into());
        for (input, output, expected) in [
            ("u32", Some("usize"), Some("data.integer.try-u32-usize")),
            ("usize", Some("u16"), Some("data.integer.try-usize-u16")),
            ("u32", Some("u16"), None),
            ("usize", Some("usize"), None),
            ("usize", Some("u64"), None),
            ("usize", None, None),
        ] {
            let mut node = conversion_node(input, output);
            assert_eq!(
                super::kind(&node, &facts).map(super::Kind::construct),
                expected
            );
            node.ty = Some(crate::ResolvedTy {
                normalized: Some("u16".into()),
                observed: "u16".into(),
                spelling: String::new(),
            });
            assert!(super::kind(&node, &facts).is_none());
        }
    }

    fn conversion_node(input: &str, output: Option<&str>) -> crate::HirNode {
        let token = output.map(|output| {
            serde_json::json!({
                "kind":"int_conversion_result", "value":output
            })
            .to_string()
        });
        serde_json::from_value(serde_json::json!({
            "hir":{"krate":0,"owner":1,"local":2},"span":{"start":0,"end":1},
            "kind":"expr_call","snippet":"", "ty":{"normalized":token,"observed":"", "spelling":""},
            "res":null,"dispatch":{"kind":"resolved_direct","op":"core::convert::TryFrom::try_from",
                "operand_tys":[input],"result_ty":token.unwrap_or_default()},
            "expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,
            "is_unsafe_block":false,"children":[]
        }))
        .unwrap()
    }

    #[test]
    fn integer_contract_pins_reviewed_population_conversion_and_error_sources() {
        let output = std::process::Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let sysroot = String::from_utf8(output.stdout).unwrap();
        let source =
            std::path::Path::new(sysroot.trim()).join("lib/rustlib/src/rust/library/core/src");
        for (file, expected) in [
            (
                "num/uint_macros.rs",
                "sha256:5525546f81f07f28c0d3bc475edf2f86770fe659b35dd4bae55c0a3b890ab12b",
            ),
            (
                "convert/num.rs",
                "sha256:8dbf6c64681e2dee556e0844eef3e53d62a40df0280d25f469352870a401e034",
            ),
            (
                "num/error.rs",
                "sha256:7447f99e89ce2f0163c83cbe177008c3b789b8129602099dfe9dc3f02909baff",
            ),
        ] {
            assert_eq!(
                crate::concrete_types::digest(&std::fs::read(source.join(file)).unwrap()),
                expected,
                "{file}"
            );
        }
    }
}
