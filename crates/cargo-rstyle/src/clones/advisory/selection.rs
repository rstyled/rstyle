//! File-level presentation reuses completed core paths and shared coverage aggregation.
use super::{Report, Unit};
use crate::clones::{
    analysis::{self, changed_scope, checked_path},
    report::Context,
};
use std::collections::BTreeSet;

impl Report {
    pub(in crate::clones) fn select_changed(mut self, ctx: &Context<'_>) -> Self {
        let Some(selection) = ctx
            .analysis_config
            .and_then(analysis::Config::changed_files)
        else {
            return self;
        };

        for unit in &mut self.units {
            unit.select_changed(selection);
        }

        self.changed_scope = Some(self.changed_report(ctx, selection));
        self
    }

    fn changed_report(
        &self,
        ctx: &Context<'_>,
        selection: &changed_scope::Selection,
    ) -> changed_scope::Report<'static> {
        let known: BTreeSet<_> = self
            .units
            .iter()
            .flat_map(|unit| &unit.observations)
            .map(|owner| owner.key)
            .collect();
        let owners = ctx
            .handoff
            .assessed
            .iter()
            .map(|function| changed_scope::Owner {
                path: analysis::source_file(function)
                    .and_then(|path| checked_path(path, &ctx.capture.plan.root)),
                complete: known.contains(&function.key),
                visible_findings: 0,
            });

        let mut report = changed_scope::evaluate(selection, owners);
        report.include_findings(self.units.iter().flat_map(Unit::selected_paths));
        report.into_owned()
    }

    pub(in crate::clones) fn finding_exit_count(&self) -> usize {
        self.units.iter().map(Unit::findings).sum()
    }
}

impl Unit {
    fn select_changed(&mut self, selection: &changed_scope::Selection) {
        self.selected_diagnostics = Some(
            self.evaluation
                .diagnostics
                .iter()
                .enumerate()
                .filter(|(index, diagnostic)| {
                    self.visible_diagnostics
                        .as_ref()
                        .is_none_or(|visible| visible.binary_search(index).is_ok())
                        && (diagnostic.rule_id == "invalid_suppression"
                            || selection.contains(&diagnostic.path))
                })
                .map(|(index, _)| index)
                .collect(),
        );
    }

    fn selected_paths(&self) -> impl Iterator<Item = &str> {
        self.selected_diagnostics
            .iter()
            .flatten()
            .map(|index| self.evaluation.diagnostics[*index].path.as_str())
    }
}
