//! Each direction has its own policy and exact reviewed implementation.
use super::*;

impl Kind {
    pub(crate) fn is_conversion(self) -> bool {
        matches!(
            self,
            Self::Convert | Self::ConvertU32Usize | Self::ConvertU64Usize
        )
    }

    pub(super) fn conversion(self) -> Self {
        match self {
            Self::Convert | Self::Expect => Self::Convert,
            Self::ConvertU32Usize | Self::ExpectUsize => Self::ConvertU32Usize,
            Self::ConvertU64Usize | Self::ExpectU64Usize => Self::ConvertU64Usize,
        }
    }

    pub(super) fn input(self) -> &'static str {
        match self.conversion() {
            Self::Convert => "usize",
            Self::ConvertU64Usize => "u64",
            _ => "u32",
        }
    }

    pub(super) fn output(self) -> &'static str {
        match self.conversion() {
            Self::Convert => "u64",
            _ => "usize",
        }
    }

    pub(crate) fn capability(self) -> &'static str {
        match self.conversion() {
            Self::Convert => CAPABILITY,
            Self::ConvertU64Usize => crate::tryfrom_calls::input::CAPABILITY,
            _ => U32_USIZE_CAPABILITY,
        }
    }

    pub(super) fn implementation(self) -> &'static str {
        match self.conversion() {
            Self::Convert => IMPLEMENTATION,
            Self::ConvertU64Usize => {
                "std::convert::num::ptr_try_from_impls::<impl std::convert::TryFrom<u64> for usize>::try_from"
            }
            _ => U32_USIZE_IMPLEMENTATION,
        }
    }

    pub(super) fn overflow_behavior(self) -> &'static str {
        match self.conversion() {
            Self::Convert => "reviewed-64-bit-usize-to-u64-always-ok",
            Self::ConvertU64Usize => "reviewed-64-bit-u64-to-usize-always-ok",
            _ => "reviewed-64-bit-u32-to-usize-always-ok",
        }
    }

    pub(super) fn matches(self, node: &HirNode) -> bool {
        let receiver = if self.is_conversion() {
            if node
                .dispatch
                .as_ref()
                .is_none_or(|dispatch| dispatch.operand_tys != [self.input()])
            {
                return false;
            }
            node
        } else {
            let Some(receiver) = node.children.first() else {
                return false;
            };
            if receiver
                .dispatch
                .as_ref()
                .is_none_or(|dispatch| dispatch.operand_tys != [self.input()])
            {
                return false;
            }
            receiver
        };
        matches!(crate::data_aggregates::parsed_type(receiver), Ok(crate::data_types::TypeToken::IntConversionResult(value)) if value == self.output())
    }
}
