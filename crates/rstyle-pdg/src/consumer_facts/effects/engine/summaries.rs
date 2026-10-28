use super::*;

impl Session<'_> {
    pub(super) fn retain_summary(&mut self, index: usize, depth: u8) -> Result<Prepared, String> {
        if depth == 0 {
            return self.retain_leaf(index);
        }
        if self.observations(index).unwrap().calls.is_empty() {
            return self.leaf_summary(index);
        }
        self.retain_wrapper(index)
    }

    fn retain_wrapper(&mut self, index: usize) -> Result<Prepared, String> {
        self.work.statistics.wrapper_checks += 1;
        let assessment = self.assess_observed(index, 0)?;
        self.complete_wrapper(index, &assessment)
    }

    fn complete_wrapper(
        &mut self,
        index: usize,
        assessment: &Assessment,
    ) -> Result<Prepared, String> {
        if assessment.coverage != mutations::Coverage::Complete {
            return Ok(Prepared::unavailable(
                "Callee wrapper is not complete within two call edges",
                assessment.execution == mutations::Execution::BudgetExhausted,
            ));
        }
        if !matches!(self.contexts.get(&index), Some(Availability::Known(_))) {
            return Ok(Prepared::unavailable(
                "Callee selected-unit context is unavailable",
                false,
            ));
        }

        let Some(parameters) = parameters(&self.unit.functions()[index])? else {
            return Ok(Prepared::unavailable(
                "Callee formal type authority is unavailable",
                false,
            ));
        };
        let cost = self.wrapper_cost(index, &assessment.calls)?;
        if !self.summary_fits(cost) {
            return Ok(Prepared::unavailable(
                "Effect summary retention budget exhausted",
                true,
            ));
        }

        let sites = self.wrapper_witnesses(index, &assessment.calls)?;
        Ok(self.publish_summary(
            Summary {
                function_index: index,
                parameters,
                sites,
            },
            cost,
        ))
    }

    pub(super) fn summary_fits(&self, cost: usize) -> bool {
        self.summaries.len() < self.limits.summaries
            && self
                .work
                .retained_sites
                .saturating_add(self.work.statistics.mapped_sites)
                .saturating_add(cost)
                <= self.limits.sites
    }

    pub(super) fn direct_witnesses(&self, index: usize) -> Vec<SiteWitness> {
        self.observations(index)
            .unwrap()
            .direct
            .sites
            .iter()
            .enumerate()
            .map(|(site_index, site)| SiteWitness {
                function_index: index,
                site_index,
                call_path: Vec::new(),
                root: site.root.clone(),
            })
            .collect()
    }

    pub(super) fn wrapper_cost(&self, index: usize, calls: &[CallEvent]) -> Result<usize, String> {
        let mut cost = self.observations(index).unwrap().direct.sites.len();
        for call in calls {
            for mapping in &call.mappings {
                let witness = self.summaries[call.summary_index]
                    .sites
                    .get(mapping.summary_site_index)
                    .ok_or("Effect wrapper mapping lacks its original summary witness")?;
                cost = cost
                    .saturating_add(2)
                    .saturating_add(witness.call_path.len());
            }
        }
        Ok(cost)
    }

    pub(super) fn wrapper_witnesses(
        &self,
        index: usize,
        calls: &[CallEvent],
    ) -> Result<Vec<SiteWitness>, String> {
        let mut sites = self.direct_witnesses(index);
        for call in calls {
            for mapping in &call.mappings {
                let original = self.summaries[call.summary_index]
                    .sites
                    .get(mapping.summary_site_index)
                    .ok_or("Effect wrapper mapping lacks its original summary witness")?;
                let call_path = std::iter::once(CallReference {
                    function_index: index,
                    call_index: call.call_index,
                })
                .chain(original.call_path.iter().copied())
                .collect();
                sites.push(SiteWitness {
                    function_index: original.function_index,
                    site_index: original.site_index,
                    call_path,
                    root: mapping.caller_root.clone(),
                });
            }
        }
        Ok(sites)
    }

    pub(super) fn publish_summary(&mut self, summary: Summary, cost: usize) -> Prepared {
        let visible = summary
            .sites
            .iter()
            .enumerate()
            .filter_map(|(index, site)| {
                (site.root.provenance == mutations::Provenance::FormalBorrowed).then_some(index)
            })
            .collect();
        let summary_index = self.summaries.len();

        self.work.retained_sites += cost;
        self.summaries.push(summary);
        self.visible_sites.push(visible);

        Prepared {
            summary: Some(summary_index),
            budget: false,
            reason: None,
        }
    }
}
