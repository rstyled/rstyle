use super::*;
use rstyle_pdg::unit_store::EvidenceFacts;

#[test]
fn scalar_alias_requires_its_actual_closed_source_declaration() {
    let (directory, report) = captured(None, false);
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "scalar_alias")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for target in ["u64", "i32"] {
        let mut changed = facts.facts().clone();
        let census = changed
            .build
            .frontend_options
            .get_mut(rstyle_pdg::nominal_boundary::census::KEY)
            .unwrap();
        let file = &mut census["files"][0];
        let bytes = file["bytes"]
            .as_str()
            .unwrap()
            .replace("type AccelTy = u32;", &format!("type AccelTy = {target};"));
        file["digest"] = rstyle_pdg::concrete_types::digest(bytes.as_bytes()).into();
        file["bytes"] = bytes.into();
        assert!(rstyle_pdg::producer::produce(&changed).is_err(), "{target}");
        shared_evidence::reject_facts(function, &changed);
    }
    packing_helper_summaries_cannot_invent_purity_or_erase_conditional_read_occurrences(&report);
    packing_requires_exact_comparison_snapshots_conditional_reads_and_policy(&report);
    packing_release_capture_omits_debug_comparison_operand_effects(directory.path());
}

fn packing_helper_summaries_cannot_invent_purity_or_erase_conditional_read_occurrences(
    report: &Value,
) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "getter")
        .unwrap();
    for mutation in ["panic", "read", "return"] {
        shared_evidence::reject_edit(function, |bundle, _| {
            let helpers = bundle["helpers"].as_array_mut().unwrap();
            let helper = helpers
                .iter_mut()
                .find(|helper| {
                    helper["summary"]["may_panic"] == true
                        && helper["summary"]["memory_reads"]
                            .as_array()
                            .is_some_and(|reads| reads.len() == 2)
                })
                .unwrap();
            match mutation {
                "panic" => helper["summary"]["may_panic"] = false.into(),
                "read" => {
                    helper["summary"]["memory_reads"]
                        .as_array_mut()
                        .unwrap()
                        .pop();
                }
                "return" => helper["summary"]["always_returns"] = true.into(),
                _ => unreachable!(),
            }
        });
    }
}

fn packing_requires_exact_comparison_snapshots_conditional_reads_and_policy(report: &Value) {
    for name in [
        "Accel::as_accel_tys",
        "compare_order",
        "scalar_alias",
        "getter",
        "saved_result",
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "admitted", "{name}");
        let (unit, index) = data_unit(function);
        let analysis = unit.analysis();
        let facts = analysis.function(index).unwrap();
        let mutations: &[&str] = match name {
            "Accel::as_accel_tys" => &["copy", "read", "bounds", "policy"],
            "compare_order" => &["operator", "operand", "order", "policy"],
            "scalar_alias" => &["policy"],
            "getter" => &["read", "panic"],
            _ => &["read", "copy", "policy"],
        };
        for mutation in mutations {
            let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
            corrupt(&mut pair, mutation);
            assert!(
                *mutation == "panic" || rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
                "{name} {mutation}"
            );
            assert!(
                rstyle_pdg::producer::verify(&pair, &facts).is_err(),
                "{name} {mutation}"
            );
            assert!(
                rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
                "{name} {mutation}"
            );
        }
    }
}

fn corrupt(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    match mutation {
        "policy" => {
            pair.pdg.profile.version = "3.52.0".into();
            pair.cfg.profile.version = "3.52.0".into();
        }
        "read" => pair
            .pdg
            .nodes
            .iter_mut()
            .find(|n| {
                n.attributes
                    .reads
                    .iter()
                    .any(|read| read.location_id.is_some())
            })
            .unwrap()
            .attributes
            .reads
            .clear(),
        "copy" => {
            let node = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|n| n.attributes.construct.as_deref() == Some("data.bytes.copy-result"))
                .unwrap();
            node.annotations.get_mut("rstyle.data:byte-call").unwrap()["reads_on"] =
                "always".into();
        }
        "bounds" => {
            let node = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|n| n.attributes.construct.as_deref() == Some("data.subslice-place"))
                .unwrap();
            node.annotations
                .get_mut("rstyle.data:subslice-bounds")
                .unwrap()["implicit_start"] = 5.into();
        }
        "panic" => {
            pair.cfg.extraction_evidence.status = rstyle_pdg::ExtractionStatus::Complete;
            pair.cfg.extraction_evidence.limitations.clear();
        }
        _ => corrupt_comparison(pair, mutation),
    }
}

fn corrupt_comparison(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let assertion = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|n| n.attributes.construct.as_deref() == Some("core::assert_eq"))
        .unwrap();
    if mutation == "operator" {
        assertion.attributes.operator = Some(rstyle_pdg::StringFact::known("assert"));
        return;
    }
    let edges: Vec<_> = pair
        .pdg
        .edges
        .iter_mut()
        .filter(|e| e.target == assertion.id && e.subkind == "value_operand")
        .collect();
    for edge in edges {
        if mutation == "order" {
            edge.operand_position = edge.operand_position.map(|position| 1 - position);
        } else {
            edge.operand_position = Some(0);
        }
    }
}
