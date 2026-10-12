//! Exhaustive pair evaluation with sequential AND gates and stage traces.
//!
//! Gate order is fixed: validation, projection, nonempty, size ratio,
//! numerical cosine, string mean, final WL score. Comparisons are inclusive
//! and unrounded. Any rejection or ineligibility leaves later stages
//! `not_evaluated`; arithmetic failures become explicit errors and mark the
//! run incomplete instead of silently truncating results.

use crate::config::{CcgraphConfig, Scoring};
use crate::features::{CosineParts, Vector9, cosine_parts, cosine_passes, numerical_features};
use crate::frac::{ArithError, Frac};
use crate::graph::{ImportedFunction, ProjectedGraph};
use crate::interface::{
    is_known_role, is_native_width, is_supported_type, jaro_winkler, serialize_interface,
    string_mean,
};
use crate::projection::{Projection, ProjectionFault, project};
use crate::scoring::{Histogram, RoundScores, cosine_score, overlap_score, score_rounds};
use crate::wl::{diameter, pair_depth, refine, weights};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const PROFILE_ID: &str = "compat.ccgraph.strict";
pub const PROFILE_VERSION: &str = "0.1.0";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub enum StageName {
    Validation,
    Projection,
    Nonempty,
    SizeRatio,
    Numerical,
    Strings,
    WlFinal,
}

impl StageName {
    pub fn token(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::Projection => "projection",
            Self::Nonempty => "nonempty",
            Self::SizeRatio => "size_ratio",
            Self::Numerical => "numerical",
            Self::Strings => "strings",
            Self::WlFinal => "wl_final",
        }
    }

    pub fn all() -> [Self; 7] {
        [
            Self::Validation,
            Self::Projection,
            Self::Nonempty,
            Self::SizeRatio,
            Self::Numerical,
            Self::Strings,
            Self::WlFinal,
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum StageOutcome {
    Passed,
    Rejected { reason: String },
    NotEvaluated,
    Ineligible { reason: String },
    Error { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StageRecord {
    pub stage: StageName,
    pub outcome: StageOutcome,
    pub detail: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FunctionProjection {
    pub projected_nodes: usize,
    pub diameter: usize,
    pub features: Vector9,
    pub intro_para: String,
    pub return_para: String,
    pub name: String,
    pub reference_count: u64,
    pub category_counts: BTreeMap<String, u64>,
    pub retained_nodes: usize,
    pub removed_nodes: usize,
    pub ignored_label_inputs: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FunctionRecord {
    pub key: String,
    pub eligible: bool,
    pub reasons: Vec<String>,
    pub projection: Option<FunctionProjection>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Decision {
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PairRecord {
    pub first: String,
    pub second: String,
    pub stages: Vec<StageRecord>,
    pub decision: Decision,
    pub displayed_score: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RunReport {
    pub profile_id: String,
    pub profile_version: String,
    pub scoring: String,
    pub effective_config: BTreeMap<String, String>,
    pub functions: Vec<FunctionRecord>,
    pub pairs: Vec<PairRecord>,
    pub incomplete: bool,
    pub diagnostics: Vec<String>,
}

struct PreparedFunction {
    imported: ImportedFunction,
    valid: bool,
    reasons: Vec<String>,
    projection: Result<ProjectionData, ProjectionFault>,
}

struct ProjectionData {
    graph: ProjectedGraph,
    record: FunctionProjection,
}

/// Evaluate every unordered distinct function-key pair, deterministically.
///
/// This is the compact stage-fixture path: it checks gate semantics on
/// caller-supplied [`ImportedFunction`] values without establishing
/// extraction eligibility or producer conformance. Production matching must
/// use [`evaluate_strict_documents`](crate::admission::evaluate_strict_documents),
/// which admits validated PDG + CFG documents through the strict adapter.
/// Fixture success here never certifies a producer.
pub fn evaluate_all(functions: &[ImportedFunction], config: &CcgraphConfig) -> RunReport {
    evaluate_with_pair_limit(functions, config, None)
}

/// An explicit execution limit fails the run before enumeration; it never
/// changes matching policy or reports a truncated pair set as complete.
/// Like `evaluate_all`, callers must separately establish production admission.
pub fn evaluate_with_pair_limit(
    functions: &[ImportedFunction],
    config: &CcgraphConfig,
    maximum_pairs: Option<usize>,
) -> RunReport {
    let mut report = new_report(config);
    let duplicates = duplicate_keys(functions);
    flag_duplicates(&mut report, &duplicates);

    let prepared = prepare_all(functions, &duplicates);
    record_functions(&mut report, &prepared);
    if let Err(error) = check_pair_limit(functions.len(), maximum_pairs) {
        return execution_failure(report, error);
    }
    enumerate_pairs(&mut report, &prepared, config);

    report
}

fn execution_failure(mut report: RunReport, error: String) -> RunReport {
    report.incomplete = true;
    report.diagnostics.push(error);

    report
}

fn check_pair_limit(count: usize, maximum: Option<usize>) -> Result<(), String> {
    let pairs = count
        .checked_mul(count.saturating_sub(1))
        .and_then(|n| n.checked_div(2))
        .ok_or("pair-count arithmetic overflow")?;

    if maximum.is_some_and(|limit| pairs > limit) {
        return Err(format!(
            "pair enumeration requires {pairs} pairs, exceeds explicit resource limit {}",
            maximum.unwrap_or(0)
        ));
    }
    Ok(())
}

fn new_report(config: &CcgraphConfig) -> RunReport {
    RunReport {
        profile_id: PROFILE_ID.to_string(),
        profile_version: PROFILE_VERSION.to_string(),
        scoring: scoring_token(config.scoring).to_string(),
        effective_config: describe_config(config),
        functions: Vec::new(),
        pairs: Vec::new(),
        incomplete: false,
        diagnostics: Vec::new(),
    }
}

fn flag_duplicates(report: &mut RunReport, duplicates: &BTreeSet<String>) {
    if !duplicates.is_empty() {
        report.incomplete = true;
        report
            .diagnostics
            .push(format!("duplicate function keys rejected: {duplicates:?}"));
    }
}

fn prepare_all(
    functions: &[ImportedFunction],
    duplicates: &BTreeSet<String>,
) -> BTreeMap<String, PreparedFunction> {
    let mut prepared = BTreeMap::new();
    for func in functions {
        if duplicates.contains(&func.key) {
            continue;
        }
        prepared.insert(func.key.clone(), prepare(func));
    }
    prepared
}

fn record_functions(report: &mut RunReport, prepared: &BTreeMap<String, PreparedFunction>) {
    for (key, item) in prepared {
        report.functions.push(FunctionRecord {
            key: key.clone(),
            eligible: item.valid && item.projection.is_ok(),
            reasons: item.reasons.clone(),
            projection: item
                .projection
                .as_ref()
                .ok()
                .map(|data| data.record.clone()),
        });
    }
}

fn enumerate_pairs(
    report: &mut RunReport,
    prepared: &BTreeMap<String, PreparedFunction>,
    config: &CcgraphConfig,
) {
    let mut keys: Vec<&String> = prepared.keys().collect();
    keys.sort();
    for (i, first) in keys.iter().enumerate() {
        for second in keys.iter().skip(i + 1) {
            let pair = evaluate_pair(
                &prepared[first.as_str()],
                &prepared[second.as_str()],
                config,
            );
            if pair.1 {
                report.incomplete = true;
            }
            report.pairs.push(pair.0);
        }
    }
}

fn scoring_token(scoring: Scoring) -> &'static str {
    match scoring {
        Scoring::SmallerGraphOverlap => "smaller_graph_overlap",
        Scoring::CrossProductCosine => "cross_product_cosine",
    }
}

fn describe_config(config: &CcgraphConfig) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "minimum_size_ratio".to_string(),
            frac_text(config.minimum_size_ratio),
        ),
        (
            "numerical_threshold".to_string(),
            frac_text(config.numerical_threshold),
        ),
        (
            "string_threshold".to_string(),
            frac_text(config.string_threshold),
        ),
        (
            "final_threshold".to_string(),
            frac_text(config.final_threshold),
        ),
        (
            "scoring".to_string(),
            scoring_token(config.scoring).to_string(),
        ),
    ])
}

fn frac_text(value: Frac) -> String {
    format!("{}/{}", value.numerator(), value.denominator())
}

fn duplicate_keys(functions: &[ImportedFunction]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut dupes = BTreeSet::new();
    for func in functions {
        if !seen.insert(func.key.clone()) {
            dupes.insert(func.key.clone());
        }
    }
    dupes
}

fn prepare(func: &ImportedFunction) -> PreparedFunction {
    let mut reasons = Vec::new();
    let valid = validate_imported(func, &mut reasons);
    let projection = if valid {
        project_or_record(func, &mut reasons)
    } else {
        Err(never_projected())
    };
    PreparedFunction {
        imported: func.clone(),
        valid,
        reasons,
        projection,
    }
}

fn never_projected() -> ProjectionFault {
    ProjectionFault::Ineligible {
        reason: "skipped after validation failure".to_string(),
    }
}

fn project_or_record(
    func: &ImportedFunction,
    reasons: &mut Vec<String>,
) -> Result<ProjectionData, ProjectionFault> {
    match project(func) {
        Ok(projection) => Ok(wrap_projection(func, projection)),
        Err(fault) => {
            reasons.push(fault_reason(&fault));
            Err(fault)
        }
    }
}

fn fault_reason(fault: &ProjectionFault) -> String {
    match fault {
        ProjectionFault::Ineligible { reason } => reason.clone(),
    }
}

fn wrap_projection(func: &ImportedFunction, projection: Projection) -> ProjectionData {
    let graph = projection.graph;
    let features = numerical_features(&graph, projection.facts.reference_count);
    let (retained_nodes, removed_nodes) = disposition_counts(&projection.trace);

    let record = FunctionProjection {
        projected_nodes: graph.node_count(),
        diameter: diameter(&graph),
        features,
        intro_para: serialize_interface(&func.inputs),
        return_para: serialize_interface(&func.outputs),
        name: func.name.clone().expect("validated functions have names"),
        reference_count: projection.facts.reference_count,
        category_counts: projection
            .facts
            .category_counts
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect(),
        retained_nodes,
        removed_nodes,
        ignored_label_inputs: "operators, literals, kinds, names, types, ids, spans, outcomes, positions, annotations",
    };
    ProjectionData { graph, record }
}

fn disposition_counts(trace: &crate::projection::ProjectionTrace) -> (usize, usize) {
    trace.nodes.iter().fold(
        (0, 0),
        |(kept, dropped), (_, disposition)| match disposition {
            crate::projection::NodeDisposition::Retained { .. } => (kept + 1, dropped),
            crate::projection::NodeDisposition::Removed { .. } => (kept, dropped + 1),
        },
    )
}

/// Imported-function validation: identifiers, interfaces, reference policy.
fn validate_imported(func: &ImportedFunction, reasons: &mut Vec<String>) -> bool {
    check_name(func, reasons);
    check_interface(func, reasons);
    check_reference(func, reasons);
    reasons.is_empty()
}

fn check_name(func: &ImportedFunction, reasons: &mut Vec<String>) {
    match &func.name {
        Some(name) if !name.is_empty() => {}
        _ => reasons.push("function name must be known and nonempty".to_string()),
    }
}

fn check_interface(func: &ImportedFunction, reasons: &mut Vec<String>) {
    for element in func.inputs.iter().chain(func.outputs.iter()) {
        if !is_known_role(&element.role) {
            reasons.push(format!("unknown interface role {:?}", element.role));
        }
        if !is_supported_type(&element.normalized_type) {
            reasons.push(format!(
                "unsupported normalized type {:?}",
                element.normalized_type
            ));
        }
    }
}

fn check_reference(func: &ImportedFunction, reasons: &mut Vec<String>) {
    if func.reference_count.is_none() {
        reasons.push("reference count must be known".to_string());
    }
}

/// Evaluate one ordered pair; the boolean flags an incomplete run.
fn evaluate_pair(
    first: &PreparedFunction,
    second: &PreparedFunction,
    config: &CcgraphConfig,
) -> (PairRecord, bool) {
    let input = GateInput {
        first,
        second,
        config,
    };
    let gates = [
        gate_validation,
        gate_projection,
        gate_nonempty,
        gate_size_ratio,
        gate_numerical,
        gate_strings,
        gate_wl_final,
    ];
    let mut stages: Vec<StageRecord> = Vec::new();
    let mut incomplete = false;

    for gate in gates {
        let result = gate(&input);
        incomplete = incomplete || result.failed_run;
        stages.push(result.record);
        if result.halt {
            break;
        }
    }
    for stage in StageName::all().iter().skip(stages.len()) {
        stages.push(not_evaluated(*stage));
    }

    (pair_record(&input, stages), incomplete)
}

struct GateInput<'a> {
    first: &'a PreparedFunction,
    second: &'a PreparedFunction,
    config: &'a CcgraphConfig,
}

struct GateResult {
    record: StageRecord,
    halt: bool,
    failed_run: bool,
}

fn gate_record(
    stage: StageName,
    outcome: StageOutcome,
    detail: BTreeMap<String, String>,
) -> GateResult {
    GateResult {
        record: StageRecord {
            stage,
            outcome,
            detail,
        },
        halt: false,
        failed_run: false,
    }
}

fn halted(stage: StageName, outcome: StageOutcome, detail: BTreeMap<String, String>) -> GateResult {
    GateResult {
        record: StageRecord {
            stage,
            outcome,
            detail,
        },
        halt: true,
        failed_run: false,
    }
}

fn failed(stage: StageName, reason: String, detail: BTreeMap<String, String>) -> GateResult {
    GateResult {
        record: StageRecord {
            stage,
            outcome: StageOutcome::Error { reason },
            detail,
        },
        halt: true,
        failed_run: true,
    }
}

fn not_evaluated(stage: StageName) -> StageRecord {
    StageRecord {
        stage,
        outcome: StageOutcome::NotEvaluated,
        detail: BTreeMap::new(),
    }
}

fn pair_record(input: &GateInput, stages: Vec<StageRecord>) -> PairRecord {
    let decision = if stages
        .iter()
        .all(|s| matches!(s.outcome, StageOutcome::Passed))
    {
        Decision::Accepted
    } else {
        Decision::Rejected
    };
    let displayed_score = stages
        .iter()
        .rev()
        .find_map(|s| s.detail.get("displayed_score").cloned());
    PairRecord {
        first: input.first.imported.key.clone(),
        second: input.second.imported.key.clone(),
        stages,
        decision,
        displayed_score,
    }
}

fn gate_validation(input: &GateInput) -> GateResult {
    let mut detail = BTreeMap::new();
    let pair_ok = widths_compatible(&input.first.imported, &input.second.imported, &mut detail);
    let ok = input.first.valid && input.second.valid && pair_ok;

    if ok {
        return gate_record(StageName::Validation, StageOutcome::Passed, detail);
    }
    halted(
        StageName::Validation,
        validation_failure(input, &detail),
        detail,
    )
}

fn validation_failure(input: &GateInput, detail: &BTreeMap<String, String>) -> StageOutcome {
    let mut reasons: Vec<String> = input
        .first
        .reasons
        .iter()
        .chain(input.second.reasons.iter())
        .cloned()
        .collect();
    if let Some(note) = detail.get("widths") {
        reasons.push(note.clone());
    }
    StageOutcome::Ineligible {
        reason: reasons.join("; "),
    }
}

fn gate_projection(input: &GateInput) -> GateResult {
    match (&input.first.projection, &input.second.projection) {
        (Ok(_), Ok(_)) => gate_record(
            StageName::Projection,
            StageOutcome::Passed,
            projection_detail(input.first, input.second),
        ),
        _ => halted(
            StageName::Projection,
            StageOutcome::Ineligible {
                reason: projection_failure(input.first, input.second),
            },
            BTreeMap::new(),
        ),
    }
}

fn gate_nonempty(input: &GateInput) -> GateResult {
    let (a, b) = pair_sizes(input);
    let detail = BTreeMap::from([("sizes".to_string(), format!("{a},{b}"))]);

    if a > 0 && b > 0 {
        return gate_record(StageName::Nonempty, StageOutcome::Passed, detail);
    }
    halted(
        StageName::Nonempty,
        StageOutcome::Rejected {
            reason: "empty projection".to_string(),
        },
        detail,
    )
}

fn pair_sizes(input: &GateInput) -> (usize, usize) {
    (
        graph_of(input.first).node_count(),
        graph_of(input.second).node_count(),
    )
}

fn gate_size_ratio(input: &GateInput) -> GateResult {
    let (a, b) = pair_sizes(input);
    let (smaller, larger) = (a.min(b) as u64, a.max(b) as u64);
    let threshold = input.config.minimum_size_ratio;
    let detail = BTreeMap::from([
        ("ratio".to_string(), format!("{smaller}/{larger}")),
        ("threshold".to_string(), frac_text(threshold)),
    ]);

    match Frac::new(smaller as i128, larger as i128) {
        Err(_) => failed(
            StageName::SizeRatio,
            "size ratio failed".to_string(),
            detail,
        ),
        Ok(ratio) => judge_size_ratio(ratio, threshold, detail),
    }
}

fn judge_size_ratio(ratio: Frac, threshold: Frac, detail: BTreeMap<String, String>) -> GateResult {
    let accepted = match ratio.ge(threshold) {
        Ok(accepted) => accepted,
        Err(error) => {
            return failed(StageName::SizeRatio, error.what.to_string(), detail);
        }
    };

    if accepted {
        return gate_record(StageName::SizeRatio, StageOutcome::Passed, detail);
    }
    halted(
        StageName::SizeRatio,
        StageOutcome::Rejected {
            reason: format!(
                "size ratio {} below {}",
                frac_text(ratio),
                frac_text(threshold)
            ),
        },
        detail,
    )
}

fn gate_numerical(input: &GateInput) -> GateResult {
    let parts = match cosine_parts(
        &record_of(input.first).features,
        &record_of(input.second).features,
    ) {
        Ok(parts) => parts,
        Err(error) => {
            return failed(
                StageName::Numerical,
                error.what.to_string(),
                BTreeMap::new(),
            );
        }
    };
    let mut detail = cosine_detail(&parts);
    detail.insert(
        "threshold".to_string(),
        frac_text(input.config.numerical_threshold),
    );

    match cosine_passes(parts, input.config.numerical_threshold) {
        Ok(true) => gate_record(StageName::Numerical, StageOutcome::Passed, detail),
        Ok(false) => halted(
            StageName::Numerical,
            StageOutcome::Rejected {
                reason: "numerical cosine below threshold".to_string(),
            },
            detail,
        ),
        Err(error) => failed(StageName::Numerical, error.what.to_string(), detail),
    }
}

fn gate_strings(input: &GateInput) -> GateResult {
    let scores = match string_scores(input.first, input.second) {
        Ok(scores) => scores,
        Err(error) => return failed(StageName::Strings, error.what.to_string(), BTreeMap::new()),
    };
    let mut detail = scores.detail();
    detail.insert(
        "threshold".to_string(),
        frac_text(input.config.string_threshold),
    );

    match scores.mean.ge(input.config.string_threshold) {
        Ok(true) => gate_record(StageName::Strings, StageOutcome::Passed, detail),
        Ok(false) => halted(
            StageName::Strings,
            StageOutcome::Rejected {
                reason: "string mean below threshold".to_string(),
            },
            detail,
        ),
        Err(error) => failed(StageName::Strings, error.what.to_string(), detail),
    }
}

fn gate_wl_final(input: &GateInput) -> GateResult {
    match wl_scores(input.first, input.second, input.config) {
        Ok(outcome) => GateResult {
            record: StageRecord {
                stage: StageName::WlFinal,
                outcome: outcome.outcome,
                detail: outcome.detail,
            },
            halt: false,
            failed_run: false,
        },
        Err(error) => failed(StageName::WlFinal, error.what.to_string(), BTreeMap::new()),
    }
}

fn graph_of(prepared: &PreparedFunction) -> &ProjectedGraph {
    match &prepared.projection {
        Ok(data) => &data.graph,
        Err(_) => panic_projection_checked(),
    }
}

fn panic_projection_checked() -> ! {
    panic!("projection checked before use")
}

fn record_of(prepared: &PreparedFunction) -> &FunctionProjection {
    match &prepared.projection {
        Ok(data) => &data.record,
        Err(_) => panic_projection_checked(),
    }
}

/// Native-width interfaces or operations need equal known pointer widths.
fn widths_compatible(
    first: &ImportedFunction,
    second: &ImportedFunction,
    detail: &mut BTreeMap<String, String>,
) -> bool {
    if !native_involved(first) && !native_involved(second) {
        return true;
    }
    detail.insert(
        "widths".to_string(),
        format!(
            "{:?},{:?}",
            first.target_pointer_width, second.target_pointer_width
        ),
    );
    match (first.target_pointer_width, second.target_pointer_width) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn native_involved(func: &ImportedFunction) -> bool {
    func.native_width_involved
        || func
            .inputs
            .iter()
            .chain(func.outputs.iter())
            .any(|e| is_native_width(&e.normalized_type))
}

fn projection_detail(
    first: &PreparedFunction,
    second: &PreparedFunction,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "first_nodes".to_string(),
            record_of(first).projected_nodes.to_string(),
        ),
        (
            "second_nodes".to_string(),
            record_of(second).projected_nodes.to_string(),
        ),
    ])
}

fn projection_failure(first: &PreparedFunction, second: &PreparedFunction) -> String {
    let mut reasons = Vec::new();
    for prepared in [first, second] {
        if let Err(fault) = &prepared.projection {
            reasons.push(fault_reason(fault));
        }
    }
    reasons.join("; ")
}

fn cosine_detail(parts: &CosineParts) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("dot".to_string(), parts.dot.to_string()),
        (
            "norm_a_squared".to_string(),
            parts.norm_a_squared.to_string(),
        ),
        (
            "norm_b_squared".to_string(),
            parts.norm_b_squared.to_string(),
        ),
    ])
}

struct StringScores {
    return_para: Frac,
    intro_para: Frac,
    name: Frac,
    mean: Frac,
    return_text: (String, String),
    intro_text: (String, String),
    name_text: (String, String),
}

impl StringScores {
    fn detail(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("return_para".to_string(), frac_text(self.return_para)),
            ("intro_para".to_string(), frac_text(self.intro_para)),
            ("name".to_string(), frac_text(self.name)),
            ("mean".to_string(), frac_text(self.mean)),
            (
                "return_texts".to_string(),
                format!("{:?}|{:?}", self.return_text.0, self.return_text.1),
            ),
            (
                "intro_texts".to_string(),
                format!("{:?}|{:?}", self.intro_text.0, self.intro_text.1),
            ),
            (
                "name_texts".to_string(),
                format!("{:?}|{:?}", self.name_text.0, self.name_text.1),
            ),
        ])
    }
}

fn string_scores(
    first: &PreparedFunction,
    second: &PreparedFunction,
) -> Result<StringScores, ArithError> {
    let a = record_of(first);
    let b = record_of(second);
    let return_para = jaro_winkler(&a.return_para, &b.return_para)?;
    let intro_para = jaro_winkler(&a.intro_para, &b.intro_para)?;

    let name = jaro_winkler(&a.name, &b.name)?;
    let mean = string_mean(return_para, intro_para, name)?;

    Ok(StringScores {
        return_para,
        intro_para,
        name,
        mean,
        return_text: (a.return_para.clone(), b.return_para.clone()),
        intro_text: (a.intro_para.clone(), b.intro_para.clone()),
        name_text: (a.name.clone(), b.name.clone()),
    })
}

struct WlOutcome {
    outcome: StageOutcome,
    detail: BTreeMap<String, String>,
}

fn wl_scores(
    first: &PreparedFunction,
    second: &PreparedFunction,
    config: &CcgraphConfig,
) -> Result<WlOutcome, ArithError> {
    let graph_a = graph_of(first);
    let graph_b = graph_of(second);
    let record_a = record_of(first);
    let record_b = record_of(second);
    let h = pair_depth(
        record_a.projected_nodes,
        record_a.diameter,
        record_b.projected_nodes,
        record_b.diameter,
    );

    let selected_weights = weights(h);
    let refinement = refine(graph_a, graph_b, h);
    let rounds: Vec<(Histogram, Histogram)> =
        refinement.rounds.iter().map(label_histograms).collect();
    let scores = score_rounds(&rounds, &selected_weights)?;

    let mut detail = wl_detail(h, &selected_weights, &scores, &rounds);
    Ok(match config.scoring {
        Scoring::SmallerGraphOverlap => judge_overlap(
            &scores,
            (record_a.projected_nodes, record_b.projected_nodes),
            config,
            &mut detail,
        )?,
        Scoring::CrossProductCosine => judge_cosine(&scores, config, &mut detail)?,
    })
}

fn label_histograms(round: &(BTreeMap<u64, u64>, BTreeMap<u64, u64>)) -> (Histogram, Histogram) {
    (rename(&round.0), rename(&round.1))
}

fn rename(hist: &BTreeMap<u64, u64>) -> Histogram {
    hist.iter()
        .map(|(label, count)| (format!("L{label}"), *count))
        .collect()
}

fn wl_detail(
    h: usize,
    selected_weights: &[u64],
    scores: &RoundScores,
    rounds: &[(Histogram, Histogram)],
) -> BTreeMap<String, String> {
    let mut detail = BTreeMap::from([
        ("depth".to_string(), h.to_string()),
        ("weights".to_string(), format!("{selected_weights:?}")),
        (
            "overlap_per_round".to_string(),
            format!("{:?}", scores.overlap_per_round),
        ),
        (
            "cross_per_round".to_string(),
            format!("{:?}", scores.cross_per_round),
        ),
        (
            "self_a_per_round".to_string(),
            format!("{:?}", scores.self_a_per_round),
        ),
        (
            "self_b_per_round".to_string(),
            format!("{:?}", scores.self_b_per_round),
        ),
    ]);
    for (index, (hist_a, hist_b)) in rounds.iter().enumerate() {
        detail.insert(format!("round_{index}_a"), format!("{hist_a:?}"));
        detail.insert(format!("round_{index}_b"), format!("{hist_b:?}"));
    }
    detail
}

fn judge_overlap(
    scores: &RoundScores,
    sizes: (usize, usize),
    config: &CcgraphConfig,
    detail: &mut BTreeMap<String, String>,
) -> Result<WlOutcome, ArithError> {
    let smaller = sizes.0.min(sizes.1) as u64;
    let larger = sizes.0.max(sizes.1) as u64;
    let scored = overlap_score(scores, smaller, larger)?;
    detail.extend(overlap_entries(&scored, config));

    let outcome = if scored.score.ge(config.final_threshold)? {
        StageOutcome::Passed
    } else {
        StageOutcome::Rejected {
            reason: "final WL score below threshold".to_string(),
        }
    };
    Ok(WlOutcome {
        outcome,
        detail: detail.clone(),
    })
}

fn overlap_entries(
    scored: &super::scoring::OverlapDetail,
    config: &CcgraphConfig,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("score".to_string(), frac_text(scored.score)),
        (
            "larger_coverage".to_string(),
            frac_text(scored.larger_coverage),
        ),
        ("threshold".to_string(), frac_text(config.final_threshold)),
        (
            "displayed_score".to_string(),
            format!("{}", scored.score.to_f64()),
        ),
    ])
}

fn judge_cosine(
    scores: &RoundScores,
    config: &CcgraphConfig,
    detail: &mut BTreeMap<String, String>,
) -> Result<WlOutcome, ArithError> {
    let scored = cosine_score(scores)?;
    detail.extend(cosine_entries(&scored, config));
    let outcome = if super::scoring::cosine_passes(&scored, config.final_threshold)? {
        StageOutcome::Passed
    } else {
        StageOutcome::Rejected {
            reason: "final WL score below threshold".to_string(),
        }
    };
    Ok(WlOutcome {
        outcome,
        detail: detail.clone(),
    })
}

fn cosine_entries(
    scored: &super::scoring::CosineDetail,
    config: &CcgraphConfig,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("kab".to_string(), scored.kab.to_string()),
        ("kaa".to_string(), scored.kaa.to_string()),
        ("kbb".to_string(), scored.kbb.to_string()),
        ("threshold".to_string(), frac_text(config.final_threshold)),
        (
            "displayed_score".to_string(),
            format!("{}", cosine_display(scored)),
        ),
    ])
}

fn cosine_display(detail: &super::scoring::CosineDetail) -> f64 {
    let numerator = detail.kab as f64;
    let denominator = (detail.kaa as f64 * detail.kbb as f64).sqrt();
    if denominator == 0.0 {
        f64::NAN
    } else {
        numerator / denominator
    }
}
