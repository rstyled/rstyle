//! Exact checked usize arithmetic for complete length-computation families.
mod authority;
mod source;

use crate::concrete_calls::ConcreteCall;
use crate::{
    HirNode, call_summaries::Summary, concrete_types::TypeTree, unit_store::EvidenceFacts,
};

pub(crate) const KEY: &str = "rstyle.data:checked-length";

pub(crate) const CAPABILITY: &str = "rust.checked-usize-lengths";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Add,
    Mul,
}

impl Kind {
    pub fn construct(self) -> &'static str {
        match self {
            Self::Add => "data.length.checked-add-usize",
            Self::Mul => "data.length.checked-mul-usize",
        }
    }

    pub fn path(self) -> &'static str {
        match self {
            Self::Add => "core::num::<impl usize>::checked_add",
            Self::Mul => "core::num::<impl usize>::checked_mul",
        }
    }

    fn method(self) -> &'static str {
        match self {
            Self::Add => "checked_add",
            Self::Mul => "checked_mul",
        }
    }
}

impl ConcreteCall for Kind {
    fn trees(self) -> (Vec<TypeTree>, TypeTree) {
        let scalar = TypeTree::Scalar("usize".into());
        (
            vec![scalar.clone(), scalar.clone()],
            TypeTree::Option(Box::new(scalar)),
        )
    }
}

pub(crate) fn kind(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<Kind> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return None;
    }
    match node.dispatch.as_ref()?.op.as_str() {
        "core::num::<impl usize>::checked_add" => Some(Kind::Add),
        "core::num::<impl usize>::checked_mul" => Some(Kind::Mul),
        _ => None,
    }
}

pub(crate) fn verify(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if self::kind(node, facts) != Some(kind) {
        return Err("Checked length operation requires its registered exact capability".into());
    }
    kind.verify_types(node, facts, false)?;
    authority::verify(node, kind, facts)?;
    source::verify(node, kind, facts)
}

pub(crate) fn resolve(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Summary>, String> {
    let Some(kind) = kind(node, facts) else {
        return Ok(None);
    };
    verify(node, kind, facts)?;
    let (inputs, output) = kind.signature()?;
    Ok(Some(Summary {
        id: format!("rstyle.{}", kind.construct()),
        version: "1.0.0".into(),
        callee_path: kind.path().into(),
        callee_crate: "core".into(),
        argument_types: inputs,
        result_type: output,
        memory_effects: false,
        mutates_arguments: false,
        may_panic: false,
        always_returns: true,
        overflow_behavior: match kind {
            Kind::Add => "checked-usize-add-none-on-overflow",
            Kind::Mul => "checked-usize-mul-none-on-overflow",
        }
        .into(),
    }))
}

pub(crate) fn semantics(kind: Kind) -> serde_json::Value {
    let arithmetic = match kind {
        Kind::Add => "addition",
        Kind::Mul => "multiplication",
    };

    serde_json::Value::Object(
        [
            ("rule", kind.construct().into()),
            ("receiver_operand", 0.into()),
            ("argument_operand", 1.into()),
            (
                "evaluation_order",
                serde_json::Value::Array(vec![0.into(), 1.into()]),
            ),
            ("evaluation_count", "once-each".into()),
            ("arithmetic", arithmetic.into()),
            ("domain", "unsigned-64-bit".into()),
            ("success", "some-exact-mathematical-value".into()),
            ("overflow", "none".into()),
            ("content_read", false.into()),
            ("may_panic", false.into()),
        ]
        .into_iter()
        .map(|(key, value)| (key.into(), value))
        .collect(),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn checked_arithmetic_pins_reviewed_unsigned_and_option_sources() {
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
                "option.rs",
                "sha256:693df7245cf3e669b2c442b62c96c76c43ad1f9cfe79f6bf4306e71c32fa9f5d",
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
