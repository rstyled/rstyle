//! Publish source-derived origin records without weakening per-family coverage.
use super::*;
use serde_json::Value;

#[derive(Clone, Copy)]
pub(super) enum Coverage {
    Complete(&'static str),
    Matching,
}

pub(super) struct Policy {
    pub key: &'static str,
    pub coverage: Coverage,
}

pub(super) struct Replay<S, G> {
    pub source: S,
    pub graph: G,
}

impl Policy {
    pub(super) fn install<'a, T, S, G>(
        &self,
        pair: &mut Pair,
        facts: &'a dyn crate::unit_store::EvidenceFacts,
        replay: Replay<S, G>,
    ) -> Result<bool, String>
    where
        S: FnOnce(&'a dyn crate::unit_store::EvidenceFacts) -> Result<Option<T>, String>,
        G: FnOnce(
            PairView<'_>,
            &'a dyn crate::unit_store::EvidenceFacts,
            &T,
        ) -> Result<BTreeMap<String, Value>, String>,
    {
        let checked = (replay.source)(facts)?;
        let Some(checked) = checked else {
            return Ok(false);
        };
        let expected = (replay.graph)(pair.as_view(), facts, &checked)?;

        install_records(pair, self.key, &expected, self.coverage)?;
        Ok(true)
    }
}

pub(super) fn install_records(
    pair: &mut Pair,
    key: &str,
    expected: &BTreeMap<String, Value>,
    coverage: Coverage,
) -> Result<(), String> {
    for node in &mut pair.pdg.nodes {
        match (expected.get(&node.id), &coverage) {
            (Some(record), _) => {
                node.annotations.insert(key.into(), record.clone());
            }
            (None, Coverage::Complete(diagnostic)) => return Err((*diagnostic).into()),
            (None, Coverage::Matching) => {}
        }
    }
    Ok(())
}

/// Each source role owns one serialized operation; aliasing cannot overwrite proof.
pub(super) fn insert_role(
    roles: &mut BTreeMap<String, Value>,
    node: &PdgNode,
    value: impl serde::Serialize,
) -> Result<(), String> {
    let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
    if roles.insert(node.id.clone(), value).is_some() {
        return Err("Origin roles alias the same serialized operation".into());
    }
    Ok(())
}
