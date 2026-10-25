//! The two independently gated primitive routes share source/selection checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Contract {
    UsizeU64,
    U32Usize,
}

impl Contract {
    pub(crate) fn from_types(input: &str, output: &str) -> Option<Self> {
        match (input, output) {
            ("usize", "u64") => Some(Self::UsizeU64),
            ("u32", "usize") => Some(Self::U32Usize),
            _ => None,
        }
    }

    pub(crate) fn input(self) -> &'static str {
        match self {
            Self::UsizeU64 => "usize",
            Self::U32Usize => "u32",
        }
    }

    pub(crate) fn output(self) -> &'static str {
        match self {
            Self::UsizeU64 => "u64",
            Self::U32Usize => "usize",
        }
    }

    pub(crate) fn capability(self) -> &'static str {
        match self {
            Self::UsizeU64 => super::CAPABILITY,
            Self::U32Usize => super::U32_USIZE_CAPABILITY,
        }
    }

    pub(super) fn permits_output(self, output: &str) -> bool {
        matches!(output, "u8" | "u16" | "u32" | "u64")
            || self == Self::U32Usize && output == "usize"
    }
}

pub(crate) fn captured(facts: &crate::HirFnFacts) -> Option<Contract> {
    let [parameter] = facts.signature.params.as_slice() else {
        return None;
    };
    Contract::from_types(
        parameter.ty.normalized.as_deref()?,
        facts.signature.return_ty.normalized.as_deref()?,
    )
}

pub(super) fn dispatch(node: &crate::HirNode) -> Option<Contract> {
    let dispatch = node.dispatch.as_ref()?;
    let [input] = dispatch.operand_tys.as_slice() else {
        return None;
    };
    Contract::from_types(input, &dispatch.result_ty)
}
