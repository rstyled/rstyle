//! SARIF 2.1.0: one log and one run per analysis, so the results of different analyses
//! never interleave, and one log holding every run as a run's final report.
//!
//! Findings and candidates are what SARIF carries. A check's findings also carry why: each
//! decision point is a related location, and the measured value and its limit are properties.
//! A check run carries its fix groups, so a consumer holding only the SARIF can plan fixes.
//! The JSON report stays the complete one: runtime bounds and signature groups aren't here.

use crate::candidates::Candidate;
use crate::defects_report::{DefectRow, DefectsReport};
use crate::patterns_report::PatternsReport;
use crate::report::{Detail, Diagnostic, FixGroup, Report};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{MAIN_SEPARATOR, Path};

const SCHEMA: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json";
const VERSION: &str = "2.1.0";
const SOURCE_ROOT: &str = "%SRCROOT%";
const CANDIDATE_FINGERPRINT: &str = "rstyle/candidate/v1";
/// Categories keep the two analyses apart once a consumer holds both.
pub const CHECK: &str = "rstyle/check/";
pub const PATTERNS: &str = "rstyle/patterns/";
pub const DEFECTS: &str = "rstyle/defects/";

#[derive(Debug, Serialize)]
pub struct Log {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<Run>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Run {
    tool: Tool,
    automation_details: Automation,
    original_uri_base_ids: BTreeMap<&'static str, Artifact>,
    results: Vec<Finding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<RunProperties>,
}

/// Beside, not under, the `rstyle` key: a service stamps `properties.rstyle` with its own record.
#[derive(Debug, Serialize)]
struct RunProperties {
    fix_groups: Vec<FixGroup>,
}

#[derive(Debug, Serialize)]
struct Tool {
    driver: Driver,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Driver {
    name: &'static str,
    information_uri: &'static str,
}

#[derive(Debug, Serialize)]
struct Automation {
    id: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Finding {
    rule_id: String,
    level: &'static str,
    message: Message,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    locations: Vec<Location>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    related_locations: Vec<Location>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fingerprints: BTreeMap<&'static str, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<Measure>,
}

/// What a check measured against its limit, and the fix group that fixes it.
#[derive(Debug, Serialize)]
struct Measure {
    actual: usize,
    limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    fix_group: Option<usize>,
}

#[derive(Debug, Serialize)]
struct Message {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Location {
    physical_location: PhysicalLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<Message>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PhysicalLocation {
    artifact_location: Artifact,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Artifact {
    uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    uri_base_id: Option<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Region {
    start_line: usize,
}

/// One warning per diagnostic, with its rule id and message. `root` is what relative paths
/// resolve against. Columns are left out: SARIF counts them in other units than a frontend.
pub fn check(report: &Report, root: &Path) -> Result<Log, String> {
    let findings = report
        .diagnostics
        .iter()
        .map(|d| diagnostic(d, group_of(d, &report.fix_groups)))
        .collect();
    let mut log = log(findings, CHECK, root)?;
    log.runs[0].properties = (!report.fix_groups.is_empty()).then(|| RunProperties {
        fix_groups: report.fix_groups.clone(),
    });
    Ok(log)
}

/// The fix group whose function is the diagnostic's, for the diagnostic's rule.
fn group_of(diagnostic: &Diagnostic, groups: &[FixGroup]) -> Option<usize> {
    groups
        .iter()
        .find(|group| {
            group.functions.iter().any(|function| {
                function.path == diagnostic.path
                    && function.line == diagnostic.line
                    && function.rules.contains(&diagnostic.rule_id)
            })
        })
        .map(|group| group.id)
}

/// One warning per candidate left in the report, with its kind as the rule id.
pub fn patterns(report: &PatternsReport, root: &Path) -> Result<Log, String> {
    log(
        report.candidates.iter().map(candidate).collect(),
        PATTERNS,
        root,
    )
}

/// One warning per defect, with its rule as the rule id.
pub fn defects(report: &DefectsReport, root: &Path) -> Result<Log, String> {
    log(report.defects.iter().map(defect).collect(), DEFECTS, root)
}

/// Whether a frontend's file is a log a report can later hold: right version, and every run
/// names its tool. The contents remain the frontend's to answer for.
pub fn validate(log: &Value) -> Result<(), String> {
    if log["version"] != VERSION {
        return Err(format!("SARIF version must be {VERSION}"));
    }
    let runs = log["runs"]
        .as_array()
        .filter(|runs| !runs.is_empty())
        .ok_or("A SARIF log needs at least one run")?;
    runs.iter().try_for_each(valid_run)
}

/// A frontend's runs as a final report holds them: each stays distinguishable from every other
/// analysis and has a root for its relative paths, unless it says otherwise. `category` is the
/// analysis's own, such as `go/calls/`.
pub fn labeled_runs(log: &Value, category: &str, root: &Path) -> Result<Vec<Value>, String> {
    let runs = log["runs"].as_array().ok_or("A SARIF log needs runs")?;
    let root = serde_json::to_value(root_artifact(root)?).map_err(|error| error.to_string())?;
    let several = runs.len() > 1;

    runs.iter()
        .enumerate()
        .map(|(index, run)| {
            let id = match several {
                true => format!("{category}{index}/"),
                false => category.to_string(),
            };
            labeled_run(run.clone(), &id, &root)
        })
        .collect()
}

fn labeled_run(mut run: Value, id: &str, root: &Value) -> Result<Value, String> {
    let fields = run.as_object_mut().ok_or("A SARIF run must be an object")?;
    fields
        .entry("automationDetails")
        .or_insert_with(|| serde_json::json!({"id": id}));
    fields
        .entry("originalUriBaseIds")
        .or_insert_with(|| serde_json::json!({SOURCE_ROOT: root}));
    Ok(run)
}

/// One log holding these runs, in order: a run's final report.
pub fn combined(runs: Vec<Value>) -> Value {
    serde_json::json!({"$schema": SCHEMA, "version": VERSION, "runs": runs})
}

fn valid_run(run: &Value) -> Result<(), String> {
    if !run["tool"]["driver"]["name"].is_string() {
        return Err("Every SARIF run must name its tool".into());
    }
    if !(run["results"].is_null() || run["results"].is_array()) {
        return Err("SARIF results must be an array".into());
    }
    Ok(())
}

fn log(results: Vec<Finding>, category: &'static str, root: &Path) -> Result<Log, String> {
    Ok(Log {
        schema: SCHEMA,
        version: VERSION,
        runs: vec![Run {
            tool: Tool {
                driver: Driver {
                    name: "rstyle",
                    information_uri: "https://github.com/rstyled/rstyle",
                },
            },
            automation_details: Automation { id: category },
            original_uri_base_ids: BTreeMap::from([(SOURCE_ROOT, root_artifact(root)?)]),
            results,
            properties: None,
        }],
    })
}

fn root_artifact(root: &Path) -> Result<Artifact, String> {
    let path = root
        .to_str()
        .ok_or_else(|| format!("{} is not valid UTF-8", root.display()))?;
    if !root.is_absolute() {
        return Err(format!("The SARIF source root {path} must be absolute"));
    }
    Ok(Artifact {
        uri: format!("{}/", file_uri(path).trim_end_matches('/')),
        uri_base_id: None,
    })
}

/// One warning at the function, with each decision point that raised it as a related location.
fn diagnostic(diagnostic: &Diagnostic, fix_group: Option<usize>) -> Finding {
    let mut finding = at_one_place(
        &diagnostic.rule_id,
        &diagnostic.message,
        (&diagnostic.path, diagnostic.line),
    );
    finding.related_locations = diagnostic
        .details
        .iter()
        .flat_map(|detail| decision_points(&diagnostic.path, detail))
        .collect();
    finding.properties = Some(Measure {
        actual: diagnostic.actual,
        limit: diagnostic.limit,
        fix_group,
    });
    finding
}

/// A detail at one line says how much it added there; at several lines, each line is one of them.
fn decision_points<'a>(path: &'a str, detail: &'a Detail) -> impl Iterator<Item = Location> + 'a {
    let text = match detail.lines.len() {
        1 => format!("{} +{}", detail.label, detail.count),
        _ => detail.label.clone(),
    };
    detail.lines.iter().filter_map(move |line| {
        let mut location = location(path, *line)?;
        location.message = Some(Message { text: text.clone() });
        Some(location)
    })
}

fn defect(defect: &DefectRow) -> Finding {
    at_one_place(defect.rule, &defect.message, (&defect.path, defect.line))
}

fn at_one_place(rule_id: &str, text: &str, (path, line): (&str, usize)) -> Finding {
    Finding {
        rule_id: rule_id.to_owned(),
        level: "warning",
        message: Message {
            text: text.to_owned(),
        },
        locations: location(path, line).into_iter().collect(),
        related_locations: Vec::new(),
        fingerprints: BTreeMap::new(),
        properties: None,
    }
}

/// The first site is primary because code scanning shows only one; the id is what
/// `cargo rstyle ignore` takes.
fn candidate(candidate: &Candidate) -> Finding {
    let mut sites = candidate
        .sites
        .iter()
        .filter_map(|site| location(&site.path, site.line));
    let locations = sites.next().into_iter().collect();
    Finding {
        rule_id: candidate.kind.name().to_string(),
        level: "warning",
        message: Message {
            text: [
                candidate.observation.as_str(),
                candidate.inference.as_str(),
                candidate.possible_refactor.as_str(),
            ]
            .join("\n"),
        },
        locations,
        related_locations: sites.collect(),
        fingerprints: BTreeMap::from([(CANDIDATE_FINGERPRINT, candidate.id.clone())]),
        properties: None,
    }
}

/// A path says where a finding is; an unknown line is left out instead of invented.
fn location(path: &str, line: usize) -> Option<Location> {
    (!path.is_empty()).then(|| Location {
        physical_location: PhysicalLocation {
            artifact_location: artifact(path),
            region: (line > 0).then_some(Region { start_line: line }),
        },
        message: None,
    })
}

fn artifact(path: &str) -> Artifact {
    match Path::new(path).is_absolute() {
        true => Artifact {
            uri: file_uri(path),
            uri_base_id: None,
        },
        false => Artifact {
            uri: encoded(path),
            uri_base_id: Some(SOURCE_ROOT),
        },
    }
}

fn file_uri(path: &str) -> String {
    // A canonical Windows path carries a verbatim prefix no URI consumer expects.
    let encoded = encoded(path.strip_prefix(r"\\?\").unwrap_or(path));
    match encoded.starts_with('/') {
        true => format!("file://{encoded}"),
        false => format!("file:///{encoded}"),
    }
}

/// Percent-encoded with `/` separators, so no segment can read as a URI scheme.
fn encoded(path: &str) -> String {
    let mut uri = String::with_capacity(path.len());
    for byte in path.replace(MAIN_SEPARATOR, "/").bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                uri.push(char::from(byte));
            }
            _ => uri.push_str(&format!("%{byte:02X}")),
        }
    }
    uri
}

#[cfg(test)]
mod tests;
