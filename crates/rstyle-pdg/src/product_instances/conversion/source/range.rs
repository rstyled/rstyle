//! Structural Range identity joins the independently captured cleanup/type authority.
use super::*;

pub(in super::super) fn verify(
    canonical: &HirFnFacts,
    observed: &Observation,
    tree: &TypeTree,
) -> Result<Structural, String> {
    let TypeTree::RangeUsize { observation } = tree else {
        return Err("Consuming conversion requires captured Range<usize> authority".into());
    };
    crate::range_values::authority::verify(observation, canonical)?;

    let [receiver] = observed.receiver_type.as_slice() else {
        return Err("Consuming Range conversion requires its exact receiver type".into());
    };
    receiver.verify(&receiver.identity)?;
    let Structural::Adt {
        definition,
        arguments,
    } = &receiver.identity
    else {
        return Err("Consuming Range receiver lacks its actual compiler ADT identity".into());
    };

    if definition.local
        || definition.path != observation.definition.path
        || definition.definition_identity != observation.definition.definition_hash
        || definition.crate_name != observation.definition.crate_name
        || definition.stable_crate_id != observed.into.requested.stable_crate_id
        || definition
            .crate_hash
            .as_ref()
            .is_some_and(|hash| hash != &observation.definition.crate_hash)
        || !matches!(arguments.as_slice(), [Structural::Builtin { name }] if name == "usize")
    {
        return Err(
            "Range receiver differs from its pinned concrete type/cleanup observation".into(),
        );
    }

    let mut definition = definition.clone();
    definition.crate_hash = Some(observation.definition.crate_hash.clone());
    Ok(Structural::Adt {
        definition,
        arguments: arguments.clone(),
    })
}
