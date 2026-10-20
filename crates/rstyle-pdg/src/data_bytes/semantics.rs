//! Typed normal-completion rules; no invocation-specific provenance copy.
use super::Kind;
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "rule", rename_all = "kebab-case")]
enum Semantics {
    CopyExactLengthOrErr {
        length: u64,
        receiver_operand: u64,
        reads_on: &'static str,
        error: &'static str,
    },
    OkPayloadOrPanic {
        result_operand: u64,
        normal_completion_requires: &'static str,
        exceptional_flow: &'static str,
    },
    UnsignedByteSignificance {
        array_operand: u64,
        length: u64,
        endian: &'static str,
        byte_significance_bits: [u8; 4],
    },
}

pub(super) fn value(kind: Kind) -> serde_json::Value {
    let semantics = match kind {
        Kind::Copy => Semantics::CopyExactLengthOrErr {
            length: 4,
            receiver_operand: 0,
            reads_on: "ok",
            error: "core::array::TryFromSliceError",
        },
        Kind::Unwrap => Semantics::OkPayloadOrPanic {
            result_operand: 0,
            normal_completion_requires: "ok",
            exceptional_flow: "omitted",
        },
        Kind::Native => Semantics::UnsignedByteSignificance {
            array_operand: 0,
            length: 4,
            endian: "little",
            byte_significance_bits: [0, 8, 16, 24],
        },
    };

    serde_json::to_value(semantics).expect("registered byte-call semantics")
}
