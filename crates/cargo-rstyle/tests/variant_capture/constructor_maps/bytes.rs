//! Complete checked byte decoder and generated consuming wrappers.
use super::*;

fn source() -> String {
    SOURCE.replace("value: usize", "value: [u8; 4]")
        .replace(
            "if value > 100 { return Err(BaseError { attempted: value as u64 }); }\n        Ok(Base(value as u32))",
            "let value = u32::from_ne_bytes(value);\n        if value > 100 { return Err(BaseError { attempted: u64::from(value) }); }\n        Ok(Base(usize::try_from(value).expect(\"u32 overflowed usize\") as u32))",
        ) + r#"
pub fn wrong_width(value: [u8; 2]) -> Result<Base, BaseError> {
    Ok(Base(u16::from_ne_bytes(value) as u32))
}
pub fn effectful_message(value: u32) -> usize {
    usize::try_from(value).expect({ let _ = value; "message" })
}
"#
}

#[test]
fn checked_byte_decoder_results_compose_through_generated_constructor_maps() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["Base::new", "Wrapper::new", "Wrapper::same"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    for name in ["wrong_width", "effectful_message"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}");
    }
    verify_replay(&report);
    verify_helper_links(&report);
    check_equivalent_matching(&report, ["Wrapper::new", "Wrapper::same"]);
    reject_selected_constructor(function(&report, "Wrapper::new"));
    reject_raw_source(function(&report, "Wrapper::new"));
    reject_decoder_graph(&report);
}

fn reject_decoder_graph(report: &Value) {
    let entry = function(report, "Base::new");
    let (store, _) = shared_evidence::load(entry);
    let analysis = store.analysis();
    let facts = analysis.function(shared_evidence::index(entry)).unwrap();
    let pair = shared_evidence::pair(&store, entry);
    for attack in ["decode", "widen", "limit", "policy"] {
        let mut forged = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        forge_decoder(&mut forged, attack);
        let wire = forged.serialize().unwrap();
        assert!(
            rstyle_pdg::verification_policy::strict(|| rstyle_pdg::producer::verify_serialized(
                &wire, &facts
            ))
            .is_err(),
            "{attack}"
        );
    }
}

fn forge_decoder(forged: &mut rstyle_pdg::producer::Pair, attack: &str) {
    match attack {
        "decode" | "widen" => {
            let construct = if attack == "decode" {
                "data.bytes.native-u32"
            } else {
                "data.scalar-from"
            };
            let node = forged
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.attributes.construct.as_deref() == Some(construct))
                .unwrap();
            node.annotations.clear();
        }
        "limit" => {
            let node = forged
                .pdg
                .nodes
                .iter_mut()
                .find(|node| {
                    node.attributes
                        .literal
                        .as_ref()
                        .and_then(rstyle_pdg::StringFact::known_value)
                        == Some("100")
                })
                .unwrap();
            node.attributes.literal = Some(rstyle_pdg::StringFact::known("101"));
        }
        "policy" => {
            forged.pdg.profile.version = "3.113.0".into();
            forged.cfg.profile.version = "3.113.0".into();
        }
        _ => unreachable!(),
    }
}
