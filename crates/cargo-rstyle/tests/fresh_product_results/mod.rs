//! Fresh owned Result products retain exact input backing and conditional errors.
use super::*;

const SOURCE: &str = r#"
    #[macro_use] extern crate alloc;
    #[derive(Clone, Copy, Debug)] pub enum Error { InvalidLength }
    use Error::InvalidLength;
    #[derive(Clone)] pub struct Reader<'a> { remaining: &'a [u8], line_width: Option<usize> }
    impl<'a> Reader<'a> {
        pub fn new_unwrapped(bytes: &'a [u8]) -> Result<Self, Error> {
            if bytes.is_empty() { Err(InvalidLength) }
            else { Ok(Self { remaining: bytes, line_width: None }) }
        }
        pub fn alpha(bytes: &'a [u8]) -> Result<Self, Error> {
            if bytes.is_empty() { Err(InvalidLength) }
            else { Ok(Self { remaining: bytes, line_width: None }) }
        }
        pub fn explicit(bytes: &'a [u8]) -> Result<Self, Error> {
            if bytes.is_empty() { return Err(InvalidLength); }
            Ok(Self { remaining: bytes, line_width: None })
        }
        pub fn width(bytes: &'a [u8]) -> Result<Self, Error> {
            Ok(Self { remaining: bytes, line_width: Some(1) })
        }
        pub fn stored(bytes: &'a [u8]) -> Result<Self, Error> {
            let value = Self { remaining: bytes, line_width: None }; Ok(value)
        }
        pub fn stored_result(bytes: &'a [u8]) -> Result<Self, Error> {
            let value = Ok(Self { remaining: bytes, line_width: None }); value
        }
        pub fn moved(value: Self) -> Result<Self, Error> { Ok(value) }
        pub fn discarded(bytes: &'a [u8]) -> Result<Self, Error> {
            Self { remaining: bytes, line_width: None }; Err(InvalidLength)
        }
    }
    pub fn caller(bytes: &[u8]) -> Result<Reader<'_>, Error> { Reader::new_unwrapped(bytes) }
    mod cleanup {
        use super::{Error, InvalidLength};
        pub struct DroppingReader<'a> { remaining: &'a [u8], line_width: Option<usize> }
        impl Drop for DroppingReader<'_> { fn drop(&mut self) {} }
        impl<'a> DroppingReader<'a> {
            pub fn make(bytes: &'a [u8]) -> Result<Self, Error> {
                if bytes.is_empty() { Err(InvalidLength) }
                else { Ok(Self { remaining: bytes, line_width: None }) }
            }
        }
    }
"#;

fn named<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap_or_else(|| panic!("Missing {name}"))
}

#[test]
fn fresh_product_results_connect_checked_ownership_option_fields_and_guarded_origins() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for function in report["functions"].as_array().unwrap() {
        let positive = matches!(
            function["facts"]["name"].as_str(),
            Some(
                "Reader::<'a>::new_unwrapped"
                    | "Reader::<'a>::alpha"
                    | "Reader::<'a>::explicit"
                    | "Reader::<'a>::width"
            )
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{}: {}",
            function["facts"]["name"],
            function["reasons"]
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(
        &report,
        ["Reader::<'a>::new_unwrapped", "Reader::<'a>::alpha"],
    );
    check_origins(named(&report, "Reader::<'a>::new_unwrapped"));
}

fn check_origins(function: &Value) {
    let (store, index) = data_unit(function);
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = shared_evidence::pair(&store, function);
    let expected = serde_json::json!([{"parameter": 0, "path": [{"kind": "SharedReferent"}]}]);
    for construct in ["data.result-ok", "data.result-err"] {
        let node = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.attributes.construct.as_deref() == Some(construct))
            .unwrap();
        assert_eq!(
            node.annotations[rstyle_pdg::data_origins::KEY],
            if construct == "data.result-ok" {
                expected.clone()
            } else {
                serde_json::json!([])
            }
        );
    }
    for attack in ["error-origin", "success-origin", "saved-payload", "policy"] {
        let mut forged = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        forge(&mut forged, attack, &expected);
        let sealed = forged.serialize().unwrap();
        assert!(
            rstyle_pdg::verification_policy::strict(|| rstyle_pdg::producer::verify_serialized(
                &sealed, &facts
            ))
            .is_err(),
            "{attack}"
        );
    }
}

fn forge(pair: &mut rstyle_pdg::producer::Pair, attack: &str, roots: &Value) {
    if attack == "policy" {
        pair.pdg.profile.version = "3.88.0".into();
        pair.cfg.profile.version = "3.88.0".into();
        return;
    }
    let construct = if attack == "error-origin" {
        "data.result-err"
    } else {
        "data.result-ok"
    };
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap();
    if attack == "saved-payload" {
        let id = node.id.clone();
        let edge = pair
            .pdg
            .edges
            .iter_mut()
            .find(|edge| edge.target == id && edge.subkind == "value_operand")
            .unwrap();
        edge.source = id;
    } else {
        node.annotations.insert(
            rstyle_pdg::data_origins::KEY.into(),
            if attack == "error-origin" {
                roots.clone()
            } else {
                serde_json::json!([])
            },
        );
    }
}
