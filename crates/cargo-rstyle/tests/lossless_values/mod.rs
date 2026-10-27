//! Total conversion, original local trait selection and complete helper callers.
use super::*;
use serde_json::json;

const SOURCE: &str = r#"
trait Widen { fn widened(self) -> u64; }
impl Widen for usize {
    fn widened(self) -> u64 { u64::try_from(self).expect("usize overflowed u64") }
}
pub fn caller(n: usize) -> u64 { n.widened() }
pub fn same(n: usize) -> u64 { n.widened() }
pub fn direct(n: usize) -> u64 { u64::try_from(n).expect("usize overflowed u64") }
pub fn arbitrary(n: Result<u64, core::num::TryFromIntError>) -> u64 { n.expect("arbitrary error") }
pub fn narrow(n: usize) -> u16 { u16::try_from(n).expect("not total") }
pub fn effectful(n: usize) -> u64 { u64::try_from(n).expect({ let _ = n; "message" }) }
pub fn escaping() -> &'static str { "outside the contract" }
"#;

#[test]
fn total_conversion_and_local_value_traits_compose_without_erasing_error_or_message() {
    for no_std in [false, true] {
        let directory = clone_workspace();
        let prefix = if no_std { "#![no_std]\n" } else { "" };
        std::fs::write(
            directory.path().join("src/lib.rs"),
            format!("{prefix}{SOURCE}"),
        )
        .unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(output.status.success(), "{report}");
        let functions = report["functions"].as_array().unwrap();
        assert_eq!(functions.len(), 8);
        let (store, _) = shared_evidence::load(&functions[0]);
        for function in functions {
            let name = function["facts"]["name"].as_str().unwrap();
            let positive =
                name.ends_with("::widened") || matches!(name, "caller" | "same" | "direct");
            assert_eq!(
                function["outcome"],
                if positive { "admitted" } else { "excluded" },
                "{name}: {}",
                function["reasons"]
            );
            if positive {
                reject_historical(function, &store);
            }
        }
        reject_selection(functions);
        reject_message(functions, &store);
        assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 6);
        assert_matching(&report);
        assert_eq!(report["pair_evaluation_complete"], true);
        assert_eq!(report["pair_enumeration_complete"], true);
    }
}

fn assert_matching(report: &Value) {
    let key = |name| {
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap()["key"]
            .clone()
    };
    let (caller, same) = (key("caller"), key("same"));
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            pair["first"] == caller && pair["second"] == same
                || pair["first"] == same && pair["second"] == caller
        })
        .unwrap();
    assert_eq!(pair["decision"], "Accepted");
    assert_eq!(pair["displayed_score"], "1");
}

fn reject_message(functions: &[Value], store: &rstyle_pdg::unit_store::UnitStore) {
    let function = functions
        .iter()
        .find(|function| function["facts"]["name"] == "direct")
        .unwrap();
    let pair = shared_evidence::pair(store, function);
    let analysis = store.analysis();
    let facts = analysis.function(shared_evidence::index(function)).unwrap();
    for attack in ["literal", "operand", "construct", "duplicate"] {
        let mut forged = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        forge_message(&mut forged, attack);
        let sealed = forged.serialize().unwrap();
        assert!(
            rstyle_pdg::verification_policy::strict(|| {
                rstyle_pdg::producer::verify_serialized(&sealed, &facts)
            })
            .is_err(),
            "{attack}"
        );
    }
}

fn forge_message(pair: &mut rstyle_pdg::producer::Pair, attack: &str) {
    let message = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.lossless-expect-message"))
        .unwrap();
    match attack {
        "literal" => {
            message.attributes.literal = Some(rstyle_pdg::StringFact::known("\"changed\""))
        }
        "construct" => message.attributes.construct = Some("data.literal".into()),
        "duplicate" => {
            let mut duplicate = message.clone();
            duplicate.id.push_str(":duplicate");
            pair.pdg.nodes.push(duplicate);
        }
        "operand" => {
            let id = message.id.clone();
            pair.pdg
                .edges
                .retain(|edge| !(edge.source == id && edge.subkind == "value_operand"));
        }
        _ => unreachable!(),
    }
}

fn reject_historical(function: &Value, store: &rstyle_pdg::unit_store::UnitStore) {
    let index = shared_evidence::index(function);
    let pair = store.pair(index).unwrap();
    let mut historical = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    historical.pdg.profile.version = "3.95.0".into();
    historical.cfg.profile.version = "3.95.0".into();
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    rstyle_pdg::producer::verify(pair, &facts).unwrap();
    assert!(rstyle_pdg::data_documents::verify(&historical, &facts).is_err());
    let documents = historical.serialize().unwrap();
    assert!(
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&documents, &facts)
        })
        .is_err()
    );
}

fn reject_selection(functions: &[Value]) {
    let function = functions
        .iter()
        .find(|function| function["facts"]["name"] == "caller")
        .unwrap();
    for attack in [
        "missing",
        "duplicate",
        "selected",
        "requested",
        "substitution",
    ] {
        let mut facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let inventory = facts
            .build
            .frontend_options
            .get_mut("rstyle.rust.call-instances")
            .unwrap();
        match attack {
            "missing" => inventory["calls"] = json!([]),
            "duplicate" => {
                let call = inventory["calls"][0].clone();
                inventory["calls"].as_array_mut().unwrap().push(call);
            }
            "selected" => {
                inventory["calls"][0]["selected"]["definition_identity"] = "different-body".into()
            }
            "requested" => {
                inventory["calls"][0]["requested"]["definition_identity"] = "different-trait".into()
            }
            "substitution" => {
                inventory["calls"][0]["requested_arguments"][0]["identity"]["name"] = "u32".into()
            }
            _ => unreachable!(),
        }
        shared_evidence::reject_facts(function, &facts);
    }
}
