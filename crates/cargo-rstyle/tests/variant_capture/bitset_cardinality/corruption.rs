use super::*;

#[test]
fn bitset_cardinality_rejects_changed_integer_graphs() {
    let (_directory, report) = captured(None);
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "LookSet::len")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["policy", "effect", "summary", "result", "inventory"] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        corrupt_graph(&mut pair, mutation);
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

fn corrupt_graph(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    if mutation == "policy" {
        pair.pdg.profile.version = "3.50.0".into();
        pair.cfg.profile.version = "3.50.0".into();
    } else if mutation == "summary" {
        pair.pdg
            .extensions
            .get_mut(rstyle_pdg::call_summaries::KEY)
            .unwrap()[0]["summary"]["may_panic"] = true.into();
    } else if mutation == "inventory" {
        pair.pdg
            .nodes
            .retain(|node| node.attributes.construct.as_deref() != Some("data.bitset.count-u32"));
    } else {
        let node = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| {
                node.attributes.construct.as_deref() == Some("data.integer.unwrap-ok-usize")
            })
            .unwrap();
        match mutation {
            "effect" => node.attributes.symbol_ids.push("invented".into()),
            "result" => {
                node.annotations.get_mut("compat.rust.hir-r0:type").unwrap()["value"] = "u32".into()
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn bitset_cardinality_rejects_changed_core_identity_width_and_result() {
    let (_directory, report) = captured(None);
    let raw: rstyle_pdg::HirFnFacts = serde_json::from_value(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["facts"]["name"] == "LookSet::len")
            .unwrap()["facts"]
            .clone(),
    )
    .unwrap();
    let original = rstyle_pdg::producer::produce(&raw).unwrap();
    for mutation in [
        "count-definition",
        "convert-implementation",
        "unwrap-definition",
        "width",
        "result",
    ] {
        let mut facts = raw.clone();
        corrupt_facts(&mut facts, mutation);
        assert!(rstyle_pdg::producer::produce(&facts).is_err(), "{mutation}");
        assert!(
            rstyle_pdg::data_documents::verify(&original, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&original, &facts).is_err(),
            "{mutation}"
        );
    }
}

fn corrupt_facts(facts: &mut rstyle_pdg::HirFnFacts, mutation: &str) {
    if mutation == "width" {
        facts.build.pointer_width.status = rstyle_pdg::FactStatus::Unknown;
        return;
    }
    let path = match mutation {
        "count-definition" => "core::num::<impl u32>::count_ones",
        "unwrap-definition" => "std::result::Result::<T, E>::unwrap",
        _ => "std::convert::TryFrom::try_from",
    };
    let node = operation(&mut facts.body, path).unwrap();
    if mutation == "result" {
        node.ty.as_mut().unwrap().normalized =
            Some("{\"kind\":\"int_conversion_result\",\"value\":\"u32\"}".into());
        return;
    }
    let record = node.res.as_mut().unwrap();
    let mut value: Value = serde_json::from_str(&record.description).unwrap();
    if mutation == "convert-implementation" {
        value["implementation"]["definition_hash"] = "unreviewed".into();
    } else {
        value["definition_hash"] = "unreviewed".into();
    }
    record.description = serde_json::to_string(&value).unwrap();
}

fn operation<'a>(
    node: &'a mut rstyle_pdg::HirNode,
    path: &str,
) -> Option<&'a mut rstyle_pdg::HirNode> {
    if node
        .dispatch
        .as_ref()
        .is_some_and(|dispatch| dispatch.op == path)
    {
        return Some(node);
    }
    node.children
        .iter_mut()
        .find_map(|child| operation(child, path))
}
