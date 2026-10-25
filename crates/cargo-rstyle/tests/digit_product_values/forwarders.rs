//! The original wrapper bodies must compose with the whole original parser.
use super::*;

#[test]
fn original_digit_forwarders_preserve_whole_guarded_results_and_read_occurrences() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), fixture_source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["callers::hour", "callers::min"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "admitted", "{}", function["reasons"]);
        let (store, index) = data_unit(function);
        let analysis = store.analysis();
        let facts = analysis.function(index).unwrap();
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        rstyle_pdg::producer::verify(&pair, &facts).unwrap();
        rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
        if name == "callers::hour" {
            reject_document_changes(&pair, &facts);
            reject_call_selection(&store, index);
        }
        expected_call(&pair);
    }
    for name in ["callers::changed", "callers::extra"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "excluded", "{name}");
    }
    check_complete_data_pairs(&report, 3);
    expected_wrapper_match(&report);
}

fn reject_document_changes(original: &Pair, facts: &dyn rstyle_pdg::unit_store::EvidenceFacts) {
    for attack in [
        "scope",
        "callee",
        "whole-result",
        "flatten",
        "missing-read",
        "merged-read",
        "selector",
        "old-policy",
    ] {
        let mut pair = Pair {
            pdg: original.pdg.clone(),
            cfg: original.cfg.clone(),
        };
        edit(&mut pair, attack);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, facts).is_err(),
            "document accepted {attack}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, facts).is_err(),
            "producer accepted {attack}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, facts).is_err(),
            "admission accepted {attack}"
        );
    }
}

fn edit(pair: &mut Pair, attack: &str) {
    let key = "rstyle.data:digit-product-call";
    match attack {
        "scope" => pair.pdg.extensions.get_mut(key).unwrap()["call"]["local"] = 99999.into(),
        "callee" => pair.pdg.extensions.get_mut(key).unwrap()["callee"] = Value::Null,
        "whole-result" => {
            pair.pdg.extensions.get_mut(key).unwrap()["result"] = "success-only".into()
        }
        "flatten" => {
            pair.pdg
                .nodes
                .iter_mut()
                .find(|node| node.category == NodeCategory::Call)
                .unwrap()
                .annotations
                .insert(
                    rstyle_pdg::data_origins::KEY.into(),
                    serde_json::json!([{"parameter":0,"path":[]}]),
                );
        }
        "missing-read" | "merged-read" => {
            let call = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.category == NodeCategory::Call)
                .unwrap();
            if attack == "missing-read" {
                call.attributes.reads.pop();
            } else {
                call.attributes.reads[1] = call.attributes.reads[0].clone();
            }
        }
        "selector" => {
            let location = &mut pair.pdg.memory_locations[0];
            location
                .extensions
                .get_mut(rstyle_pdg::data_documents::LOCATION_KEY)
                .unwrap()["path"][1]["operation"] = "other".into();
        }
        "old-policy" => {
            pair.pdg.profile.version = "3.112.0".into();
            pair.cfg.profile.version = "3.112.0".into();
        }
        _ => unreachable!(),
    }
}

fn fixture_source() -> String {
    let selection = selected_sources();
    let wrappers: Value = serde_json::from_str(include_str!(
        "../../../../eval/coverage-development/time-digit-forwarders/selection.json"
    ))
    .unwrap();
    let mut source = format!(
        "pub(crate) struct ParsedItem<'a,T>(&'a [u8],T);\n\
         pub(crate) struct ExactlyNDigits<const N:u8>;\n\
         impl ExactlyNDigits<2> {{ {} }}\nmod callers {{\n\
         use super::{{ParsedItem, ExactlyNDigits}};\n",
        selection["functions"][1]["source"].as_str().unwrap()
    );
    for wrapper in wrappers["functions"].as_array().unwrap() {
        source.push_str(wrapper["source"].as_str().unwrap());
        source.push('\n');
    }
    source.push_str("fn changed(input: &[u8]) -> Option<ParsedItem<'_,u8>> { ExactlyNDigits::<2>::parse(&input[0..]) }\n\
        fn extra(input: &[u8]) -> Option<ParsedItem<'_,u8>> { let _ = input[0]; ExactlyNDigits::<2>::parse(input) }\n}\n");
    source
}

fn expected_call(pair: &Pair) {
    let call = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == NodeCategory::Call)
        .unwrap();
    assert_eq!(pair.pdg.nodes.len(), 4);
    assert_eq!(call.attributes.reads.len(), 4);
    assert_eq!(pair.pdg.memory_locations.len(), 4);
    assert_eq!(
        pair.pdg.extensions["rstyle.data:digit-product-call"]["conditions"],
        "callee-graph-scoped-under-call/1"
    );
}

fn expected_wrapper_match(report: &Value) {
    let keys: Vec<_> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| {
            matches!(
                function["facts"]["name"].as_str(),
                Some("callers::hour" | "callers::min")
            )
        })
        .map(|function| &function["key"])
        .collect();
    assert_eq!(keys.len(), 2);
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| keys.contains(&&pair["first"]) && keys.contains(&&pair["second"]))
        .unwrap();
    assert_eq!(pair["decision"], "Accepted");
}

fn reject_call_selection(original: &rstyle_pdg::unit_store::UnitStore, index: usize) {
    for attack in ["width", "implementation"] {
        let mut functions = original.functions().to_vec();
        edit_selection(&mut functions[index].body.children[0], attack);
        let store = rstyle_pdg::unit_store::UnitStore::capture(functions).unwrap();
        let analysis = store.analysis();
        let facts = analysis.function(index).unwrap();
        let errors = rstyle_pdg::producer::produce(&facts)
            .err()
            .unwrap()
            .join(";");
        let expected = if attack == "implementation" {
            "Resolved inherent call/receiver"
        } else {
            "Resolved helper identity, signature"
        };
        assert!(errors.contains(expected), "{attack}: {errors}");
    }
}

fn edit_selection(call: &mut rstyle_pdg::HirNode, attack: &str) {
    let mut resolution: Value =
        serde_json::from_str(&call.res.as_ref().unwrap().description).unwrap();
    if attack == "width" {
        let path = resolution["path"]
            .as_str()
            .unwrap()
            .replace("::<2>", "::<1>");
        assert_ne!(resolution["path"], path);
        resolution["path"] = path.clone().into();
        call.dispatch.as_mut().unwrap().op = path;
    } else {
        resolution["inherent"]["implementation_identity"] = "another-original-impl".into();
    }
    call.res.as_mut().unwrap().description = serde_json::to_string(&resolution).unwrap();
}
