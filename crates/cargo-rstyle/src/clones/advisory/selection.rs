//! File-level presentation reuses completed core paths and shared coverage aggregation.
use super::{Report, Unit};
use crate::clones::{
    analysis::{self, changed_scope, checked_path},
    report::Context,
};
use std::collections::BTreeSet;
use std::{borrow::Cow, path::Path};

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
        let known = self.known_selection_inputs();
        let pattern_outside: BTreeSet<_> = self
            .units
            .iter()
            .filter_map(|unit| unit.patterns.as_ref())
            .flat_map(super::patterns::Report::outside_scope)
            .collect();
        let mut outside = if ctx.args.compiler_advisory {
            BTreeSet::new()
        } else {
            pattern_outside.clone()
        };
        outside.extend(
            self.units
                .iter()
                .filter_map(|unit| unit.diagnostic_scope.as_ref())
                .flat_map(super::diagnostic_scope::Report::outside_scope)
                .filter(|key| !ctx.args.compiler_patterns || pattern_outside.contains(key)),
        );
        let paths = source_paths(ctx, self.metrics);
        let owners = ctx
            .handoff
            .assessed
            .iter()
            .zip(&paths)
            .map(|(function, path)| changed_scope::Owner {
                path: path.as_deref(),
                complete: known.contains(&function.key),
                outside_scope: outside.contains(&function.key),
                visible_findings: 0,
            });

        let mut report = changed_scope::evaluate(selection, owners);
        if ctx.args.compiler_advisory {
            report.include_findings(self.units.iter().flat_map(Unit::selected_paths));
        }
        report.include_findings(
            self.units
                .iter()
                .filter_map(|unit| unit.patterns.as_ref())
                .flat_map(|patterns| patterns.selected_paths(selection)),
        );
        report.into_owned()
    }

    fn known_selection_inputs(&self) -> BTreeSet<crate::clones::FunctionKey> {
        let mut known: BTreeSet<_> = self
            .units
            .iter()
            .flat_map(|unit| &unit.observations)
            .map(|owner| owner.key)
            .collect();
        for key in self
            .units
            .iter()
            .filter_map(|unit| unit.patterns.as_ref())
            .flat_map(super::patterns::Report::missing_graphs)
        {
            known.remove(&key);
        }
        known
    }

    pub(in crate::clones) fn finding_exit_count(&self) -> usize {
        self.units
            .iter()
            .map(|unit| unit.findings(super::patterns::Report::selected_findings))
            .sum()
    }
}

fn source_paths<'a>(ctx: &'a Context<'_>, metrics: &str) -> Vec<Option<Cow<'a, str>>> {
    ctx.handoff
        .assessed
        .iter()
        .map(|function| {
            analysis::source_file(function)
                .and_then(|path| source_path(path, &ctx.capture.plan.root, metrics))
        })
        .collect()
}

fn source_path<'a>(original: &'a str, root: &Path, metrics: &str) -> Option<Cow<'a, str>> {
    if let Some(path) = checked_path(original, root) {
        return Some(Cow::Borrowed(path));
    }
    if !super::normalized_source_paths(metrics) {
        return None;
    }
    let normalized = rstyle_pdg::nominal_binding::absolute_path(Path::new(original)).ok()?;
    checked_path(normalized.to_str()?, root).map(|path| Cow::Owned(path.to_owned()))
}

impl Unit {
    fn select_changed(&mut self, selection: &changed_scope::Selection) {
        let Some(evaluation) = &self.evaluation else {
            return;
        };
        self.selected_diagnostics = Some(
            evaluation
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
            .flat_map(|index| {
                self.evaluation
                    .as_ref()
                    .map(|evaluation| evaluation.diagnostics[*index].path.as_str())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_paths_borrow_canonical_locations_and_preserve_metric_version_boundaries() {
        let root = Path::new("/workspace");
        let canonical = "/workspace/src/shared.rs";
        assert!(matches!(
            source_path(canonical, root, super::super::METRICS),
            Some(Cow::Borrowed("src/shared.rs"))
        ));
        for metrics in [
            rstyle_core::advisory::V5_METRICS,
            rstyle_core::advisory::V6_METRICS,
            super::super::METRICS,
        ] {
            assert_eq!(
                source_path("/workspace/src/nested/../shared.rs", root, metrics).as_deref(),
                Some("src/shared.rs")
            );
            for invalid in [
                "relative.rs",
                "/../shared.rs",
                "/workspace/src/bad:name.rs",
                "/workspace/src/bad\nname.rs",
            ] {
                assert!(source_path(invalid, root, metrics).is_none(), "{invalid:?}");
            }
        }
        assert!(
            source_path(
                "/workspace/src/nested/../shared.rs",
                root,
                rstyle_core::advisory::V4_METRICS
            )
            .is_none()
        );
    }
}
