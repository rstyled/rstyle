//! Strict replay and selected identity matching for the complete original Line methods.
use rstyle_pdg::{producer::Pair, unit_store::UnitStore, verification_policy};
use serde_json::{Value, json};
#[path = "line_tail_activation/reader.rs"]
mod reader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(1..=2).contains(&args.len()) {
        return Err("Expected report.json and optional selection.json".into());
    }
    let report: Value = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let selection: Value = if let Some(path) = args.get(1) {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        serde_json::from_str(include_str!(
            "../../../eval/coverage-development/line-tail/selection.json"
        ))?
    };
    run(&report, &selection)?;
    Ok(())
}

struct Replay<'a> {
    store: UnitStore,
    raw: Value,
    roster: Vec<&'a Value>,
    selection: &'a Value,
}

fn run(report: &Value, selection: &Value) -> Result<(), String> {
    let replay = Replay::load(report, selection)?;
    replay.check_roster()?;
    let attacks = replay.check_selected()?;
    let selected = selection["functions"]
        .as_array()
        .ok_or("Missing selection")?
        .len();
    let build = &replay.roster[0]["facts"]["build"];
    println!(
        "{}",
        json!({"original_roster":replay.roster.len(), "selected_denominator":selected,
        "strictly_admitted":selected, "identity_accepted":selected, "resealed_attacks_rejected":attacks,
        "target":build["target_triple"], "features":build["features"], "strict_import":true,
        "full_library_matching_claimed":false})
    );
    Ok(())
}

impl<'a> Replay<'a> {
    fn load(report: &'a Value, selection: &'a Value) -> Result<Self, String> {
        if report["capture_complete"] != true || report["assessment_complete"] != true {
            return Err("Original library capture and assessment must be complete".into());
        }
        let selected = selection["functions"]
            .as_array()
            .ok_or("Missing selection")?;
        let names: Vec<_> = selected
            .iter()
            .map(|function| function["name"].as_str())
            .collect();
        let expected = match selection["identity"].as_str() {
            Some("rstyle.development.base64ct-line-tail/1") => vec![
                Some("decoder::Line::<'i>::slice_tail"),
                Some("decoder::Line::<'i>::len"),
            ],
            Some("rstyle.development.base64ct-line-reader/1") => {
                vec![Some("decoder::LineReader::<'i>::new_unwrapped")]
            }
            _ => return Err("Unregistered original family selection".into()),
        };
        if names != expected {
            return Err("Original family selection identities/denominator changed".into());
        }
        let records = report["functions"]
            .as_array()
            .ok_or("Missing original roster")?;
        let first = records
            .iter()
            .find(|f| f["facts"]["name"] == selected[0]["name"])
            .ok_or("Missing selected original")?;
        let path = first["unit_evidence"]["bundle"]
            .as_str()
            .ok_or("Missing emitted evidence bundle")?;
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let raw: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        check_sources(&raw, selection)?;
        let store = UnitStore::from_bundle(&bytes)?;
        let roster = records
            .iter()
            .filter(|f| f["unit_evidence"]["bundle"] == path)
            .collect();
        Ok(Self {
            store,
            raw,
            roster,
            selection,
        })
    }

    fn check_roster(&self) -> Result<(), String> {
        if self.roster.len() != self.store.functions().len() {
            return Err("Report/bundle complete original roster changed".into());
        }
        for (ordinal, fact) in self.store.functions().iter().enumerate() {
            let record = self
                .roster
                .iter()
                .find(|r| r["unit_evidence"]["function"] == ordinal)
                .ok_or("Missing original ordinal")?;
            if record["unit_evidence"]["namespace"] != self.raw["namespace"]
                || record["facts"] != serde_json::to_value(fact).map_err(|e| e.to_string())?
            {
                return Err(format!(
                    "Original source/build/roster correspondence changed at {ordinal}"
                ));
            }
        }
        Ok(())
    }

    fn check_selected(&self) -> Result<usize, String> {
        let mut attacks = 0;
        for expected in self.selection["functions"]
            .as_array()
            .ok_or("Missing selection")?
        {
            let matches: Vec<_> = self
                .roster
                .iter()
                .filter(|r| r["facts"]["name"] == expected["name"])
                .collect();
            let [record] = matches.as_slice() else {
                return Err("Selected original is not unique".into());
            };
            attacks += self.check_method(record, expected)?;
        }
        Ok(attacks)
    }

    fn check_method(&self, record: &Value, expected: &Value) -> Result<usize, String> {
        let ordinal = record["unit_evidence"]["function"]
            .as_u64()
            .ok_or("Missing ordinal")? as usize;
        if record["outcome"] != "admitted"
            || record["facts"]["fn_span"] != expected["span"]
            || record["facts"]["source"]["bytes"] != expected["source"]
        {
            return Err("Selected complete original name/span/bytes/admission changed".into());
        }
        check_build(
            &serde_json::to_value(&self.store.functions()[ordinal].build)
                .map_err(|e| e.to_string())?,
            self.selection,
        )?;
        let analysis = self.store.analysis();
        let view = analysis.function(ordinal).ok_or("Missing original view")?;
        let pair = self
            .store
            .pair(ordinal)
            .ok_or("Missing complete original graph")?;
        verification_policy::strict(|| rstyle_pdg::producer::verify(pair, &view))
            .map_err(|e| e.join("; "))?;
        identity_matching(pair, &view)?;
        if expected["name"] == "decoder::LineReader::<'i>::new_unwrapped" {
            return reader::check(pair, &view);
        }
        if expected["name"]
            .as_str()
            .is_some_and(|name| name.ends_with("::slice_tail"))
        {
            expected_origins(pair, &record["facts"])?;
            return semantic_attacks(pair, &view);
        }
        Ok(0)
    }
}

fn identity_matching(
    pair: &Pair,
    view: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let mut first = verification_policy::strict(|| rstyle_ccgraph::admit_produced(pair, view))
        .map_err(|e| e.join("; "))?;
    let mut second = verification_policy::strict(|| rstyle_ccgraph::admit_produced(pair, view))
        .map_err(|e| e.join("; "))?;
    if first.matching_profile != rstyle_ccgraph::MatchingProfile::DataCallsV1 {
        return Err("Selected method lost DataCalls matching profile".into());
    }
    first.key = "selected-original".into();
    second.key = "strict-replay".into();
    let matching = rstyle_ccgraph::evaluate_all(&[first, second], &Default::default());
    if matching.incomplete
        || matching.pairs.len() != 1
        || matching.pairs[0].decision != rstyle_ccgraph::Decision::Accepted
    {
        return Err(format!("Selected identity matching failed: {matching:?}"));
    }
    Ok(())
}

fn semantic_attacks(
    pair: &Pair,
    view: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) -> Result<usize, String> {
    let attacks = [
        "suffix",
        "operand",
        "saved-offset",
        "guard",
        "error",
        "origin",
        "summary",
        "policy",
    ];
    verify_attacks(pair, view, &attacks, forge)
}

fn verify_attacks(
    pair: &Pair,
    view: &dyn rstyle_pdg::unit_store::EvidenceFacts,
    attacks: &[&str],
    forge: impl Fn(&Pair, &str) -> Result<Pair, String>,
) -> Result<usize, String> {
    for attack in attacks {
        let forged = forge(pair, attack)?;
        let sealed = forged.serialize().map_err(|e| e.to_string())?;
        if verification_policy::strict(|| rstyle_pdg::producer::verify_serialized(&sealed, view))
            .is_ok()
        {
            return Err(format!("Strict verifier accepted resealed {attack} attack"));
        }
    }
    Ok(attacks.len())
}

fn check_sources(bundle: &Value, selection: &Value) -> Result<(), String> {
    let sources = bundle["sources"]
        .as_array()
        .ok_or("Missing original file snapshots")?;
    for obligation in selection["source_obligations"]
        .as_array()
        .ok_or("Missing source obligations")?
        .iter()
        .chain(selection["owners"].as_array().into_iter().flatten())
    {
        let start = obligation["span"]["start"]
            .as_u64()
            .ok_or("Missing source start")? as usize;
        let end = obligation["span"]["end"]
            .as_u64()
            .ok_or("Missing source end")? as usize;
        let expected = obligation["source"]
            .as_str()
            .ok_or("Missing original source")?;
        if !sources
            .iter()
            .any(|s| s.as_str().and_then(|s| s.get(start..end)) == Some(expected))
        {
            return Err(format!(
                "Complete original source obligation absent: {}",
                obligation["name"]
            ));
        }
    }
    Ok(())
}

fn check_build(build: &Value, selection: &Value) -> Result<(), String> {
    let expected = &selection["build_selection"];
    for key in [
        "edition",
        "rustc_commit",
        "pointer_width",
        "overflow_checks",
        "debug_assertions",
        "panic_strategy",
        "opt_level",
    ] {
        if build[key]["value"] != expected[key] {
            return Err(format!("Selected build changed: {key}"));
        }
    }
    let requested = build["frontend_options"]["target"]
        .as_str()
        .ok_or("Missing requested target")?;
    let canonical = if requested == "aarch64-apple-darwin" {
        "arm64-apple-macosx"
    } else {
        requested
    };
    if build["target_triple"]["value"] != canonical
        || !expected["targets"]
            .as_array()
            .ok_or("Missing selected targets")?
            .contains(&json!(requested))
        || !expected["contexts"]
            .as_array()
            .ok_or("Missing selected contexts")?
            .iter()
            .any(|c| c["features"] == build["features"])
    {
        return Err("Build is outside the six selected contexts".into());
    }
    Ok(())
}

fn origin_oracle(facts: &Value) -> Result<Value, String> {
    // Nominal identity comes from the original declared parameter, not graph output.
    // The independently frozen Line declaration has exactly field zero `remaining`.
    let ty: Value = serde_json::from_str(
        facts["signature"]["params"][0]["ty"]["normalized"]
            .as_str()
            .ok_or("Missing Line type")?,
    )
    .map_err(|e| e.to_string())?;
    let owner = &ty["value"]["value"];
    if owner["nominal"]["definition_path"] != "decoder::Line"
        || owner["fields"][0][0] != "remaining"
        || owner["fields"].as_array().map(Vec::len) != Some(1)
    {
        return Err("Original Line declaration differs from origin oracle".into());
    }
    Ok(json!([{"parameter":0,"path":[{"kind":"SharedReferent"},
        {"kind":"StructField","nominal":owner["nominal"],"position":0,"name":"remaining"},
        {"kind":"SharedReferent"}]}]))
}

fn expected_origins(pair: &Pair, facts: &Value) -> Result<(), String> {
    let expected = origin_oracle(facts)?;
    let suffix = pair
        .pdg
        .nodes
        .iter()
        .find(|n| n.attributes.construct.as_deref() == Some("data.option-get-suffix"))
        .ok_or("Missing suffix")?;
    if suffix.annotations.get(rstyle_pdg::data_origins::KEY) != Some(&expected) {
        return Err("Suffix differs from independent Line.remaining backing origin".into());
    }
    let returns: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|n| n.category == rstyle_pdg::NodeCategory::Return)
        .collect();
    if returns.len() != 2
        || returns
            .iter()
            .filter(|n| n.annotations.get(rstyle_pdg::data_origins::KEY) == Some(&expected))
            .count()
            != 1
        || returns
            .iter()
            .filter(|n| n.annotations.get(rstyle_pdg::data_origins::KEY) == Some(&json!([])))
            .count()
            != 1
    {
        return Err("Expected one unborrowed error and one exact backing success return".into());
    }
    Ok(())
}

fn forge(pair: &Pair, attack: &str) -> Result<Pair, String> {
    let mut pdg = serde_json::to_value(&pair.pdg).map_err(|e| e.to_string())?;
    let mut cfg = pair.cfg.clone();
    if attack == "policy" {
        pdg["profile"]["version"] = json!("3.87.0");
        cfg.profile.version = "3.87.0".into();
    } else if attack == "saved-offset" {
        forge_offset(&mut pdg)?;
    } else if attack == "summary" {
        forge_summary(&mut pdg)?;
        cfg.extensions.insert(
            rstyle_pdg::call_summaries::KEY.into(),
            pdg["extensions"][rstyle_pdg::call_summaries::KEY].clone(),
        );
    } else {
        forge_node(&mut pdg, attack)?;
    }
    Ok(Pair {
        pdg: serde_json::from_value(pdg).map_err(|e| e.to_string())?,
        cfg,
    })
}

fn forge_offset(pdg: &mut Value) -> Result<(), String> {
    let suffix = pdg["nodes"]
        .as_array()
        .ok_or("Missing nodes")?
        .iter()
        .find(|n| n["attributes"]["construct"] == "data.option-get-suffix")
        .ok_or("Missing suffix operand attack site")?["id"]
        .clone();
    let edges = pdg["edges"].as_array_mut().ok_or("Missing edges")?;
    let receiver = edges
        .iter()
        .find(|e| {
            e["target"] == suffix && e["subkind"] == "value_operand" && e["operand_position"] == 0
        })
        .ok_or("Missing saved receiver operand")?["source"]
        .clone();
    let start = edges
        .iter_mut()
        .find(|e| {
            e["target"] == suffix && e["subkind"] == "value_operand" && e["operand_position"] == 1
        })
        .ok_or("Missing saved offset operand")?;
    start["source"] = receiver;

    Ok(())
}

fn forge_summary(pdg: &mut Value) -> Result<(), String> {
    let summaries = pdg["extensions"][rstyle_pdg::call_summaries::KEY]
        .as_array_mut()
        .ok_or("Missing summaries")?;
    let summary = summaries
        .iter_mut()
        .find(|s| s["summary"]["id"] == "rstyle.data.slice.get-suffix")
        .ok_or("Missing suffix summary")?;
    summary["summary"]["always_returns"] = json!(false);

    Ok(())
}

fn forge_node(pdg: &mut Value, attack: &str) -> Result<(), String> {
    let construct = match attack {
        "guard" => "try.scalar-result-is-err",
        "error" => "data.fresh-error-variant",
        _ => "data.option-get-suffix",
    };
    let node = pdg["nodes"]
        .as_array_mut()
        .ok_or("Missing nodes")?
        .iter_mut()
        .find(|n| n["attributes"]["construct"] == construct)
        .ok_or("Missing semantic attack site")?;
    let (path, value) = match attack {
        "suffix" => (
            "/annotations/rstyle.data:checked-suffix-view/some_pointer",
            json!("saved_receiver_pointer"),
        ),
        "operand" => (
            "/annotations/rstyle.data:checked-suffix-view/start_operand",
            json!(0),
        ),
        "guard" => (
            "/annotations/rstyle.data:scalar-result-propagation/true",
            json!("ok"),
        ),
        "error" => ("/attributes/literal/value", json!("InvalidEncoding")),
        "origin" => ("/annotations/rstyle.data:reference-origins", json!([])),
        _ => return Err("Unknown attack".into()),
    };
    *node
        .pointer_mut(path)
        .ok_or("Missing semantic attack field")? = value;

    Ok(())
}
