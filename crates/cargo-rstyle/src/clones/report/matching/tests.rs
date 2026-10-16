use super::*;
use serde_json::Value;

fn matching_report(report: &RunReport, trace: bool) -> Result<Value, String> {
    let mut value = serde_json::to_value(report).map_err(|e| e.to_string())?;
    let pairs = value["pairs"]
        .as_array_mut()
        .ok_or("missing serialized pair list")?;
    for pair in pairs {
        format_pair(pair, trace)?;
    }

    Ok(value)
}

fn format_pair(pair: &mut Value, trace: bool) -> Result<(), String> {
    let stages = pair["stages"].as_array_mut().ok_or("missing stages")?;
    let status = evaluation_status(stages);
    if !trace {
        for stage in stages {
            strip_histograms(stage);
        }
    }

    pair["evaluation_status"] = Value::String(status.into());
    if status == "failed" || status == "incompatible" {
        pair["decision"] = Value::Null;
    }
    Ok(())
}

fn evaluation_status(stages: &[Value]) -> &'static str {
    if stages.iter().any(|s| s["outcome"].get("Error").is_some()) {
        return "failed";
    }
    if stages
        .iter()
        .any(|s| s["outcome"].get("Ineligible").is_some())
    {
        return "incompatible";
    }
    "evaluated"
}

fn strip_histograms(stage: &mut Value) {
    if let Some(detail) = stage["detail"].as_object_mut() {
        detail.retain(|key, _| !key.starts_with("round_"));
    }
}

fn outcome_matrix() -> RunReport {
    use rstyle_ccgraph::StageName;
    let outcomes = [
        StageOutcome::Passed,
        StageOutcome::Rejected {
            reason: "different λ".into(),
        },
        StageOutcome::NotEvaluated,
        StageOutcome::Ineligible {
            reason: "unknown".into(),
        },
        StageOutcome::Error {
            reason: "invalid".into(),
        },
    ];
    let mut report = RunReport {
        profile_id: "profile".into(),
        profile_version: "1".into(),
        scoring: "overlap".into(),
        effective_config: BTreeMap::new(),
        functions: vec![],
        pairs: vec![],
        incomplete: true,
        diagnostics: vec!["retained diagnostic".into()],
    };
    for first in &outcomes {
        for second in &outcomes {
            report.pairs.push(PairRecord {
                first: "caller λ".into(),
                second: "callee".into(),
                stages: [first, second]
                    .into_iter()
                    .map(|outcome| StageRecord {
                        stage: StageName::WlFinal,
                        outcome: outcome.clone(),
                        detail: BTreeMap::from([
                            ("round_0".into(), "histogram".into()),
                            ("round_custom".into(), "future histogram".into()),
                            ("unknown".into(), "keep λ".into()),
                            ("depth".into(), "2".into()),
                        ]),
                    })
                    .collect(),
                decision: Decision::Accepted,
                displayed_score: Some("1/2".into()),
            });
        }
    }
    report
}

#[test]
fn borrowed_matching_export_preserves_legacy_json_meaning() {
    let report = outcome_matrix();
    for trace in [false, true] {
        let mut expected = matching_report(&report, trace).unwrap();
        let mut view = Matching {
            report: &report,
            trace,
            partitions: BTreeMap::new(),
        };
        assert_eq!(serde_json::to_value(&view).unwrap(), expected);
        let partitions = vec![rstyle_ccgraph::wl::LabelPartition {
            first: BTreeMap::from([("λ".into(), 2)]),
            second: BTreeMap::new(),
        }];
        expected["pairs"][0]["wl_label_partitions"] = serde_json::to_value(&partitions).unwrap();
        view.partitions.insert(0, partitions);
        assert_eq!(serde_json::to_value(&view).unwrap(), expected);
    }
}
