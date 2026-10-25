//! Exact core NonZero<u8/u16> values. This is no permission for arbitrary unsafe bodies.
//!
//! The pinned core declaration stores `T::NonZeroInner`; the sealed primitive
//! contract requires that inner type to be Copy, and NonZero explicitly implements
//! Copy. Thus these two concrete types have no destructor or owned cleanup. The
//! safe `new` method's reviewed niche transmute maps zero to None and preserves
//! every nonzero scalar. Layout, transmutation and memory operations are not exposed.
mod authority;
#[cfg(test)]
mod tests;

use crate::structural_json::object;
use crate::unit_store::EvidenceFacts;
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scalar {
    #[serde(rename = "u8")]
    U8,
    #[serde(rename = "u16")]
    U16,
}

impl Scalar {
    pub fn spelling(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
        }
    }

    pub fn maximum(self) -> u64 {
        match self {
            Self::U8 => u64::from(u8::MAX),
            Self::U16 => u64::from(u16::MAX),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub path: String,
    pub definition_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub stable_crate_id: String,
    pub local: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub definition: Definition,
    pub scalar: Scalar,
}

impl Observation {
    /// Shape check only; a caller must also verify compiler/build authority.
    pub fn check_shape(&self) -> Result<(), String> {
        authority::definition(&self.definition, authority::NONZERO)
    }

    /// Normalize a reviewed visible alias only after validating its full identity.
    pub fn canonicalized(self) -> Result<Self, String> {
        self.check_shape()?;
        Ok(authority::observation(self.scalar))
    }
}

/// A validated mathematical payload; construction still requires type authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Value {
    scalar: Scalar,
    payload: u64,
}

/// Verified type authority, reusable for constant-time scalar construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckedType {
    scalar: Scalar,
}

impl CheckedType {
    pub fn construct(self, input: u64) -> Result<Option<Value>, String> {
        if input > self.scalar.maximum() {
            return Err("NonZero input is outside its concrete scalar domain".into());
        }
        Ok((input != 0).then_some(Value {
            scalar: self.scalar,
            payload: input,
        }))
    }
}

impl Value {
    pub fn scalar(self) -> Scalar {
        self.scalar
    }

    pub fn get(self) -> u64 {
        self.payload
    }
}

/// Type/cleanup authority only; does not grant source, call, alias or flow support.
pub fn verify(observation: &Observation, facts: &dyn EvidenceFacts) -> Result<CheckedType, String> {
    authority::verify(facts)?;
    observation.check_shape()?;
    Ok(CheckedType {
        scalar: observation.scalar,
    })
}

pub fn observe_type(identity: &Json, facts: &dyn EvidenceFacts) -> Result<Observation, String> {
    recognize(identity, facts, Contract::Type)
}

pub fn verify_new(callable: &Json, facts: &dyn EvidenceFacts) -> Result<Observation, String> {
    recognize(callable, facts, Contract::Constructor)
}

enum Contract {
    Type,
    Constructor,
}

impl Contract {
    fn definitions(&self) -> &[(&str, (&str, &str))] {
        match self {
            Self::Type => &[("/definition", authority::NONZERO)],
            Self::Constructor => &[
                ("/definition", authority::NEW),
                ("/signature/output/definition", authority::OPTION),
                (
                    "/signature/output/arguments/0/definition",
                    authority::NONZERO,
                ),
            ],
        }
    }

    fn expected(&self, observation: &Observation) -> Json {
        match self {
            Self::Type => structural(observation),
            Self::Constructor => constructor(observation),
        }
    }
}

fn recognize(
    identity: &Json,
    facts: &dyn EvidenceFacts,
    contract: Contract,
) -> Result<Observation, String> {
    authority::verify(facts)?;
    let mut canonical = identity.clone();
    for (pointer, expected) in contract.definitions() {
        canonical_definition(&mut canonical, pointer, *expected)?;
    }

    for scalar in [Scalar::U8, Scalar::U16] {
        let observation = authority::observation(scalar);
        if canonical == contract.expected(&observation) {
            return Ok(observation);
        }
    }
    Err(
        "NonZero requires exact core identity, concrete substitution and complete safe signature"
            .into(),
    )
}

fn structural(observation: &Observation) -> Json {
    json!({"kind":"adt", "definition":observation.definition,
        "arguments":[builtin(observation.scalar)]})
}

fn builtin(scalar: Scalar) -> Json {
    json!({"kind":"builtin", "name":scalar.spelling()})
}

fn constructor(observation: &Observation) -> Json {
    object([
        ("kind", json!("function_item")),
        ("definition", json!(authority::identity(authority::NEW))),
        ("arguments", json!([builtin(observation.scalar)])),
        ("signature", signature(observation)),
    ])
}

fn signature(observation: &Observation) -> Json {
    object([
        ("abi", json!("\"Rust\"")),
        ("bound_variables", json!(0)),
        ("safety", json!("Safe")),
        ("variadic", json!(false)),
        ("inputs", json!([builtin(observation.scalar)])),
        ("output", option(observation)),
    ])
}

fn option(observation: &Observation) -> Json {
    json!({"kind":"adt", "definition":authority::identity(authority::OPTION),
        "arguments":[structural(observation)]})
}

/// Normalize only a reviewed visible alias after checking every nominal identity field.
fn canonical_definition(
    value: &mut Json,
    pointer: &str,
    expected: (&str, &str),
) -> Result<(), String> {
    let slot = value
        .pointer_mut(pointer)
        .ok_or("Missing NonZero nominal definition")?;
    let observed: Definition = serde_json::from_value(slot.clone()).map_err(|e| e.to_string())?;
    authority::definition(&observed, expected)?;

    *slot = serde_json::to_value(authority::identity(expected)).map_err(|e| e.to_string())?;
    Ok(())
}
