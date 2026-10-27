//! Index selectors remain in their callee graph, with CALL input snapshots.
use super::*;
use crate::{HirNode, MemoryLocation, producer::PairView, unit_store::EvidenceFacts};

pub(super) const CAPABILITY: &str = "rust.indexed-read-helper-calls";
pub(super) const ABSTRACTION: &str = "rstyle.formal-helper-index/1";

pub(super) fn contains(path: &[Projection]) -> bool {
    path.iter().any(|step| {
        matches!(
            step,
            Projection::IndexedElement { .. }
                | Projection::HelperIndex { .. }
                | Projection::PrefixElement { .. }
        )
    })
}

pub(super) fn substitute(
    read: &Read,
    actual: &[Projection],
    call: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Projection>, String> {
    if !contains(&read.path) {
        return super::calls::substituted_path(&read.path, actual);
    }
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Indexed helper reads require their registered capability".into());
    }

    if read
        .path
        .iter()
        .any(|part| matches!(part, Projection::PrefixElement { .. }))
        && !crate::scalar_product_instances::forwarders::candidate(facts)
        && !crate::nonzero_digit_calls::candidate(facts)
    {
        return Err("Prefix reads require their exact guarded whole-result caller".into());
    }

    let tail = read
        .path
        .strip_prefix(&[Projection::SharedReferent])
        .ok_or("Indexed helper read lacks a shared formal root")?;
    let scope = Projection::HelperIndex {
        call_span: [call.span.start, call.span.end],
        operation: read.operation.clone(),
        access: read.access,
        path: tail.to_vec(),
    };
    Ok(actual.iter().cloned().chain([scope]).collect())
}

pub(super) fn verify_policy<'a>(
    pair: PairView<'_>,
    locations: impl Iterator<Item = &'a MemoryLocation>,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        CAPABILITY,
        || {
            locations
                .into_iter()
                .any(|location| location.abstraction == ABSTRACTION)
        },
        "Indexed helper reads require their registered document policy",
    )
}

pub(super) fn project(value: TypeToken, projection: &Projection) -> Result<TypeToken, String> {
    match (value, projection) {
        (
            TypeToken::Slice(element) | TypeToken::Array { element, .. },
            Projection::IteratorPrefix {
                operand_position: 0,
            },
        ) if matches!(element.as_ref(), TypeToken::Scalar(scalar) if scalar == "u8") => {
            Ok(TypeToken::Slice(element))
        }
        (
            TypeToken::Slice(element) | TypeToken::Array { element, .. },
            Projection::ByteWindow {
                operand_position: 0,
                length,
            },
        ) if matches!(element.as_ref(), TypeToken::Scalar(scalar) if scalar == "u8")
            && matches!(length, 2 | 4 | 16) =>
        {
            Ok(TypeToken::Array {
                element,
                length: *length,
            })
        }
        (value, Projection::HelperIndex { path, .. }) => {
            path.iter().try_fold(value, super::project)
        }
        (
            TypeToken::Slice(element) | TypeToken::Array { element, .. },
            Projection::IndexedElement {
                operand_position: 1,
            }
            | Projection::PrefixElement { .. },
        ) if matches!(element.as_ref(), TypeToken::Scalar(_)) => Ok(*element),
        _ => Err("Indexed helper selector contradicts its concrete type".into()),
    }
}
