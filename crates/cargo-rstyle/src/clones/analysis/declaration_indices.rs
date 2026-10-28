//! Shared immutable original declaration indexes for local abstraction consumers.
use crate::clones::{Args, Handoff, identity::UnitId};
use rstyle_pdg::consumer_facts::dispatch;
use std::collections::BTreeMap;

pub(in crate::clones) struct Indices {
    units: BTreeMap<UnitId, dispatch::Index>,
}

impl Indices {
    pub fn new(handoff: &Handoff, args: &Args) -> Result<Self, String> {
        let limit = if args.enum_dispatch {
            args.max_analysis_functions.unwrap_or(usize::MAX)
        } else if args.capability_proposals
            || ((args.source_rules || args.mutation_rules || args.effect_density)
                && (args.review_file.is_some()
                    || args.source_baseline.is_some()
                    || args.write_source_baseline.is_some()
                    || args.density_baseline.is_some()
                    || args.write_density_baseline.is_some()))
        {
            args.max_analysis_functions.unwrap_or(4096).min(4096)
        } else {
            0
        };

        let mut units = BTreeMap::new();
        for function in handoff.assessed.iter().take(limit) {
            units
                .entry(function.key.unit())
                .or_insert_with(Vec::new)
                .push(&*function.facts);
        }
        let units = units
            .into_iter()
            .map(|(unit, facts)| dispatch::Index::from_facts(&facts).map(|index| (unit, index)))
            .collect::<Result<_, _>>()?;
        Ok(Self { units })
    }

    pub fn get(&self, unit: UnitId) -> Result<&dispatch::Index, String> {
        self.units
            .get(&unit)
            .ok_or_else(|| "Original declaration unit was not indexed".into())
    }
}
