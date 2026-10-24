//! Check complete evaluated table values without re-running graph production.
use super::*;
use crate::{bool_constants, unit_store::EvidenceFacts};

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let mut count = 0;
    for source in crate::field_locations::operations(&facts.body, &["expr_path"]) {
        if !source
            .res
            .as_ref()
            .is_some_and(|res| res.kind == bool_constants::RESOLUTION)
        {
            continue;
        }
        if !crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            bool_constants::CAPABILITY,
        ) {
            return Err("Const tables require their registered document policy".into());
        }
        let record = bool_constants::verify_use(source, facts)?;
        let node = super::value_operation(pair, source)?;
        super::verify_value_effects(node, true, "Evaluated table invents runtime effects")?;
        if node.category != NodeCategory::Computation
            || node.attributes.construct.as_deref() != Some("constant.bool-array")
            || !node.attributes.outcomes.is_empty()
            || node.annotations.contains_key(crate::data_places::KEY)
            || node
                .attributes
                .literal
                .as_ref()
                .and_then(|value| value.known_value())
                != Some(&record.literal())
        {
            return Err("Serialized table loses its complete evaluated value/category".into());
        }
        super::verify_saved_operands(pair, node, &[])?;
        count += 1;
    }
    super::verify_construct_inventory(pair, ["constant.bool-array"], [count])
}
