//! Borrow completed evaluations for the root style command, retaining unit identity.
use super::{Report, Unit, UnitId};
use crate::clones::analysis::Completion;
use rstyle_core::report::{Diagnostic, FixGroup, Summary};
use serde::Serialize;

#[derive(Serialize)]
pub(in crate::clones) struct Projection<'a> {
    diagnostics: Vec<&'a Diagnostic>,
    diagnostic_units: Vec<UnitId>,
    diagnostic_indices: Vec<usize>,
    summary: Option<Summary>,
    summary_complete: bool,
    fix_groups: Vec<&'a FixGroup>,
    functions: Vec<&'a rstyle_core::complexity_report::Function>,
    function_units: Vec<UnitId>,
    planning_input_complete: Option<bool>,
    planning_required_edits_complete: Option<bool>,
    #[serde(skip)]
    workflow: Option<&'a Report>,
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

        let rows = evaluated_rows(units, visible_diagnostics, rstyle_core::report::ordering);
        let diagnostic_indices = rows.iter().map(|(_, index, _)| *index).collect();
        let (diagnostic_units, diagnostics): (Vec<_>, Vec<_>) =
            rows.into_iter().map(|(unit, _, row)| (unit, row)).unzip();
        if let Some(summary) = &mut summary {
            summary.violations = diagnostics.len();
        }
        let (function_units, functions) = evaluated_rows(
            units,
            |_, evaluation| evaluation.functions.iter().enumerate(),
            rstyle_core::complexity_report::ordering,
        )
        .into_iter()
        .map(|(unit, _, row)| (unit, row))
        .unzip();
        let planning = report.and_then(|report| report.planning.as_ref());

        Ok(Self {
            summary_complete: summary.is_some()
                && report.is_some_and(Completion::permits_success)
                && summaries.len() == units.len(),
            diagnostics,
            diagnostic_units,
            diagnostic_indices,
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
            workflow: report,
        })
    }

    pub(in crate::clones) fn text(&self) -> String {
        let mut lines: Vec<_> = self
            .diagnostics
            .iter()
            .zip(&self.diagnostic_units)
            .zip(&self.diagnostic_indices)
            .map(|((diagnostic, unit), index)| {
                format!(
                    "{} [diagnostic {}] [compiler unit {}]",
                    rstyle_core::report::diagnostic_line(diagnostic),
                    index,
                    unit.ordinal(),
                )
            })
            .collect();

        if !self.fix_groups.is_empty() {
            lines.extend(rstyle_core::report::groups_text_refs(
                self.fix_groups.iter().copied(),
            ));
        }
        lines.extend(self.function_text());

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
        let workflow = self
            .workflow
            .map_or_else(String::new, Report::render_summaries);
        format!("{}{workflow}", lines.join("\n"))
    }

    fn function_text(&self) -> Vec<String> {
        let mut lines = rstyle_core::complexity_report::text_refs(self.functions.iter().copied());
        if let Some(header) = lines.first_mut() {
            header.push_str("  UNIT");
        }
        for (line, unit) in lines.iter_mut().skip(1).zip(&self.function_units) {
            *line = format!("{line} [compiler unit {}]", unit.ordinal());
        }
        lines
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
) -> impl Iterator<Item = (usize, &'a Diagnostic)> {
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
}

fn evaluated_rows<'a, T: 'a, I: Iterator<Item = (usize, &'a T)>>(
    units: &'a [Unit],
    select: impl Fn(&'a Unit, &'a rstyle_core::report::Report) -> I,
    compare: impl Fn(&T, &T) -> std::cmp::Ordering,
) -> Vec<(UnitId, usize, &'a T)> {
    let mut rows: Vec<_> = units
        .iter()
        .flat_map(|unit| {
            let select = &select;
            unit.evaluation.iter().flat_map(move |evaluation| {
                select(unit, evaluation).map(move |(index, row)| (unit.unit, index, row))
            })
        })
        .collect();
    rows.sort_by(|(a_unit, _, a), (b_unit, _, b)| compare(a, b).then_with(|| a_unit.cmp(b_unit)));
    rows
}
