use super::*;

#[test]
fn scalar_enum_documents_reject_tag_payload_operand_effect_and_policy_changes() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/tagged-units.rs"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for (name, construct) in [
        ("Unit::new_byte", "data.enum-value"),
        ("Unit::byte", "match.enum-variant"),
        ("Unit::end", "data.enum-payload"),
        ("saved", "data.copy-discard"),
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        for mutation in ["tag", "type", "effects", "operand", "inventory", "policy"] {
            let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
            corrupt(&mut pair, construct, mutation);
            assert!(
                rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
                "{name}/{mutation}"
            );
            assert!(
                rstyle_pdg::producer::verify(&pair, &facts).is_err(),
                "{name}/{mutation}"
            );
            assert!(
                rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
                "{name}/{mutation}"
            );
        }
    }
}

fn corrupt(pair: &mut rstyle_pdg::producer::Pair, construct: &str, mutation: &str) {
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap();
    match mutation {
        "tag" => node.attributes.operator = Some(rstyle_pdg::StringFact::known("Wrong")),
        "type" => {
            node.annotations.get_mut("compat.rust.hir-r0:type").unwrap()["value"] = "u32".into()
        }
        "effects" => node.attributes.callee = Some(rstyle_pdg::StringFact::known("invented")),
        "inventory" => node.attributes.construct = Some("data.unregistered".into()),
        "policy" => {
            pair.cfg.profile.version = "3.46.0".into();
            pair.pdg.profile.version = "3.46.0".into();
        }
        "operand" if construct == "data.copy-discard" => {
            node.attributes.symbol_ids.push("invented".into());
        }
        "operand" => pair
            .pdg
            .edges
            .retain(|edge| !(edge.target == node.id && edge.subkind == "value_operand")),
        _ => unreachable!(),
    }
}

#[test]
fn scalar_enum_payload_requires_its_successful_variant_guard() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"),
        "#[derive(Clone, Copy)] pub enum Kind { Byte(u8), End(u16) }
\
         pub fn byte(kind: Kind) -> Option<u8> { match kind { Kind::Byte(b) => Some(b), Kind::End(_) => None } }").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "byte")
        .unwrap();
    assert_eq!(function["outcome"], "admitted");
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    remove_successful_guard(&mut pair);
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

fn remove_successful_guard(pair: &mut rstyle_pdg::producer::Pair) {
    let selector = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some("match.enum-variant"))
        .unwrap();
    let map = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
    let cfg = &map
        .iter()
        .find(|entry| entry.pdg_node_id == selector.id)
        .unwrap()
        .cfg_node_id;
    let edge = pair
        .cfg
        .edges
        .iter_mut()
        .find(|edge| edge.source == *cfg && edge.kind == rstyle_pdg::CfgEdgeKind::True)
        .unwrap();
    edge.kind = rstyle_pdg::CfgEdgeKind::False;
}
