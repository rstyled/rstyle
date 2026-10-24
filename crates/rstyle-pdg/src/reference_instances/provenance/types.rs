//! Binder identities remain symbolic; erased substitutions never prove lifetimes.
use super::Result;
use super::model::{Argument, Clause, Region, Signature, Structural, Symbolic, SymbolicArgument};
use crate::HirFnFacts;
use crate::reference_instances::view_types::ViewType;

fn region() -> Region {
    Region::Parameter {
        index: 0,
        name: "'h".into(),
    }
}

fn parameter() -> Symbolic {
    Symbolic::Parameter {
        index: 1,
        name: "H".into(),
    }
}

impl ViewType for Structural {
    const HAYSTACK_ERROR: &'static str =
        "Reference provenance supports only concrete str/[u8] arguments";

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

pub(super) fn arguments(values: &[Argument], haystack: &str, lifetime: bool) -> Result<()> {
    let expected = Structural::haystack(haystack)?;
    if values.len() != 1 + usize::from(lifetime) {
        return Err("Reference substitution omits or invents generic arguments".into());
    }
    if lifetime && !argument(&values[0], "lifetime", &Structural::ErasedRegion) {
        return Err("Reference instance lifetime substitution is not explicitly erased".into());
    }

    if !argument(&values[usize::from(lifetime)], "type", &expected) {
        return Err("Reference type substitution lacks its exact structural identity".into());
    }
    Ok(())
}

fn argument(value: &Argument, kind: &str, expected: &Structural) -> bool {
    value.kind == kind
        && value.display_only
        && value.identity_status == "known"
        && value.identity == *expected
}

pub(super) fn clause(record: &Clause, haystack: &str, index: usize) -> Result<()> {
    let mut original = vec![SymbolicArgument::Type(parameter())];
    if index == 1 {
        original.push(SymbolicArgument::Type(Symbolic::Slice {
            element: Box::new(Symbolic::Builtin { name: "u8".into() }),
        }));
    }
    let mut substituted = vec![Structural::haystack(haystack)?];
    if index == 1 {
        substituted.push(Structural::bytes());
    }

    if record.original_arguments != original
        || record.substituted_arguments.len() != substituted.len()
        || !record
            .substituted_arguments
            .iter()
            .zip(substituted)
            .all(|(value, expected)| argument(value, "type", &expected))
    {
        return Err(
            "Reference bound loses original H/[u8] identity or concrete substitution".into(),
        );
    }
    Ok(())
}

pub(super) fn signature(canonical: &HirFnFacts, signature: &Signature) -> Result<()> {
    let input = Symbolic::Reference {
        lifetime: region(),
        mutable: false,
        referent: Box::new(parameter()),
    };
    if signature.inputs.as_slice() != [input] {
        return Err("Reference original signature loses its shared lifetime0/H1 input".into());
    }
    let Symbolic::Adt {
        definition,
        arguments,
    } = &signature.output
    else {
        return Err("Reference result lacks its original nominal binder identity".into());
    };
    let compiled = canonical
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Reference result lacks canonical compiled owner")?;

    if definition.path != "util::search::Input"
        || !definition.local
        || definition.definition_identity.is_empty()
        || compiled
            .get("crate_name")
            .and_then(serde_json::Value::as_str)
            != Some(definition.crate_name.as_str())
        || compiled
            .get("stable_crate_id")
            .and_then(serde_json::Value::as_str)
            != Some(definition.stable_crate_id.as_str())
        || arguments.as_slice() != [SymbolicArgument::Lifetime(region())]
    {
        return Err(
            "Reference result differs from original Input<'h> lifetime relationship".into(),
        );
    }
    Ok(())
}
