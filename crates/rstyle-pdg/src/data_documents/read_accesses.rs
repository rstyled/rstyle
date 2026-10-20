//! Shared installation of source-backed reads and their existing annotations.

use super::*;
use crate::{Access, HirNode, MemoryLocation};

pub(super) struct Effects {
    pub reads: Vec<Access>,
    pub locations: BTreeMap<String, MemoryLocation>,
    pub annotations: BTreeMap<String, serde_json::Value>,
}

pub(super) enum Kind {
    Index,
    Indirect,
    Byte,
    Option,
}

impl Kind {
    fn accepts(&self, node: &crate::PdgNode) -> bool {
        match self {
            Self::Option => node.attributes.construct.as_deref() == Some("data.option-payload"),
            Self::Byte => matches!(
                node.attributes.construct.as_deref(),
                Some(
                    "data.bytes.copy-result"
                        | "data.bytes.unwrap"
                        | "data.bytes.native-u32"
                        | "data.bytes.borrow-result"
                        | "data.bytes.unwrap-borrow"
                        | "data.bytes.native-usize"
                )
            ),
            Self::Index => matches!(
                node.attributes.construct.as_deref(),
                Some("data.index" | "data.value-index")
            ),
            Self::Indirect => matches!(
                node.attributes.construct.as_deref(),
                Some("data.indirect-field" | "data.indirect-deref")
            ),
        }
    }

    fn sources<'a>(
        &self,
        facts: &'a dyn crate::unit_store::EvidenceFacts,
    ) -> Result<Vec<&'a HirNode>, String> {
        match self {
            Self::Option => crate::checked_options::matches(facts)
                .into_iter()
                .filter(|node| {
                    node.children
                        .first()
                        .is_some_and(|scrutinee| !crate::checked_options::owned(scrutinee))
                })
                .map(|node| {
                    crate::checked_options::verify_match(node, facts).map(|checked| checked.binding)
                })
                .collect(),
            Self::Byte => Ok(super::byte_calls::sources(facts)),
            Self::Index => Ok(
                crate::field_locations::operations(&facts.body, &["expr_index"])
                    .into_iter()
                    .filter(|source| !crate::data_subslices::candidate(source, facts))
                    .collect(),
            ),
            Self::Indirect => {
                let places = crate::data_places::Places::verify(facts)?;
                Ok(
                    crate::field_locations::operations(&facts.body, &["expr_field", "expr_unary"])
                        .into_iter()
                        .filter(|source| {
                            crate::field_locations::derived::candidate(source, facts)
                                && !places.contains(source)
                        })
                        .collect(),
                )
            }
        }
    }

    fn effects(
        &self,
        source: &HirNode,
        context: &mut LocationContext<'_>,
    ) -> Result<Effects, String> {
        let (locations, annotations) = match self {
            Self::Option => (
                crate::checked_options::locations(source, context.facts)?,
                BTreeMap::from([(
                    crate::checked_options::KEY.into(),
                    crate::checked_options::semantics("data.option-payload"),
                )]),
            ),
            Self::Byte => {
                let kind = crate::data_bytes::verify(source, context.facts)?;
                (
                    crate::data_bytes::locations(source, context.facts, kind)?,
                    BTreeMap::from([(crate::data_bytes::KEY.into(), kind.semantics())]),
                )
            }
            Self::Index => {
                let checked = crate::data_index_reads::verify(source, context.facts)?;
                (
                    checked.locations(context.facts)?,
                    BTreeMap::from([(
                        crate::data_index_reads::BOUNDS_KEY.into(),
                        checked.bounds()?,
                    )]),
                )
            }
            Self::Indirect => (
                crate::field_locations::derived::locations(source, context.facts)?,
                BTreeMap::new(),
            ),
        };
        let (reads, locations) = context.read_accesses(locations)?;
        Ok(Effects {
            reads,
            locations,
            annotations,
        })
    }
}

pub(super) fn install(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
    kind: Kind,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    if !pair.pdg.nodes.iter().any(|node| kind.accepts(node)) {
        return Ok(BTreeMap::new());
    }
    let sources = kind.sources(context.facts)?;
    let locations = sources
        .into_iter()
        .map(|source| {
            let effects = kind.effects(source, context)?;
            install_node(pair, source, effects)
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(locations.into_iter().flatten().collect())
}

fn install_node(
    pair: &mut Pair,
    source: &HirNode,
    effects: Effects,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let node = super::serialized_operation_mut(pair, source)?;
    node.attributes.reads = effects.reads;
    node.annotations.extend(effects.annotations);

    Ok(effects.locations)
}
