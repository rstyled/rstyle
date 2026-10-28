use super::super::{Function, Region, located_region, report};
use rstyle_pdg::{
    ByteSpan,
    consumer_facts::{
        Availability,
        planning::{self, SourceRegions},
    },
};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Regions {
    owners: BTreeMap<usize, Availability<SourceRegions>>,
}

impl Regions {
    fn owner(&mut self, record: &Function) -> Result<&Availability<SourceRegions>, String> {
        let index = record.facts.index;
        if let std::collections::btree_map::Entry::Vacant(entry) = self.owners.entry(index) {
            entry.insert(planning::inspect_regions(&record.facts)?);
        }
        Ok(&self.owners[&index])
    }

    pub(super) fn signature(
        &mut self,
        record: &Function,
        ctx: &report::Context<'_>,
    ) -> Result<Result<Region, String>, String> {
        let original = match self.owner(record)? {
            Availability::Known(regions) => regions.signature.clone(),
            Availability::Unknown { reason }
            | Availability::Unsupported { reason }
            | Availability::NotApplicable { reason } => return Ok(Err(reason.clone())),
        };
        Ok(located_region(record, ctx, original)?.map_err(|failure| failure.reason))
    }

    pub(super) fn call(
        &mut self,
        record: &Function,
        span: ByteSpan,
        ctx: &report::Context<'_>,
    ) -> Result<Result<Region, String>, String> {
        let mut original = match self.owner(record)? {
            Availability::Known(regions) => regions.body.clone(),
            Availability::Unknown { reason }
            | Availability::Unsupported { reason }
            | Availability::NotApplicable { reason } => return Ok(Err(reason.clone())),
        };

        let span = absolute(record.facts.fn_span, span)?;
        if span.start < original.span.start || span.end > original.span.end {
            return Err("Caller edit region escapes its original body".into());
        }
        original.span = span;
        Ok(located_region(record, ctx, original)?.map_err(|failure| failure.reason))
    }
}

pub(super) use rstyle_pdg::consumer_facts::planning::absolute_span as absolute;
