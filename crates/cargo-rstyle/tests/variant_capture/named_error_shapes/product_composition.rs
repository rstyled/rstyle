//! Whole product parser compositions preserve fields, saved counts and error flow.
use super::*;

const KEY: &str = "rstyle.data:reader-product-origins";

const PARSERS: &str = r#"
#[derive(Clone, Copy, Debug)]
pub struct Flags { has_empty: bool, is_utf8: bool, is_always_start_anchored: bool }
impl Flags {
    pub fn from_bytes(slice: &[u8]) -> Result<(Flags, usize), Error> {
        let (bits, nread) = reader(slice, "flag bitset")?;
        let flags = Flags {
            has_empty: bits & (1 << 0) != 0,
            is_utf8: bits & (1 << 1) != 0,
            is_always_start_anchored: bits & (1 << 2) != 0,
        };
        Ok((flags, nread))
    }
    pub fn alpha(bytes: &[u8]) -> Result<(Flags, usize), Error> {
        let (word, consumed) = reader(bytes, "flag bitset")?;
        let value = Flags {
            has_empty: word & (1 << 0) != 0,
            is_utf8: word & (1 << 1) != 0,
            is_always_start_anchored: word & (1 << 2) != 0,
        };
        Ok((value, consumed))
    }
    pub fn wrong_consumed(slice: &[u8]) -> Result<(Flags, usize), Error> {
        let (bits, nread) = reader(slice, "flag bitset")?;
        let flags = Flags {
            has_empty: bits & (1 << 0) != 0,
            is_utf8: bits & (1 << 1) != 0,
            is_always_start_anchored: bits & (1 << 2) != 0,
        };
        Ok((flags, 0))
    }
    pub fn hidden_work(slice: &[u8]) -> Result<(Flags, usize), Error> {
        let (bits, nread) = reader(slice, "flag bitset")?;
        let ignored = bits;
        let flags = Flags {
            has_empty: bits & (1 << 0) != 0,
            is_utf8: bits & (1 << 1) != 0,
            is_always_start_anchored: bits & (1 << 2) != 0,
        };
        Ok((flags, nread))
    }
}
pub fn touched(bits: u32) -> u32 { assert!(bits != u32::MAX); bits }
impl Flags {
    pub fn hidden_call(slice: &[u8]) -> Result<(Flags, usize), Error> {
        let (bits, nr) = reader(slice, "flag bitset")?;
        let flags = Flags {
            has_empty: touched(bits) & (1 << 0) != 0,
            is_utf8: bits & (1 << 1) != 0,
            is_always_start_anchored: bits & (1 << 2) != 0,
        };
        Ok((flags, nr))
    }
}
#[derive(Clone, Debug)]
pub struct NotCopy { has_empty: bool, is_utf8: bool, is_always_start_anchored: bool }
impl NotCopy {
    pub fn unproved_copy(slice: &[u8]) -> Result<(NotCopy, usize), Error> {
        let (bits, nr) = reader(slice, "flag bitset")?;
        let flags = NotCopy {
            has_empty: bits & (1 << 0) != 0,
            is_utf8: bits & (1 << 1) != 0,
            is_always_start_anchored: bits & (1 << 2) != 0,
        };
        Ok((flags, nr))
    }
}
#[derive(Clone, Debug)]
pub struct DropFlags { has_empty: bool, is_utf8: bool, is_always_start_anchored: bool }
impl Drop for DropFlags { fn drop(&mut self) {} }
impl DropFlags {
    pub fn product_hidden_drop(slice: &[u8]) -> Result<(DropFlags, usize), Error> {
        let (bits, nr) = reader(slice, "flag bitset")?;
        let flags = DropFlags {
            has_empty: bits & (1 << 0) != 0,
            is_utf8: bits & (1 << 1) != 0,
            is_always_start_anchored: bits & (1 << 2) != 0,
        };
        Ok((flags, nr))
    }
}
"#;

#[test]
fn named_error_product_parsers_compose_saved_fields_counts_and_guarded_reads() {
    // Fixed independently from the original source: short lengths0..3 preserve
    // BufferTooSmall("flag bitset"); other lengths decode native bits0/1/2,
    // ignore upper bits/tails and return consumed4. The unchanged whole-crate
    // oracle checks82368 cases per default debug-assertions context.
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{SOURCE}\n{}\n{PARSERS}", reader_composition::READERS),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["Flags::from_bytes", "Flags::alpha"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }

    for name in [
        "Flags::wrong_consumed",
        "Flags::hidden_work",
        "Flags::hidden_call",
        "NotCopy::unproved_copy",
        "DropFlags::product_hidden_drop",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
    verify_strict(&report);
}

fn verify_strict(report: &Value) {
    let (raw, _) = shared_evidence::load(function(report, "Flags::from_bytes"));
    let originals = raw.functions().to_vec();
    let store = raw.with_slice_length_instances().unwrap();
    assert_eq!(store.functions(), originals);

    rstyle_pdg::verification_policy::strict(|| {
        let analysis = store.analysis();
        let imported: Vec<_> = ["Flags::from_bytes", "Flags::alpha"]
            .map(|name| {
                let index = shared_evidence::index(function(report, name));
                let facts = analysis.function(index).unwrap();
                let pair = store.pair(index).unwrap();
                rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts)
                    .unwrap();
                check_documents(pair);
                reject_mutations(pair, &facts);
                rstyle_ccgraph::admit_produced(pair, &facts).unwrap()
            })
            .into_iter()
            .collect();
        let matching =
            rstyle_ccgraph::evaluate_all(&imported, &rstyle_ccgraph::CcgraphConfig::default());
        assert!(!matching.incomplete, "{:?}", matching.diagnostics);
        assert_eq!(matching.pairs.len(), 1);
        assert_eq!(
            matching.pairs[0].decision,
            rstyle_ccgraph::Decision::Accepted
        );
        assert_eq!(matching.pairs[0].displayed_score.as_deref(), Some("1"));
    });
}

fn check_documents(pair: &rstyle_pdg::producer::Pair) {
    assert_eq!(
        pair.cfg.extraction_evidence.status,
        rstyle_pdg::ExtractionStatus::Partial
    );
    for node in &pair.pdg.nodes {
        let document = node.annotations.get(KEY).unwrap();
        assert!(!node.annotations.contains_key(rstyle_pdg::data_origins::KEY));
        match document["role"].as_str().unwrap() {
            "scalar_or_explicit_empty_variant" => {
                assert_eq!(document["origins"], serde_json::json!([]))
            }
            "static_literal" => assert_eq!(
                document["origins"][0]["root"]["descriptor"]["value"],
                "flag bitset"
            ),
            "guarded_reader_result" => {
                assert!(!node.attributes.reads.is_empty());
                assert_eq!(document["cases"][0]["variant"], "Ok");
                assert_eq!(document["cases"][0]["origins"], serde_json::json!([]));
                assert_eq!(document["cases"][1]["variant"], "Err");
                assert!(
                    !document["cases"][1]["origins"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                );
            }
            "propagated_static_error" => {
                assert_eq!(document["guard"]["outcome"], "true");
                assert_eq!(document["guard"]["variant"], "Err");
                assert!(!document["origins"].as_array().unwrap().is_empty());
            }
            "formal_reference" => {}
            role => panic!("Unknown product-parser origin role: {role}"),
        }
    }
}

fn reject_mutations(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    for change in [
        "field-order",
        "saved-count",
        "operator",
        "literal",
        "reader-effects",
        "message",
        "saved-actuals",
        "error-guard",
        "error-origin",
        "policy",
    ] {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        mutate(&mut changed, change);
        // Originals cover serialized wire integrity; typed adversaries isolate
        // semantic proof without repeating expensive serialization/seals.
        assert!(
            rstyle_pdg::producer::verify(&changed, facts).is_err(),
            "product parser mutation {change} was accepted"
        );
        if change == "policy" {
            assert!(rstyle_ccgraph::admit_produced(&changed, facts).is_err());
        }
    }
}

fn mutate(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    match change {
        "policy" => {
            pair.pdg.profile.version = "3.122.0".into();
            pair.cfg.profile.version = "3.122.0".into();
        }
        "field-order" | "saved-count" => change_component(pair, change),
        "literal" => change_literal(&mut pair.pdg.nodes),
        "message" | "saved-actuals" | "error-guard" | "error-origin" => {
            change_origins(&mut pair.pdg.nodes, change);
        }
        _ => change_operation(pair, change),
    }
}

fn change_operation(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    if change == "operator" {
        let binary = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| {
                node.attributes
                    .operator
                    .as_ref()
                    .and_then(|operator| operator.known_value())
                    == Some("&")
            })
            .unwrap();
        binary.attributes.operator = Some(rstyle_pdg::StringFact::known("|"));
    } else {
        let reader = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| {
                node.attributes
                    .reads
                    .iter()
                    .any(|read| read.location_id.is_some())
            })
            .unwrap();
        reader.attributes.reads.clear();
    }
}

fn change_component(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    let kind = if change == "field-order" {
        "expr_struct"
    } else {
        "expr_tup"
    };
    let target = &pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.original_kind.known_value() == Some(kind))
        .unwrap()
        .id;
    let edge = pair
        .pdg
        .edges
        .iter_mut()
        .find(|edge| {
            edge.target == *target
                && edge.subkind == "value_operand"
                && edge.operand_position == Some(0)
        })
        .unwrap();
    // Duplicate field/component slots cannot describe the original saved tuple.
    edge.operand_position = Some(1);
}

fn change_origins(nodes: &mut [rstyle_pdg::PdgNode], change: &str) {
    let role = if change == "message" {
        "static_literal"
    } else {
        "propagated_static_error"
    };
    let node = nodes
        .iter_mut()
        .find(|node| node.annotations[KEY]["role"] == role)
        .unwrap();
    let document = node.annotations.get_mut(KEY).unwrap();

    match change {
        "message" => {
            document["origins"][0]["root"]["descriptor"]["value"] =
                serde_json::json!("wrong message")
        }
        "saved-actuals" => document["scope"]["saved_actual_pdg_node_ids"] = serde_json::json!([]),
        "error-guard" => document["guard"]["outcome"] = serde_json::json!("false"),
        _ => document["origins"] = serde_json::json!([]),
    }
}

fn change_literal(nodes: &mut [rstyle_pdg::PdgNode]) {
    let literal = nodes
        .iter_mut()
        .find(|node| {
            node.original_kind.known_value() == Some("expr_lit")
                && node
                    .attributes
                    .literal
                    .as_ref()
                    .and_then(|value| value.known_value())
                    == Some("1")
        })
        .unwrap();
    literal.attributes.literal = Some(rstyle_pdg::StringFact::known("7"));
}
