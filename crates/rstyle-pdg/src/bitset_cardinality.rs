//! Bounded count, lossless integer TryFrom, and immediate successful unwrap.
mod authority;
mod source;
mod types;
use crate::{HirNode, call_summaries::Summary, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.bitset-cardinality";
pub(crate) const COUNT: &str = "core::num::<impl u32>::count_ones";
pub(crate) const CONVERT: &str = "std::convert::TryFrom::try_from";
pub(crate) const UNWRAP: &str = "std::result::Result::<T, E>::unwrap";
pub(crate) const IMPLEMENTATION: &str =
    "std::convert::num::ptr_try_from_impls::<impl std::convert::TryFrom<u32> for usize>::try_from";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Count,
    Convert,
    Unwrap,
}
impl Kind {
    pub fn construct(self) -> &'static str {
        match self {
            Self::Count => "data.bitset.count-u32",
            Self::Convert => "data.integer.try-u32-usize",
            Self::Unwrap => "data.integer.unwrap-ok-usize",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Count => "rstyle.data.bitset.count-u32",
            Self::Convert => "rstyle.data.integer.try-u32-usize",
            Self::Unwrap => "rstyle.data.integer.unwrap-ok-usize",
        }
    }
    fn path(self) -> &'static str {
        match self {
            Self::Count => COUNT,
            Self::Convert => CONVERT,
            Self::Unwrap => UNWRAP,
        }
    }
    pub fn signature(self) -> Result<(String, String), String> {
        let result = crate::data_types::TypeToken::IntConversionResult("usize".into()).text()?;
        Ok(match self {
            Self::Count => ("u32".into(), "u32".into()),
            Self::Convert => ("u32".into(), result),
            Self::Unwrap => (result, "usize".into()),
        })
    }
}

pub(crate) fn kind(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Kind> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return None;
    }
    let dispatch = node.dispatch.as_ref()?;
    match dispatch.op.as_str() {
        COUNT => Some(Kind::Count),
        CONVERT | "core::convert::TryFrom::try_from" if dispatch.operand_tys == ["u32"] => {
            Some(Kind::Convert)
        }
        UNWRAP | "core::result::Result::<T, E>::unwrap"
            if node.children.first().is_some_and(|operand| {
                matches!(
                    crate::data_aggregates::parsed_type(operand),
                    Ok(crate::data_types::TypeToken::IntConversionResult(_))
                )
            }) =>
        {
            Some(Kind::Unwrap)
        }
        _ => None,
    }
}

pub(crate) fn resolve(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Summary>, String> {
    let Some(kind) = kind(node, facts) else {
        return Ok(None);
    };
    let [operand] = node.children.as_slice() else {
        return Err("Integer operation requires one evaluated operand".into());
    };

    types::verify(node, operand, kind, facts)?;
    authority::verify(node, kind, facts)?;
    source::verify(node, operand, kind, facts)?;
    if kind == Kind::Unwrap {
        if self::kind(operand, facts) != Some(Kind::Convert) {
            return Err(
                "Integer unwrap requires an immediate checked successful conversion".into(),
            );
        }
        resolve(operand, facts)?.ok_or("Missing successful integer conversion")?;
    }

    let (input, output) = kind.signature()?;
    Ok(Some(Summary {
        id: kind.id().into(),
        version: "1.0.0".into(),
        callee_path: node.dispatch.as_ref().unwrap().op.clone(),
        callee_crate: "core".into(),
        argument_types: vec![input],
        result_type: output,
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: "exact-u32-population-or-lossless-64-bit-conversion".into(),
    }))
}

#[cfg(test)]
mod tests {
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
