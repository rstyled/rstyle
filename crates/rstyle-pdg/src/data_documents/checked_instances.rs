//! Complete checked-instance call inventories share the paired policy boundary.
use crate::{HirNode, producer::PairView, unit_store::EvidenceFacts};

pub(super) trait Operations {
    const CONSTRUCT: &'static str;
    const CAPABILITY: &'static str;
    const POLICY_ERROR: &'static str;
    fn sources(facts: &dyn EvidenceFacts) -> Result<Vec<&HirNode>, String>;
    fn verify_operation(
        pair: PairView<'_>,
        facts: &dyn EvidenceFacts,
        source: &HirNode,
    ) -> Result<(), String>;
}

pub(super) fn verify<O: Operations>(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let sources = O::sources(facts)?;
    super::verify_construct_inventory(pair, [O::CONSTRUCT], [sources.len()])?;
    if !sources.is_empty()
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, O::CAPABILITY)
    {
        return Err(O::POLICY_ERROR.into());
    }

    for source in sources {
        O::verify_operation(pair, facts, source)?;
    }
    Ok(())
}
