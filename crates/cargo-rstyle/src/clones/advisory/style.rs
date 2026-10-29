//! Borrow completed evaluations for the root style command, retaining unit identity.
use super::{Report, Unit, UnitId};
use rstyle_core::report::{Diagnostic, FixGroup, Summary};
use serde::Serialize;

#[derive(Serialize)]
pub(in crate::clones) struct Projection<'a> {
    diagnostics: Vec<&'a Diagnostic>,
    diagnostic_units: Vec<UnitId>,
    summary: Option<Summary>,
    summary_complete: bool,
    fix_groups: Vec<&'a FixGroup>,
    functions: Vec<&'a rstyle_core::complexity_report::Function>,
    function_units: Vec<UnitId>,
    planning_input_complete: Option<bool>,
    planning_required_edits_complete: Option<bool>,
}

impl<'a> Projection<'a> {
    pub(in crate::clones) fn new(report: Option<&'a Report>) -> Result<Self, String> {
        let units = report.map_or(&[][..], |report| report.units.as_slice());
        let summaries: Vec<_> = units
            .iter()
            .filter_map(|unit| unit.evaluation.as_ref().map(|report| &report.summary))
            .collect();
        let mut summary = Summary::aggregate(summaries.iter().copied());
        if !summaries.is_empty() && summary.is_none() {
            return Err("Cannot aggregate completed compiler style summaries".into());
        }

        let (diagnostic_units, diagnostics): (Vec<_>, Vec<_>) =
            evaluated_rows(units, visible_diagnostics, rstyle_core::report::ordering)
                .into_iter()
                .unzip();
        if let Some(summary) = &mut summary {
            summary.violations = diagnostics.len();
        }
        let (function_units, functions) = evaluated_rows(
            units,
            |_, evaluation| evaluation.functions.iter(),
            rstyle_core::complexity_report::ordering,
        )
        .into_iter()
        .unzip();
        let planning = report.and_then(|report| report.planning.as_ref());

        Ok(Self {
            summary_complete: summary.is_some()
                && report.is_some_and(|report| report.complete)
                && summaries.len() == units.len(),
            diagnostics,
            diagnostic_units,
            summary,
            fix_groups: planning
                .into_iter()
                .flat_map(super::planning::Report::groups)
                .collect(),
            functions,
            function_units,
            planning_input_complete: planning.map(super::planning::Report::input_complete),
            planning_required_edits_complete: planning
                .map(super::planning::Report::required_edits_complete),
        })
    }

    pub(in crate::clones) fn text(&self) -> String {
        let mut lines: Vec<_> = self
            .diagnostics
            .iter()
            .map(|diagnostic| rstyle_core::report::diagnostic_line(diagnostic))
            .collect();

        if !self.fix_groups.is_empty() {
            lines.extend(rstyle_core::report::groups_text_refs(
                self.fix_groups.iter().copied(),
            ));
        }
        lines.extend(rstyle_core::complexity_report::text_refs(
            self.functions.iter().copied(),
        ));

        lines.extend([
            self.summary.as_ref().map_or_else(
                || "rstyle: summary unavailable; no completed compiler evaluations".into(),
                rstyle_core::report::summary_line,
            ),
            format!(
                "compiler style: summary complete={}, planning input complete={}, required edits complete={}",
                self.summary_complete,
                completion(self.planning_input_complete),
                completion(self.planning_required_edits_complete),
            ),
        ]);
        lines.join("\n")
    }
}

fn completion(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

fn visible_diagnostics<'a>(
    unit: &'a Unit,
    evaluation: &'a rstyle_core::report::Report,
) -> impl Iterator<Item = &'a Diagnostic> {
    let selected = unit
        .selected_diagnostics
        .as_ref()
        .or(unit.visible_diagnostics.as_ref());
    evaluation
        .diagnostics
        .iter()
        .enumerate()
        .filter(move |(index, _)| {
            selected.is_none_or(|indices| indices.binary_search(index).is_ok())
        })
        .map(|(_, diagnostic)| diagnostic)
}

fn evaluated_rows<'a, T: 'a, I: Iterator<Item = &'a T>>(
    units: &'a [Unit],
    select: impl Fn(&'a Unit, &'a rstyle_core::report::Report) -> I,
    compare: impl Fn(&T, &T) -> std::cmp::Ordering,
) -> Vec<(UnitId, &'a T)> {
    let mut rows: Vec<_> = units
        .iter()
        .flat_map(|unit| {
            let select = &select;
            unit.evaluation.iter().flat_map(move |evaluation| {
                select(unit, evaluation).map(move |row| (unit.unit, row))
            })
        })
        .collect();
    rows.sort_by(|(a_unit, a), (b_unit, b)| compare(a, b).then_with(|| a_unit.cmp(b_unit)));
    rows
}
