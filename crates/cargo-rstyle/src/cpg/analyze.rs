//! One report over a run: the selected analyses in one pass, then whichever SARIF they make.
use crate::compiler::{DocumentReport, document_bounds};
use rstyle_core::config::Config;
use rstyle_core::cpg::flow;
use rstyle_core::cpg::records::{self, Record};
use rstyle_core::cpg::transport::{self, Encoding};
use rstyle_core::filter::{Analysis, Filter};
use rstyle_core::patterns_report::{self, PatternsReport};
use rstyle_core::report::{self, Report};
use rstyle_core::{sarif, unified};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// What one run analyzes, and how.
pub(crate) struct Request<'a> {
    pub filter: &'a Filter,
    pub config: &'a Config,
    pub options: &'a patterns_report::Options,
    pub top: Option<usize>,
}

/// An analysis a frontend ran itself. Its results stay in their own file, pending until the
/// report can hold them.
#[derive(Serialize)]
pub(crate) struct FrontendResult {
    pub analysis: String,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl FrontendResult {
    fn text(&self) -> String {
        match (&self.file, &self.error) {
            (Some(file), _) => format!("{}: pending in {}", self.analysis, file.display()),
            (None, Some(error)) => format!("{}: failed: {error}", self.analysis),
            (None, None) => format!("{}: {}", self.analysis, self.status),
        }
    }
}

/// A function's derived flow, in terms of its document's node ids.
#[derive(Serialize)]
pub(crate) struct FlowRow {
    pub function: String,
    pub name: String,
    pub path: String,
    pub flow: flow::Derived,
}

/// Each function's flow. A record's own flow moves here, since nothing else reads it.
#[derive(Serialize)]
pub(crate) struct FlowReport {
    pub functions: Vec<FlowRow>,
}

impl FlowReport {
    /// One line per function is `cpg flow`'s to print. A whole run's text says how many stand
    /// where, and the JSON report has each function.
    fn summary(&self) -> String {
        let count = |standing: flow::Standing| {
            self.functions
                .iter()
                .filter(|row| row.flow.standing == standing)
                .count()
        };
        format!(
            "Control flow of {} functions: {} known, {} approximated, {} unknown",
            self.functions.len(),
            count(flow::Standing::Known),
            count(flow::Standing::Approximated),
            count(flow::Standing::Unknown),
        )
    }
}

/// Moves out the flow of each record whose name holds `name`, when one is given.
pub(crate) fn take_flow(records: &mut [Record], name: Option<&str>) -> Vec<FlowRow> {
    records
        .iter_mut()
        .filter(|record| name.is_none_or(|name| record.name.contains(name)))
        .filter_map(|record| {
            Some(FlowRow {
                flow: record.flow.take()?,
                function: record.function.clone(),
                name: record.name.clone(),
                path: record.path.clone(),
            })
        })
        .collect()
}

/// A run's report. A filtered analysis stays absent. Nothing here says where or when the run
/// happened, so two runs over the same source are byte-comparable.
#[derive(Serialize)]
pub(crate) struct Unified {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check: Option<Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patterns: Option<PatternsReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_bounds: Option<DocumentReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<FlowReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frontend: Vec<FrontendResult>,
}

/// Fails before anything runs when the flags can't both be honored.
pub(crate) fn validate(filter: &Filter, fail_on_candidates: bool) -> Result<(), String> {
    if fail_on_candidates && !filter.core(Analysis::Patterns) {
        return Err(
            "--fail-on-candidates needs the patterns analysis, which the filter leaves out".into(),
        );
    }
    Ok(())
}

pub(crate) fn analyze(mut records: Vec<Record>, request: &Request<'_>) -> Result<Unified, String> {
    records::canonical_order(&mut records);
    let runtime_bounds = request
        .filter
        .core(Analysis::Bounds)
        .then(|| document_bounds(&records, request.top));
    let flow = request.filter.core(Analysis::Flow).then(|| FlowReport {
        functions: take_flow(&mut records, None),
    });

    let analyses = unified::analyze(records, request.filter, request.config, request.options)?;
    Ok(Unified {
        check: analyses.check,
        patterns: analyses.patterns,
        runtime_bounds,
        flow,
        frontend: Vec::new(),
    })
}

impl Unified {
    pub(crate) fn text(&self) -> String {
        let check = self.check.as_ref().map(report::text);
        let patterns = self.patterns.as_ref().map(patterns_report::text);
        let frontend = self.frontend.iter().map(FrontendResult::text);
        let bounds = self.runtime_bounds.as_ref().map(DocumentReport::text);
        let flow = self.flow.as_ref().map(FlowReport::summary);

        [check, patterns, bounds, flow]
            .into_iter()
            .flatten()
            .chain(frontend)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The worst result of everything that ran: 0 clean, 1 findings, 2 incomplete.
    pub(crate) fn exit_code(&self, fail_on_candidates: bool) -> u8 {
        let check = self.check.as_ref().map_or(0, Report::exit_code);
        let candidates = self
            .patterns
            .as_ref()
            .is_some_and(|report| fail_on_candidates && !report.candidates.is_empty());
        let frontend = self.frontend.iter().any(|result| result.error.is_some());
        check
            .max(u8::from(candidates))
            .max(if frontend { 2 } else { 0 })
    }

    /// One file per analysis in `directory`; `root` is what their relative paths resolve against.
    pub(crate) fn write_sarif(&self, directory: &Path, root: &Path) -> Result<(), String> {
        if let Some(report) = &self.check {
            write(&directory.join("check.sarif"), &sarif::check(report, root)?)?;
        }
        if let Some(report) = &self.patterns {
            write(
                &directory.join("patterns.sarif"),
                &sarif::patterns(report, root)?,
            )?;
        }
        Ok(())
    }
}

fn write(path: &Path, log: &sarif::Log) -> Result<(), String> {
    transport::write_json(path, Encoding::Json, log).map(|_| ())
}
