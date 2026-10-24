use super::*;

fn captured() -> (tempfile::TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "pub fn compare(actual: u8, mut expected: u8) -> bool { Some(actual).map_or({ expected = 1; false }, |byte| byte == expected) }").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["functions"][0]["outcome"], "admitted");
    (directory, report)
}

#[test]
fn callback_documents_reject_omission_early_reads_guard_and_result_corruption() {
    let (_directory, report) = captured();
    let (unit, index) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in [
        "environment",
        "content-read",
        "payload",
        "early-body",
        "none-payload",
        "argument-order",
        "result",
        "policy",
    ] {
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

fn construct(pair: &rstyle_pdg::producer::Pair, name: &str) -> String {
    pair.pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some(name))
        .unwrap()
        .id
        .clone()
}

fn corrupt(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let environment = construct(pair, "callback.shared-byte-environment");
    let selector = construct(pair, "callback.option-some");
    let payload = construct(pair, "data.option-value-payload");
    match mutation {
        "environment" => {
            pair.pdg
                .nodes
                .iter_mut()
                .find(|node| node.id == environment)
                .unwrap()
                .attributes
                .construct = None
        }
        "content-read" => {
            let reads = pair
                .pdg
                .nodes
                .iter()
                .find(|node| !node.attributes.reads.is_empty())
                .unwrap()
                .attributes
                .reads
                .clone();
            pair.pdg
                .nodes
                .iter_mut()
                .find(|node| node.id == environment)
                .unwrap()
                .attributes
                .reads = reads;
        }
        "payload" => pair
            .pdg
            .edges
            .retain(|edge| !(edge.target == payload && edge.subkind == "value_operand")),
        "result" => corrupt_result(pair),
        "policy" => {
            pair.pdg.profile.version = "3.48.0".into();
            pair.cfg.profile.version = "3.48.0".into();
        }
        _ => bypass(pair, mutation, &selector),
    }
}

fn cfg(pair: &rstyle_pdg::producer::Pair, id: &str) -> String {
    rstyle_pdg::read_operation_map(&pair.pdg.extensions)
        .unwrap()
        .into_iter()
        .find(|entry| entry.pdg_node_id == id)
        .unwrap()
        .cfg_node_id
}

fn bypass(pair: &mut rstyle_pdg::producer::Pair, mutation: &str, selector: &str) {
    let target = match mutation {
        "none-payload" => construct(pair, "data.option-value-payload"),
        "argument-order" => construct(pair, "callback.shared-byte-environment"),
        "early-body" => pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.original_kind.known_value() == Some("expr_binary"))
            .unwrap()
            .id
            .clone(),
        _ => unreachable!(),
    };
    let mut edge = pair
        .cfg
        .edges
        .iter()
        .find(|edge| edge.source == cfg(pair, selector))
        .unwrap()
        .clone();
    edge.source = if mutation == "argument-order" {
        pair.cfg.entry_node_id.clone()
    } else {
        cfg(pair, selector)
    };
    edge.target = cfg(pair, &target);
    edge.kind = rstyle_pdg::CfgEdgeKind::Normal;
    pair.cfg.edges.push(edge);
}

fn corrupt_result(pair: &mut rstyle_pdg::producer::Pair) {
    let key = "compat.rust.hir-r0:value-sites";
    for extensions in [&mut pair.pdg.extensions, &mut pair.cfg.extensions] {
        let alternatives = extensions.get_mut(key).unwrap()[0]["alternatives"]
            .as_array_mut()
            .unwrap();
        let some = alternatives
            .iter()
            .find(|alternative| alternative["outcome"] == "true")
            .unwrap()["value_pdg_node_id"]
            .clone();
        alternatives
            .iter_mut()
            .find(|alternative| alternative["outcome"] == "false")
            .unwrap()["value_pdg_node_id"] = some;
    }
}

#[test]
fn callback_capture_evidence_requires_complete_immutable_scalar_storage() {
    let (_directory, report) = captured();
    for mutation in [
        "missing",
        "parameter",
        "payload",
        "capture-mode",
        "projected",
        "storage",
    ] {
        shared_evidence::reject_edit(&report["functions"][0], |bundle, index| {
            assert!(edit_callback(
                &mut bundle["functions"][index]["body"],
                mutation
            ));
        });
    }
}

fn edit_callback(node: &mut Value, mutation: &str) -> bool {
    if node["kind"] == "expr_closure" {
        match mutation {
            "missing" => node["res"] = Value::Null,
            "parameter" => node["children"][0]["param_index"] = 0.into(),
            "payload" => {
                node["children"][1]["children"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            _ => edit_environment(node, mutation),
        }
        return true;
    }
    node["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .any(|child| edit_callback(child, mutation))
}

fn edit_environment(node: &mut Value, mutation: &str) {
    let mut environment: Value =
        serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
    match mutation {
        "capture-mode" => environment["captures"][0]["mode"] = "Value".into(),
        "projected" => environment["captures"][0]["projections"] = 1.into(),
        "storage" => {
            environment["stored_type"] =
                serde_json::json!({"status":"Unsupported","reason":"missing storage evidence"})
        }
        _ => unreachable!(),
    }
    node["res"]["description"] = environment.to_string().into();
}
