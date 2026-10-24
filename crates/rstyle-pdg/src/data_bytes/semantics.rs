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
    BorrowExactLengthOrErr {
        length: u64,
        receiver_operand: u64,
        payload_origins_on: &'static str,
        content_read: bool,
        error: &'static str,
    },
    OkPayloadOrPanic {
        result_operand: u64,
        normal_completion_requires: &'static str,
        exceptional_flow: &'static str,
    },
    LittleEndianU32 {
        array_operand: u64,
        byte_significance_bits: [u8; 4],
    },
    UnsignedByteSignificance {
        array_operand: u64,
        length: u64,
        endian: &'static str,
        byte_significance_bits: Vec<u8>,
    },
}

pub(super) fn value(kind: Kind) -> serde_json::Value {
    let semantics = match kind {
        Kind::Borrow(_) => Semantics::BorrowExactLengthOrErr {
            length: kind.length(),
            receiver_operand: 0,
            payload_origins_on: "ok",
            content_read: false,
            error: "core::array::TryFromSliceError",
        },
        Kind::Copy(_) => Semantics::CopyExactLengthOrErr {
            length: kind.length(),
            receiver_operand: 0,
            reads_on: "ok",
            error: "core::array::TryFromSliceError",
        },
        Kind::Unwrap(_) | Kind::UnwrapBorrow(_) => Semantics::OkPayloadOrPanic {
            result_operand: 0,
            normal_completion_requires: "ok",
            exceptional_flow: "omitted",
        },
        Kind::LittleU32 => Semantics::LittleEndianU32 {
            array_operand: 0,
            byte_significance_bits: [0, 8, 16, 24],
        },
        Kind::Native | Kind::NativeUsize | Kind::NativeU16 | Kind::NativeU128 => {
            Semantics::UnsignedByteSignificance {
                array_operand: 0,
                length: kind.length(),
                endian: "little",
                byte_significance_bits: (0..kind.length())
                    .map(|position| (position * 8) as u8)
                    .collect(),
            }
        }
    };

    serde_json::to_value(semantics).expect("registered byte-call semantics")
}
