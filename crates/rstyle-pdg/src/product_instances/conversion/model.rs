//! Typed observations; display strings never establish a type or a callee.
use crate::{ByteSpan, HirRef, data_library::Callee};
use serde::Deserialize;

mod range;
pub(super) use range::verify as verify_range;

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Definition {
    pub path: String,
    pub definition_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub stable_crate_id: String,
    pub local: bool,
    pub crate_hash: Option<String>,
}

impl Definition {
    pub fn callee(&self) -> Result<Callee, String> {
        if self.local || self.stable_crate_id.is_empty() {
            return Err("Product conversion requires an actual external core definition".into());
        }
        Ok(Callee {
            path: self.path.clone(),
            crate_name: self.crate_name.clone(),
            crate_hash: self
                .crate_hash
                .clone()
                .ok_or("Missing conversion core crate identity")?,
            definition_hash: self.definition_identity.clone(),
            implementation: None,
        })
    }
}

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Structural {
    Builtin {
        name: String,
    },
    Adt {
        definition: Definition,
        arguments: Vec<Structural>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Argument {
    kind: String,
    #[serde(rename = "value")]
    _display: String,
    display_only: bool,
    identity_status: String,
    pub identity: Structural,
}

impl Argument {
    pub fn verify(&self, expected: &Structural) -> Result<(), String> {
        if self.kind != "type"
            || !self.display_only
            || self.identity_status != "known"
            || &self.identity != expected
        {
            return Err("Product conversion loses its complete concrete Span identity".into());
        }
        Ok(())
    }
}

pub(super) fn arguments(
    values: &[Argument],
    span: &Structural,
    count: usize,
) -> Result<(), String> {
    crate::source_evidence::exact_pairs(
        values.iter(),
        std::iter::repeat_n(span, count),
        "Product conversion argument roster differs from its reviewed substitution",
        Argument::verify,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Signature {
    pub inputs: Vec<Vec<Argument>>,
    pub output: Vec<Argument>,
    pub bound_variables: usize,
}

impl Signature {
    pub fn verify(&self, span: &Structural) -> Result<(), String> {
        self.between(
            span,
            span,
            SignatureContract {
                input_error: "Product conversion must have exactly one value input",
                bound_error: "Product conversion introduces unsupported late-bound variables",
                argument: single_argument,
            },
        )
    }

    fn between(
        &self,
        input_type: &Structural,
        output: &Structural,
        contract: SignatureContract,
    ) -> Result<(), String> {
        let [input] = self.inputs.as_slice() else {
            return Err(contract.input_error.into());
        };
        if self.bound_variables != 0 {
            return Err(contract.bound_error.into());
        }
        (contract.argument)(input, input_type)?;
        (contract.argument)(&self.output, output)
    }
}

struct SignatureContract {
    input_error: &'static str,
    bound_error: &'static str,
    argument: fn(&[Argument], &Structural) -> Result<(), String>,
}

fn single_argument(values: &[Argument], expected: &Structural) -> Result<(), String> {
    arguments(values, expected, 1)
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Original {
    Parameter {
        index: u32,
        name: String,
    },
    Adt {
        definition: Definition,
        arguments: Vec<Structural>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    pub requested: Definition,
    pub requested_trait: Definition,
    pub requested_arguments: Vec<Argument>,
    pub selected: Definition,
    pub selected_impl: Definition,
    pub selected_trait: Definition,
    pub selected_arguments: Vec<Argument>,
    pub original_impl_trait_arguments: Vec<Original>,
    pub substituted_impl_trait_arguments: Vec<Argument>,
    pub requested_signature: Signature,
    pub selected_signature: Signature,
}

impl Selection {
    pub fn structure(&self, span: &Structural, parameters: &[u32]) -> Result<(), String> {
        arguments(&self.requested_arguments, span, 2)?;
        arguments(
            &self.selected_arguments,
            span,
            if parameters == [0, 1] { 2 } else { 1 },
        )?;
        arguments(&self.substituted_impl_trait_arguments, span, 2)?;

        let original = &self.original_impl_trait_arguments;
        if original.len() != parameters.len()
            || original.iter().zip(parameters).any(|(observed, expected)| {
                let Original::Parameter { index, name } = observed else {
                    return true;
                };
                index != expected || name != if *expected == 0 { "T" } else { "U" }
            })
        {
            return Err("Product conversion changes its actual impl generic roles".into());
        }
        self.requested_signature.verify(span)?;
        self.selected_signature.verify(span)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observation {
    pub identity: String,
    pub hir: HirRef,
    pub span: ByteSpan,
    pub receiver: HirRef,
    pub receiver_type: Vec<Argument>,
    pub result_type: Vec<Argument>,
    pub adjustments: Vec<serde_json::Value>,
    pub from_selection_basis: String,
    pub into: Selection,
    pub from: Selection,
}
