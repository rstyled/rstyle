use super::*;
use crate::reference_instances::view_types::{BorrowedViewType, ViewType};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Definition {
    pub path: String,
    pub definition_identity: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub stable_crate_id: String,
    pub local: bool,
    pub crate_hash: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Argument {
    kind: String,
    #[serde(rename = "value")]
    _display: String,
    display_only: bool,
    identity_status: String,
    identity: Option<Type>,
    reason: Option<String>,
}

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Type {
    Builtin {
        name: String,
    },
    Str,
    Slice {
        element: Box<Type>,
    },
    Reference {
        region: String,
        mutable: bool,
        referent: Box<Type>,
    },
}

impl ViewType for Type {
    const HAYSTACK_ERROR: &'static str = "AsRef instances are restricted to exact slice-u8 and str";

    fn u8() -> Self {
        Self::Builtin { name: "u8".into() }
    }

    fn text() -> Self {
        Self::Str
    }

    fn slice(element: Self) -> Self {
        Self::Slice {
            element: Box::new(element),
        }
    }
}

impl BorrowedViewType for Type {
    fn reference(self, region: String, mutable: bool) -> Self {
        Self::Reference {
            region,
            mutable,
            referent: Box::new(self),
        }
    }
}

impl Argument {
    pub fn verify(&self, expected: &Type) -> Result<(), String> {
        if self.kind != "type"
            || !self.display_only
            || self.identity_status != "known"
            || self.identity.as_ref() != Some(expected)
            || self.reason.is_some()
        {
            return Err("AsRef structural argument identity disagrees".into());
        }
        Ok(())
    }

    pub fn original_generic(&self) -> Result<(), String> {
        if self.kind != "type"
            || !self.display_only
            || self.identity_status != "unsupported"
            || self.identity.is_some()
            || self.reason.as_deref().is_none_or(str::is_empty)
        {
            return Err("AsRef original unresolved argument observation disagrees".into());
        }
        Ok(())
    }
}

pub(super) fn arguments(observed: &[Argument], expected: &[Type]) -> Result<(), String> {
    crate::source_evidence::exact_pairs(
        observed.iter(),
        expected.iter(),
        "AsRef structural argument arity disagrees",
        Argument::verify,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Signature {
    inputs: Vec<Argument>,
    output: Vec<Argument>,
    bound_variables: usize,
    receiver_region: Region,
    result_region: Region,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Region {
    binder_depth: u32,
    variable: u32,
}

impl Signature {
    pub fn verify(&self, receiver: &Type, result: &Type) -> Result<(), String> {
        if self.bound_variables != 1
            || self.receiver_region
                != (Region {
                    binder_depth: 0,
                    variable: 0,
                })
            || self.receiver_region != self.result_region
        {
            return Err(
                "AsRef signature lacks its exact original late-bound borrow relation".into(),
            );
        }
        arguments(&self.inputs, std::slice::from_ref(receiver))?;
        arguments(&self.output, std::slice::from_ref(result))
    }
}
