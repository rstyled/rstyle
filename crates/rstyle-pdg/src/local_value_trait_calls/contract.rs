//! Exact independently gated primitive routes share source/selection checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Contract {
    UsizeU64,
    U32Usize,
    U16Usize,
    U64Usize,
}

impl Contract {
    pub(crate) fn from_types(input: &str, output: &str) -> Option<Self> {
        match (input, output) {
            ("usize", "u64") => Some(Self::UsizeU64),
            ("u32", "usize") => Some(Self::U32Usize),
            ("u16", "usize") => Some(Self::U16Usize),
            ("u64", "usize") => Some(Self::U64Usize),
            _ => None,
        }
    }

    pub(crate) fn input(self) -> &'static str {
        match self {
            Self::UsizeU64 => "usize",
            Self::U32Usize => "u32",
            Self::U16Usize => "u16",
            Self::U64Usize => "u64",
        }
    }

    pub(crate) fn output(self) -> &'static str {
        match self {
            Self::UsizeU64 => "u64",
            Self::U32Usize | Self::U16Usize | Self::U64Usize => "usize",
        }
    }

    pub(crate) fn capability(self) -> &'static str {
        match self {
            Self::UsizeU64 => super::CAPABILITY,
            Self::U32Usize => super::U32_USIZE_CAPABILITY,
            Self::U16Usize | Self::U64Usize => crate::tryfrom_calls::input::CAPABILITY,
        }
    }

    pub(super) fn permits_output(self, output: &str) -> bool {
        matches!(output, "u8" | "u16" | "u32" | "u64")
            || self != Self::UsizeU64 && output == "usize"
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
