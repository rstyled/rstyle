//! Exact serialized guarded component inventory.
use super::*;
use crate::{data_types::TypeToken, producer::Atom};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(test)]
mod tests;
mod wire_values;

pub(super) const KEY: &str = "rstyle.data:digit-product-origins";
pub(super) type Guards = Vec<Vec<Atom>>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Component {
    pub selection: Vec<usize>,
    pub ty: TypeToken,
    pub origins: Vec<Origin>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Case {
    pub tag: String,
    pub guards: Guards,
    pub components: Vec<Component>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Footprint {
    Direct {
        origins: Vec<Origin>,
        guards: Guards,
    },
    Product {
        components: Vec<Component>,
        guards: Guards,
    },
    Tagged {
        cases: Vec<Case>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Value {
    pub ty: TypeToken,
    pub footprint: Footprint,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Contract {
    pub instance: super::super::Observation,
    pub prefix: crate::slice_patterns::semantics::Contract,
    #[serde(with = "wire_values")]
    pub values: BTreeMap<String, Value>,
}

impl Contract {
    pub fn new(checked: &Checked<'_>) -> Self {
        Self {
            instance: checked.instance().clone(),
            prefix: checked.prefix().contract(),
            values: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, node: &crate::PdgNode, value: Value) -> Result<(), String> {
        if self
            .values
            .get(&node.id)
            .is_some_and(|previous| previous != &value)
        {
            return Err("Digit product value has conflicting complete origin roles".into());
        }

        self.values.insert(node.id.clone(), value);
        Ok(())
    }
}

pub(super) fn components(checked: &Checked<'_>, tagged: bool) -> Result<Vec<Component>, String> {
    checked
        .instance()
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            Ok(Component {
                selection: if tagged { vec![0, index] } else { vec![index] },
                ty: TypeToken::from_tree(&field.ty),
                origins: if index == 0 {
                    vec![source::suffix(checked)?]
                } else {
                    Vec::new()
                },
            })
        })
        .collect()
}
