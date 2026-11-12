//! One pass over a run's records: every selected analysis in one place, each handing what it
//! completed to the next, so core derives facts and effects once however many analyses read them.

use crate::config::Config;
use crate::cpg::bounds::report::{DocumentReport, from_records};
use crate::cpg::flow::{Derived, Standing};
use crate::cpg::records::{self, Measured, Record};
use crate::cpg::{self, RunInputs};
use crate::defects_report::{self, DefectsReport};
use crate::filter::{Analysis, Filter};
use crate::patterns_report::{self, PatternsReport};
use crate::report::{self, Report};
use crate::{Retention, candidates, rules, sarif};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// What the selected analyses reported; a filtered one stays absent. Nothing here says where or
/// when the run happened, so two runs over the same source are byte-comparable.
#[derive(Default, Serialize)]
pub struct Analyses {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check: Option<Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patterns: Option<PatternsReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_bounds: Option<DocumentReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<FlowReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defects: Option<DefectsReport>,
}

/// A function's derived flow, in terms of its document's node ids.
#[derive(Serialize)]
pub struct FlowRow {
    pub function: String,
    pub name: String,
    pub path: String,
    pub flow: Derived,
}

/// Each function's flow.
#[derive(Serialize)]
pub struct FlowReport {
    pub functions: Vec<FlowRow>,
}

impl FlowReport {
    /// One line per function is `cpg flow`'s to print. A whole run's text says how many stand
    /// where, and the JSON report has each function.
    pub fn summary(&self) -> String {
        let count = |standing: Standing| {
            self.functions
                .iter()
                .filter(|row| row.flow.standing == standing)
                .count()
        };
        format!(
            "Control flow of {} functions: {} known, {} approximated, {} unknown",
            self.functions.len(),
            count(Standing::Known),
            count(Standing::Approximated),
            count(Standing::Unknown),
        )
    }
}

/// Moves out the flow of each record whose name holds `name`, when one is given. Nothing else
/// reads a record's flow, so it needn't be copied.
pub fn take_flow(records: &mut [Record], name: Option<&str>) -> Vec<FlowRow> {
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

/// Runs the selected analyses over records in canonical order. `options.top` also limits the
/// bounds rows.
pub fn analyze(
    mut records: Vec<Record>,
    filter: &Filter,
    config: &Config,
    options: &patterns_report::Options,
) -> Result<Analyses, String> {
    reject_a_run_outside_its_root(&records)?;
    records::canonical_order(&mut records);
    let runtime_bounds = filter
        .core(Analysis::Bounds)
        .then(|| from_records(&records, options.top));
    let flow = filter.core(Analysis::Flow).then(|| FlowReport {
        functions: take_flow(&mut records, None),
    });
    let defects = filter
        .core(Analysis::Defects)
        .then(|| defects_report::from_records(&mut records, config))
        .transpose()?;

    let (check, patterns) = checked_and_mined(records, filter, config, options)?;
    Ok(Analyses {
        check,
        patterns,
        runtime_bounds,
        flow,
        defects,
    })
}

/// Analyses read only what's under the source root. If nothing is, the root and the frontend
/// disagree about where the files are, and a report would read as clean having read nothing.
fn reject_a_run_outside_its_root(records: &[Record]) -> Result<(), String> {
    // What a build wrote legitimately lies outside the root when the target directory does.
    let outside = |record: &Record| {
        record.path.starts_with("../")
            && !matches!(&record.measured, Measured::Generated(observation) if observation.generated_file)
    };
    match records.first() {
        Some(first) if records.iter().all(outside) => Err(format!(
            "None of this run's {} functions are under its source root, such as `{}`. Nothing \
             would be analyzed, so this isn't reported as a clean run.",
            records.len(),
            first.path
        )),
        _ => Ok(()),
    }
}

impl Analyses {
    pub fn text(&self) -> String {
        let check = self.check.as_ref().map(report::text);
        let patterns = self.patterns.as_ref().map(patterns_report::text);
        let bounds = self.runtime_bounds.as_ref().map(DocumentReport::text);
        let flow = self.flow.as_ref().map(FlowReport::summary);
        let defects = self.defects.as_ref().map(DefectsReport::text);

        [check, patterns, bounds, flow, defects]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The worst result of what ran: 0 clean, 1 findings, 2 incomplete.
    pub fn exit_code(&self, fail_on_candidates: bool) -> u8 {
        let check = self.check.as_ref().map_or(0, Report::exit_code);
        let candidates = self
            .patterns
            .as_ref()
            .is_some_and(|report| fail_on_candidates && !report.candidates.is_empty());
        check.max(u8::from(candidates))
    }

    /// One log per analysis that makes SARIF, with its file name. `root` is what their relative
    /// paths resolve against, and `config` says how loudly superlinear bounds are reported.
    pub fn sarif(
        &self,
        root: &Path,
        config: &Config,
    ) -> Result<Vec<(&'static str, sarif::Log)>, String> {
        let check = self.check.as_ref().map(|report| {
            let bounds = self.runtime_bounds.as_ref();
            let bounds = bounds.map(|bounds| (bounds, &config.algorithmic_complexity));
            Ok(("check.sarif", sarif::check(report, bounds, root)?))
        });
        let patterns = self
            .patterns
            .as_ref()
            .map(|report| Ok(("patterns.sarif", sarif::patterns(report, root)?)));
        let defects = self
            .defects
            .as_ref()
            .map(|report| Ok(("defects.sarif", sarif::defects(report, root)?)));
        [check, patterns, defects].into_iter().flatten().collect()
    }
}

fn checked_and_mined(
    records: Vec<Record>,
    filter: &Filter,
    config: &Config,
    options: &patterns_report::Options,
) -> Result<(Option<Report>, Option<PatternsReport>), String> {
    let check = filter.core(Analysis::Check);
    if !filter.core(Analysis::Patterns) {
        return match check {
            true => Ok((
                Some(checked(cpg::run_inputs(records, config)?, config)?),
                None,
            )),
            false => Ok((None, None)),
        };
    }

    let inputs = cpg::run_inputs(records, config)?;
    let completed = match check {
        true => check_then_observed(inputs, config)?,
        false => observed(inputs, config)?,
    };
    let (mut patterns, _) =
        patterns_report::run_distinct_owned_and_retain(completed.facts, options);
    candidates::annotate_completed_effects(&mut patterns.candidates, &completed.effects)?;
    Ok((completed.report, Some(patterns)))
}

fn checked(inputs: RunInputs, config: &Config) -> Result<Report, String> {
    crate::evaluate_with_effects(inputs.facts, config, Retention::Report, inputs.effects)
        .map(|evaluation| evaluation.report)
}

struct Completed {
    report: Option<Report>,
    facts: Vec<crate::facts::FnFacts>,
    effects: BTreeMap<String, Vec<rules::Effect>>,
}

fn check_then_observed(inputs: RunInputs, config: &Config) -> Result<Completed, String> {
    let evaluation = crate::evaluate_retaining_effects(
        inputs.facts,
        config,
        Retention::Report,
        Some(inputs.effects),
    )?;
    Ok(Completed {
        report: Some(evaluation.report),
        facts: evaluation.facts,
        effects: evaluation
            .effects
            .ok_or("The evaluation did not retain its effects")?,
    })
}

fn observed(inputs: RunInputs, config: &Config) -> Result<Completed, String> {
    let observed = crate::effect_observations(inputs.facts, config, Some(inputs.effects))?;
    Ok(Completed {
        report: None,
        facts: observed.facts,
        effects: observed.effects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpg::read_records;
    use serde_json::Value;

    fn run(names: &[&str]) -> Analyses {
        let filter = Filter::parse(names).unwrap();
        let document = serde_json::from_str(include_str!("cpg/testdata/example.cpg.json")).unwrap();
        let records = read_records([Ok(document)], filter.demand()).unwrap();
        let options = patterns_report::Options::default();
        analyze(records, &filter, &Config::default(), &options).unwrap()
    }

    #[test]
    fn no_filter_runs_every_analysis() {
        let analyses = run(&[]);
        assert!(analyses.check.is_some() && analyses.patterns.is_some());
        assert!(analyses.runtime_bounds.is_some() && analyses.flow.is_some());
        assert!(analyses.defects.is_some());
        let text = analyses.text();
        assert!(text.contains("Runtime upper bounds"), "{text}");
        assert!(text.contains("Control flow of 3 functions"), "{text}");
    }

    #[test]
    fn a_filtered_analysis_is_absent_and_what_ran_is_unchanged() {
        let whole = serde_json::to_value(run(&[])).unwrap();
        for (name, key) in [
            ("check", "check"),
            ("patterns", "patterns"),
            ("bounds", "runtime_bounds"),
            ("flow", "flow"),
            ("defects", "defects"),
        ] {
            let alone = serde_json::to_value(run(&[name])).unwrap();
            let object = alone.as_object().unwrap();
            assert_eq!(object.keys().collect::<Vec<_>>(), [key], "{name}");
            assert_eq!(object[key], whole[key], "{name}");
        }
    }

    #[test]
    fn sarif_comes_only_from_the_analyses_that_make_it() {
        let names = |analyses: &Analyses| -> Vec<&str> {
            let root = Path::new(if cfg!(windows) { r"C:\w" } else { "/w" });
            let logs = analyses.sarif(root, &Config::default()).unwrap();
            logs.into_iter().map(|(name, _)| name).collect()
        };
        assert_eq!(
            names(&run(&[])),
            ["check.sarif", "patterns.sarif", "defects.sarif"]
        );
        assert_eq!(names(&run(&["patterns"])), ["patterns.sarif"]);
        assert_eq!(names(&run(&["defects"])), ["defects.sarif"]);
        assert!(names(&run(&["bounds", "flow"])).is_empty());
    }

    #[test]
    fn a_run_with_nothing_under_its_root_is_an_error_not_a_clean_report() {
        let mut document: Value =
            serde_json::from_str(include_str!("cpg/testdata/example.cpg.json")).unwrap();
        for file in document["unit"]["files"].as_array_mut().unwrap() {
            file["path"] = format!("../elsewhere/{}", file["path"].as_str().unwrap()).into();
        }
        let filter = Filter::Everything;
        let records = read_records([Ok(document)], filter.demand()).unwrap();
        let options = patterns_report::Options::default();
        let result = analyze(records, &filter, &Config::default(), &options);
        let error = result.err().expect("no error");
        assert!(error.contains("None of this run's 3 functions"), "{error}");
        assert!(error.contains("../elsewhere/"), "{error}");
    }

    #[test]
    fn a_run_with_no_functions_at_all_is_still_clean() {
        let filter = Filter::Everything;
        let options = patterns_report::Options::default();
        let analyses = analyze(Vec::new(), &filter, &Config::default(), &options).unwrap();
        assert_eq!(analyses.exit_code(true), 0);
    }

    #[test]
    fn findings_decide_the_exit_only_where_they_count() {
        let mut analyses = run(&["patterns"]);
        let candidate = |report: &Value| report["candidates"].as_array().map_or(0, Vec::len);
        let found = candidate(&serde_json::to_value(&analyses.patterns).unwrap());
        assert_eq!(analyses.exit_code(true), u8::from(found > 0));
        analyses.patterns = None;
        assert_eq!(analyses.exit_code(true), 0);
    }
}
