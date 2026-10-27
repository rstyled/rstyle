//! Fresh non-Copy owners do not grant ordinary owned transport or cleanup.
use super::*;
use rstyle_pdg::{NodeCategory, StringFact, producer::Pair};

const SOURCE: &str = r#"
mod fresh {
    const ACCEL_CAP: usize = 8;
    #[derive(Clone)]
    pub struct Accel { bytes: [u8; ACCEL_CAP] }
    impl Accel {
        pub fn from_bytes_unchecked(bytes: [u8; 4]) -> Accel {
            Accel { bytes: [bytes[0], bytes[1], bytes[2], bytes[3], 0, 0, 0, 0] }
        }
        pub fn same(bytes: [u8; 4]) -> Accel {
            Accel { bytes: [bytes[0], bytes[1], bytes[2], bytes[3], 0, 0, 0, 0] }
        }
        pub fn scalar(value: u8) -> Accel {
            Accel { bytes: [value, 0, 0, 0, 0, 0, 0, 0] }
        }
        pub fn same_scalar(value: u8) -> Accel {
            Accel { bytes: [value, 0, 0, 0, 0, 0, 0, 0] }
        }
        pub fn owned_parameter(value: Accel) -> Accel { value }
        pub fn owned_local(value: u8) -> Accel {
            let result = Accel { bytes: [value, 0, 0, 0, 0, 0, 0, 0] }; result
        }
        pub fn discarded(value: u8) {
            Accel { bytes: [value, 0, 0, 0, 0, 0, 0, 0] };
        }
        pub fn updated(value: Accel) -> Accel { Accel { ..value } }
        pub fn helper_argument(value: u8) -> u8 {
            take(Accel { bytes: [value, 0, 0, 0, 0, 0, 0, 0] })
        }
    }
    fn take(value: Accel) -> u8 { value.bytes[0] }
    pub fn caller(bytes: [u8; 4]) -> Accel { Accel::from_bytes_unchecked(bytes) }
}
mod cleanup {
    #[derive(Clone)]
    pub struct Dropping { bytes: [u8; 8] }
    impl Drop for Dropping { fn drop(&mut self) {} }
    impl Dropping {
        pub fn make(value: u8) -> Dropping {
            Dropping { bytes: [value, 0, 0, 0, 0, 0, 0, 0] }
        }
    }
}
mod hidden {
    #[derive(Clone)]
    pub struct Owned { bytes: [u8; 8], hidden: String }
    impl Owned {
        pub fn make(value: u8) -> Owned {
            Owned { bytes: [value, 0, 0, 0, 0, 0, 0, 0], hidden: String::new() }
        }
    }
}
"#;

fn named<'a>(report: &'a Value, name: &str) -> &'a Value {
    function_named(report["functions"].as_array().unwrap(), name)
}

#[test]
fn fresh_scalar_owner_construction_preserves_array_slots_without_owned_transport() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    frontier(&report);
    check_equivalent_matching(
        &report,
        ["fresh::Accel::from_bytes_unchecked", "fresh::Accel::same"],
    );
    check_equivalent_matching(
        &report,
        ["fresh::Accel::scalar", "fresh::Accel::same_scalar"],
    );
    let function = named(&report, "fresh::Accel::from_bytes_unchecked");
    let (store, index) = shared_evidence::load(function);
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = store.pair(index).unwrap();
    rstyle_pdg::verification_policy::strict(|| {
        rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts).unwrap();
        rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
        rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
        for attack in ["slots", "padding-order", "missing-padding", "historical"] {
            let mut changed = pair.clone();
            mutate(&mut changed, attack);
            assert!(
                rstyle_pdg::data_documents::verify(&changed, &facts).is_err(),
                "independent replay: {attack}"
            );
            assert!(
                rstyle_pdg::producer::verify_serialized(&changed.serialize().unwrap(), &facts)
                    .is_err(),
                "producer replay: {attack}"
            );
        }
    });
    shared_evidence::reject_edit(function, |bundle, index| {
        bundle["functions"][index]["build"]["frontend_options"]["extraction_profile"] =
            Value::from("hir-data-v2");
    });
}

fn frontier(report: &Value) {
    for name in ["from_bytes_unchecked", "same", "scalar", "same_scalar"] {
        let function = named(report, &format!("fresh::Accel::{name}"));
        assert_eq!(
            function["outcome"], "admitted",
            "{name}: {}",
            function["reasons"]
        );
    }
    for name in [
        "fresh::Accel::owned_parameter",
        "fresh::Accel::owned_local",
        "fresh::Accel::discarded",
        "fresh::Accel::updated",
        "fresh::Accel::helper_argument",
        "fresh::take",
        "fresh::caller",
        "cleanup::Dropping::make",
        "hidden::Owned::make",
    ] {
        let function = named(report, name);
        assert_eq!(
            function["outcome"], "excluded",
            "{name}: {}",
            function["reasons"]
        );
    }
}

fn mutate(pair: &mut Pair, attack: &str) {
    if attack == "historical" {
        pair.pdg.profile.version = "3.129.0".into();
        pair.cfg.profile.version = "3.129.0".into();
        return;
    }
    if attack == "slots" {
        let owner = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| {
                node.category == NodeCategory::Computation
                    && node.attributes.construct.as_deref() == Some("data.struct")
            })
            .unwrap();
        owner.attributes.literal = Some(StringFact::known("[1]"));
        return;
    }
    mutate_array(pair, attack);
}

fn mutate_array(pair: &mut Pair, attack: &str) {
    let array = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.original_kind.known_value() == Some("expr_array"))
        .unwrap()
        .id
        .clone();
    if attack == "missing-padding" {
        let position = pair
            .pdg
            .edges
            .iter()
            .position(|edge| {
                edge.target == array
                    && edge.subkind == "value_operand"
                    && edge.operand_position == Some(7)
            })
            .unwrap();
        pair.pdg.edges.remove(position);
        return;
    }
    assert_eq!(attack, "padding-order");
    for edge in pair
        .pdg
        .edges
        .iter_mut()
        .filter(|edge| edge.target == array && edge.subkind == "value_operand")
    {
        edge.operand_position = match edge.operand_position {
            Some(0) => Some(4),
            Some(4) => Some(0),
            position => position,
        };
    }
}

/// Newly supported fresh constructors retain strict import and historical gates.
pub(crate) fn verify_boundary(function: &Value) {
    let (store, index) = shared_evidence::load(function);
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = store.pair(index).unwrap();
    rstyle_pdg::verification_policy::strict(|| {
        rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts).unwrap();
        rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
        rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
        let mut historical = pair.clone();
        mutate(&mut historical, "historical");
        assert!(rstyle_pdg::data_documents::verify(&historical, &facts).is_err());
        assert!(
            rstyle_pdg::producer::verify_serialized(&historical.serialize().unwrap(), &facts)
                .is_err()
        );
        assert!(rstyle_ccgraph::admit_produced(&historical, &facts).is_err());
    });
}
