//! Owning checked tokens activate only the proved original byte/view constructor.
//! Raw generic facts, diagnostic observations and matching names grant nothing.
use super::{CheckedProjection, Haystack, shared_bytes};
use crate::unit_store::EvidenceFacts;
use crate::{HirFnFacts, HirNode, data_origins::Origin, field_locations::Projection};

#[cfg(test)]
mod tests;

pub const CAPABILITY: &str = "rust.checked-reference-constructor-instances";
pub const CONSTRUCT: &str = "data.reference-instance.as-ref";
pub const STRING_CAPABILITY: &str = "rust.checked-string-byte-view-instances";
pub const STRING_SUMMARY: &str = "rstyle.reference-instance.str-as-bytes";

/// Absence stays ordinary source evidence. String views require their separate
/// bounded capability and the same complete private constructor proof.
pub fn verify(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    Ok(checked(facts)?.is_some())
}

fn checked(facts: &dyn EvidenceFacts) -> Result<Option<&CheckedProjection>, String> {
    let Some(checked) = facts.reference_instance() else {
        return Ok(None);
    };
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Checked reference instance requires its registered capability".into());
    }
    if checked.haystack() == Haystack::Str
        && !crate::extraction_profile::facts_support(facts, STRING_CAPABILITY)
    {
        return Err("String byte-view instances require their registered capability".into());
    }

    owning(checked, facts.facts())?;
    Ok(Some(checked))
}

fn owning(checked: &CheckedProjection, facts: &HirFnFacts) -> Result<(), String> {
    if checked.facts() != facts {
        return Err("Checked reference instance differs from its owning facts".into());
    }
    findings(facts, checked.as_ref())
}

/// Coverage of the exact preserved finding, never deletion of raw diagnostics.
pub fn source_findings(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    verify(facts)
}

fn findings(facts: &HirFnFacts, call: &HirNode) -> Result<(), String> {
    let [finding] = facts.source_diagnostics.as_slice() else {
        return Err("Checked constructor requires its exact deferred AsRef finding roster".into());
    };
    if facts.source_scan_complete
        || finding.capability != "rust.call-summaries"
        || finding.construct != "expr_method_call"
        || finding.span != call.span
        || finding.snippet != call.snippet
        || finding.severity != "error"
        || finding.reason
            != "Original owned source contains deferred rust.call-summaries; cfg removal cannot certify a supported remainder."
    {
        return Err(
            "Checked constructor deferred AsRef finding differs from its proved source site".into(),
        );
    }
    Ok(())
}

/// One checked conversion; evaluates operand zero exactly once and preserves its
/// shared root. A string view adds an explicit UTF-8 byte projection.
pub struct Operation<'a> {
    pub receiver: &'a HirNode,
    pub input_origin: Origin,
    pub output_origin: Origin,
    haystack: Haystack,
}

impl Operation<'_> {
    pub(crate) fn project(&self, roots: Vec<Origin>) -> Result<Vec<Origin>, String> {
        if roots != [self.input_origin.clone()] {
            return Err("Checked AsRef operand loses its exact shared formal origin".into());
        }
        Ok(vec![self.output_origin.clone()])
    }

    pub(crate) fn summary_id(&self) -> &'static str {
        match self.haystack {
            Haystack::Bytes => "rstyle.reference-instance.byte-as-ref",
            Haystack::Str => STRING_SUMMARY,
        }
    }

    pub(crate) fn verify_types(&self, input: &str, output: &str) -> Result<(), String> {
        let expected = match self.haystack {
            Haystack::Bytes => shared_bytes(),
            Haystack::Str => {
                crate::data_types::TypeToken::Shared(Box::new(crate::data_types::TypeToken::Str))
            }
        };
        if crate::data_types::parse(input)? != expected
            || crate::data_types::parse(output)? != shared_bytes()
        {
            return Err(
                "Checked AsRef changes its independently proved input/output view types".into(),
            );
        }
        Ok(())
    }
}

pub fn operation<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Operation<'a>>, String> {
    let Some(checked) = checked(facts)? else {
        return Ok(None);
    };
    checked_operation(node, checked.as_ref(), checked.haystack())
}

fn checked_operation<'a>(
    node: &'a HirNode,
    expected: &HirNode,
    haystack: Haystack,
) -> Result<Option<Operation<'a>>, String> {
    if node.hir != expected.hir {
        return Ok(None);
    }
    if node != expected {
        return Err("Checked AsRef operation differs from its exact original HIR/source".into());
    }
    let [receiver] = node.children.as_slice() else {
        return Err("Checked AsRef operation requires exactly operand zero".into());
    };

    let input_origin = Origin {
        parameter: 0,
        path: vec![Projection::SharedReferent],
    };
    let mut output_origin = input_origin.clone();
    if haystack == Haystack::Str {
        output_origin.path.push(Projection::Utf8Bytes);
    }
    Ok(Some(Operation {
        receiver,
        input_origin,
        output_origin,
        haystack,
    }))
}

pub(crate) fn verify_returned_origin(
    origin: &Origin,
    facts: &dyn EvidenceFacts,
    result: &crate::data_types::TypeToken,
) -> Result<(), String> {
    if !verify(facts)?
        || !facts
            .reference_instance()
            .is_some_and(|checked| checked.haystack() == Haystack::Str)
        || origin.parameter != 0
        || origin.path != [Projection::SharedReferent, Projection::Utf8Bytes]
        || *result != shared_bytes()
    {
        return Err(
            "String byte-view result lacks its exact checked formal/projection/type proof".into(),
        );
    }
    if crate::helper_reads::type_at(facts, origin.parameter, &origin.path)?
        != crate::data_types::TypeToken::Slice(Box::new(crate::data_types::TypeToken::Scalar(
            "u8".into(),
        )))
    {
        return Err("String byte-view projection does not resolve to u8 storage".into());
    }
    Ok(())
}
