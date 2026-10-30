use super::*;
use rstyle_pdg::unit_store::EvidenceFacts;

#[test]
fn end_of_input_rejects_forged_message_operands_and_panic_summary() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../../tests/fixtures/function-families/end-of-input.rs"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "Unit::eoi")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["message", "operand", "panic", "policy"] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        corrupt(&mut pair, mutation);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{mutation}"
        );
    }
}

fn corrupt(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let assertion = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("core::assert.scalar-format"))
        .unwrap();
    match mutation {
        "message" => assertion.attributes.literal = Some(rstyle_pdg::StringFact::known("invented")),
        "operand" => pair.pdg.edges.retain(|edge| {
            !(edge.target == assertion.id
                && edge.subkind == "value_operand"
                && edge.operand_position == Some(1))
        }),
        "panic" => {
            let records = pair
                .pdg
                .extensions
                .get_mut(rstyle_pdg::call_summaries::KEY)
                .unwrap()
                .as_array_mut()
                .unwrap();
            let record = records
                .iter_mut()
                .find(|record| record["summary"]["id"] == "rstyle.data.integer.unwrap-u16")
                .unwrap();
            record["summary"]["may_panic"] = false.into();
        }
        "policy" => {
            pair.pdg.profile.version = "3.51.0".into();
            pair.cfg.profile.version = "3.51.0".into();
        }
        _ => unreachable!(),
    }
}

#[test]
fn checked_integer_operations_require_exact_definitions_and_known_width() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn narrow(n: usize) -> u16 { u16::try_from(n).unwrap() }\n\
         pub fn subtract(n: usize, k: usize) -> usize { n.checked_sub(k).unwrap() }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted");
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        for mutation in ["unwrap", "producer", "width", "compiler"] {
            let mut changed = facts.facts().clone();
            match mutation {
                "width" => {
                    changed.build.pointer_width.status = rstyle_pdg::FactStatus::Unknown;
                    changed.build.pointer_width.value = None;
                }
                "compiler" => changed.build.rustc_commit.value = Some("unreviewed".into()),
                _ => {
                    let unwrap = &mut changed.body.children[0];
                    let call = if mutation == "unwrap" {
                        unwrap
                    } else {
                        &mut unwrap.children[0]
                    };
                    let resolution = call.res.as_mut().unwrap();
                    let mut callee: Value = serde_json::from_str(&resolution.description).unwrap();
                    if callee.get("implementation").is_some() {
                        callee["implementation"]["definition_hash"] = "unreviewed".into();
                    } else {
                        callee["definition_hash"] = "unreviewed".into();
                    }
                    resolution.description = serde_json::to_string(&callee).unwrap();
                }
            }
            assert!(
                rstyle_pdg::producer::produce(&changed).is_err(),
                "{mutation}"
            );
        }
    }
}
