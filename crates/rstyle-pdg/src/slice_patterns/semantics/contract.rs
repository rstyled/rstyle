//! Bounded semantic roles use real pattern/expression HIR identities only.
use super::*;
use crate::HirRef;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Region {
    Element {
        position: u8,
    },
    /// Pointer = saved input pointer + start; length = saved input length - start.
    /// Empty suffix retains the one-past pointer of the exact saved input.
    Suffix {
        start: u8,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum SelectorKind {
    Length { minimum: u8 },
    Digit { position: u8, lower: u8, upper: u8 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "source",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum Destination {
    Selector(HirRef),
    Success(HirRef),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Selector {
    pub source: HirRef,
    pub kind: SelectorKind,
    pub saved_input: HirRef,
    /// This predecessor's true edge must dominate this operation.
    pub requires: Option<HirRef>,
    pub on_true: Destination,
    pub on_false: HirRef,
    /// Length tests read metadata only. Digit tests retain their content READ,
    /// including the mismatch edge leading to None.
    pub content: Option<Region>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ElementRead {
    pub binding: HirRef,
    pub pointer: HirRef,
    pub read: HirRef,
    pub region: Region,
    /// The final digit selector's true edge guards binding and body dereference.
    pub requires: HirRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Suffix {
    pub binding: HirRef,
    pub rest: HirRef,
    pub value: HirRef,
    pub region: Region,
    pub requires: HirRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Contract {
    pub formal: HirRef,
    pub saved_input: HirRef,
    pub pattern: HirRef,
    pub consumed: u8,
    pub selectors: Vec<Selector>,
    pub elements: Vec<ElementRead>,
    /// Fresh reference value; neither a byte-content read nor a stored-slot read.
    pub suffix: Suffix,
    pub none: HirRef,
    /// None is a complete explicit empty case, never inferred from absent data.
    pub none_references: Vec<Region>,
}

impl Contract {
    /// The caller supplies independently checked source semantics. Wire roles
    /// cannot establish source authority or CFG dominance by themselves.
    #[cfg(test)]
    pub(crate) fn validate(&self, checked: &Checked<'_>) -> Result<(), String> {
        if self != &derive(checked) {
            return Err("Prefix selector, element or suffix contract changes original saved-input semantics".into());
        }
        Ok(())
    }
}

pub(super) fn derive(checked: &Checked<'_>) -> Contract {
    let final_selector = &checked
        .digits
        .last()
        .expect("checked nonempty prefix")
        .range
        .hir;
    Contract {
        formal: checked.formal.hir.clone(),
        saved_input: checked.scrutinee.hir.clone(),
        pattern: checked.prefix.pattern().hir.clone(),
        consumed: checked.consumed(),
        selectors: selectors(checked),
        elements: elements(checked, final_selector),
        suffix: suffix(checked, final_selector),
        none: checked.failure.hir.clone(),
        none_references: Vec::new(),
    }
}

fn elements(checked: &Checked<'_>, final_selector: &HirRef) -> Vec<ElementRead> {
    checked
        .digits
        .iter()
        .enumerate()
        .map(|(position, digit)| ElementRead {
            binding: digit.binding.hir.clone(),
            pointer: digit.pointer.hir.clone(),
            read: digit.read.hir.clone(),
            region: Region::Element {
                position: position as u8,
            },
            requires: final_selector.clone(),
        })
        .collect()
}

fn suffix(checked: &Checked<'_>, final_selector: &HirRef) -> Suffix {
    let rest = checked.prefix.remainder();
    Suffix {
        binding: rest.binding().hir.clone(),
        rest: rest.rest().hir.clone(),
        value: checked.remainder.hir.clone(),
        region: Region::Suffix {
            start: checked.consumed(),
        },
        requires: final_selector.clone(),
    }
}

fn selectors(checked: &Checked<'_>) -> Vec<Selector> {
    let sources: Vec<_> = std::iter::once(checked.prefix.pattern())
        .chain(checked.digits.iter().map(|digit| digit.range))
        .collect();
    sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            let position = index.saturating_sub(1) as u8;
            Selector {
                source: source.hir.clone(),
                kind: if index == 0 {
                    SelectorKind::Length {
                        minimum: checked.consumed(),
                    }
                } else {
                    SelectorKind::Digit {
                        position,
                        lower: b'0',
                        upper: b'9',
                    }
                },
                saved_input: checked.scrutinee.hir.clone(),
                requires: index
                    .checked_sub(1)
                    .map(|previous| sources[previous].hir.clone()),
                on_true: sources.get(index + 1).map_or_else(
                    || Destination::Success(checked.success.hir.clone()),
                    |next| Destination::Selector(next.hir.clone()),
                ),
                on_false: checked.failure.hir.clone(),
                content: (index != 0).then_some(Region::Element { position }),
            }
        })
        .collect()
}
