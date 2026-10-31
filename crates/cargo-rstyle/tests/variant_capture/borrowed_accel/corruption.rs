use super::*;

#[test]
fn borrowed_array_views_reject_changed_bounds_reads_origins_and_policy() {
    let (_directory, report) = captured(None);
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "Accel::needles")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["bounds", "place-read", "callee-read", "origin", "policy"] {
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
    view_callers_cannot_erase_callee_panic_or_field_origin(&report);
}

fn corrupt(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    match mutation {
        "bounds" => {
            let node = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.attributes.construct.as_deref() == Some("data.subslice-place"))
                .unwrap();
            node.annotations
                .get_mut("rstyle.data:subslice-bounds")
                .unwrap()["array_length"] = 0.into();
        }
        "place-read" => {
            add_place_read(pair);
        }
        "callee-read" => pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| {
                node.category == rstyle_pdg::NodeCategory::Call && !node.attributes.reads.is_empty()
            })
            .unwrap()
            .attributes
            .reads
            .clear(),
        "origin" => {
            let node = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
                .unwrap();
            node.annotations
                .get_mut(rstyle_pdg::data_origins::KEY)
                .unwrap()[0]["path"]
                .as_array_mut()
                .unwrap()
                .pop();
        }
        "policy" => {
            pair.pdg.profile.version = "2.0.0".into();
            pair.cfg.profile.version = "2.0.0".into();
        }
        _ => unreachable!(),
    }
}

fn view_callers_cannot_erase_callee_panic_or_field_origin(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for name in ["getter_view", "first", "saved", "selected"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        for mutation in ["complete", "unknown", "origin"] {
            let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
            corrupt_caller(&mut pair, mutation);
            assert!(
                rstyle_pdg::producer::verify(&pair, &facts).is_err(),
                "{name}: {mutation}"
            );
            assert!(
                rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
                "{name}: {mutation}"
            );
        }
    }
}

fn corrupt_caller(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    match mutation {
        "complete" => {
            pair.cfg.extraction_evidence.status = rstyle_pdg::ExtractionStatus::Complete;
            pair.cfg.extraction_evidence.limitations.clear();
        }
        "unknown" => pair.cfg.extraction_evidence.status = rstyle_pdg::ExtractionStatus::Unknown,
        "origin" => {
            let call = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.category == rstyle_pdg::NodeCategory::Call)
                .unwrap();
            call.annotations
                .get_mut(rstyle_pdg::data_origins::KEY)
                .unwrap()[0]["path"]
                .as_array_mut()
                .unwrap()
                .pop();
        }
        _ => unreachable!(),
    }
}

fn add_place_read(pair: &mut rstyle_pdg::producer::Pair) {
    let reads = pair
        .pdg
        .nodes
        .iter()
        .find(|node| {
            node.category == rstyle_pdg::NodeCategory::Call && !node.attributes.reads.is_empty()
        })
        .unwrap()
        .attributes
        .reads
        .clone();
    pair.pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.place"))
        .unwrap()
        .attributes
        .reads = reads;
}
