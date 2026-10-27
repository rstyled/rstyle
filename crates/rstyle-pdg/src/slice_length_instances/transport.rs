//! Strict bounded wire inputs. Display strings never establish type identity.
use crate::{ByteSpan, HirRef, data_receivers, generated_sources::capture::Definition};
use serde::Deserialize;
use serde_json::Value;

pub(super) const KEY: &str = "rstyle.rust.shared-slice-instance-projections";
pub(super) const SCOPE: &str = "selected local u8 shared-slice original-owner type overlay; no obligation, source verification or admission authority";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Envelope {
    pub identity: String,
    pub scope: String,
    pub obligations_verified: bool,
    pub call_receipts: Vec<Receipt>,
    pub outcome: Outcome,
}

#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Outcome {
    Observed { projection: Box<Projection> },
    Unsupported { reason: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub caller: String,
    pub caller_source_digest: String,
    pub caller_fn_span: ByteSpan,
    pub call: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Projection {
    pub definition: Definition,
    pub type_parameter: Definition,
    pub owner: HirRef,
    pub source_digest: String,
    pub fn_span: ByteSpan,
    pub original_source_scan_complete: bool,
    pub arguments: Vec<Argument>,
    pub parameters: Vec<Typed>,
    pub result: Endpoint,
    pub bindings: Vec<Typed>,
    pub nodes: Vec<Typed>,
    pub methods: Vec<Method>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Typed {
    pub hir: HirRef,
    pub concrete: Endpoint,
    pub adjustments: Vec<Endpoint>,
    pub dispatch_operands: Vec<Endpoint>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Endpoint {
    Structural {
        arguments: Vec<Argument>,
    },
    FunctionItem {
        definition: Definition,
        arguments: Vec<Argument>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Argument {
    pub kind: String,
    #[serde(rename = "value")]
    pub display: String,
    pub display_only: bool,
    pub identity_status: String,
    pub identity: Type,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Type {
    Builtin {
        name: String,
    },
    Str,
    Reference {
        region: String,
        mutable: bool,
        referent: Box<Type>,
    },
    Slice {
        element: Box<Type>,
    },
    Tuple {
        elements: Vec<Type>,
    },
    Adt {
        definition: Definition,
        arguments: Vec<Type>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Method {
    pub original: data_receivers::Method,
    pub concrete: ConcreteMethod,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConcreteMethod {
    pub call: HirRef,
    pub source_span: ByteSpan,
    pub receiver: HirRef,
    pub receiver_type: Endpoint,
    pub adjusted_type: Endpoint,
    pub adjustments: Vec<Adjustment>,
    pub signature_inputs: Vec<Endpoint>,
    pub signature_result: Endpoint,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Adjustment {
    pub kind: data_receivers::Kind,
    pub target: Endpoint,
}
