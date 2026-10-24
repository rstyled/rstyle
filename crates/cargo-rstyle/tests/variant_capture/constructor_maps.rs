//! Generated constructors preserve both Result payloads through selected helpers.
use super::float_kernels::function;
use super::*;

const SOURCE: &str = r#"
#![no_std]
#[derive(Copy, Clone)]
pub struct Base(u32);
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaseError { attempted: u64 }
impl Base {
    pub fn new(value: usize) -> Result<Base, BaseError> {
        if value > 100 { return Err(BaseError { attempted: value as u64 }); }
        Ok(Base(value as u32))
    }
}
#[derive(Copy, Clone)]
pub struct Wrapper(Base);
macro_rules! identifiers {
    ($name:ident, $err:ident, $iter:ident, $withiter:ident) => {
        impl $name {
            pub fn new(value: usize) -> Result<$name, $err> {
                Base::new(value).map($name).map_err($err)
            }
            pub fn same(value: usize) -> Result<$name, $err> {
                Base::new(value).map($name).map_err($err)
            }
        }
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $err(BaseError);
    };
}
identifiers!(Wrapper, WrappedError, WrapperIter, WithWrapperIter);
"#;

const ORACLE: &str = r#"
#[test]
fn independently_expected_result_payloads() {
    assert!(!core::mem::needs_drop::<BaseError>());
    assert!(!core::mem::needs_drop::<WrappedError>());
    for value in [0, 1, 99, 100, 101, u32::MAX as usize, usize::MAX] {
        for result in [Wrapper::new(value), Wrapper::same(value)] {
            match result {
                Ok(wrapper) => {
                    assert!(value <= 100);
                    assert_eq!(wrapper.0.0 as usize, value);
                }
                Err(error) => {
                    assert!(value > 100);
                    assert_eq!(error.0.attempted, value as u64);
                }
            }
        }
    }
}
"#;

#[test]
fn generated_constructor_maps_compose_with_static_result_helpers() {
    let directory = clone_workspace();
    native_oracle(directory.path());
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "status={}; capture_failures={}; handoff_failures={}; stderr={}",
        output.status,
        report["capture_failures"],
        report["handoff_failures"],
        String::from_utf8_lossy(&output.stderr)
    );
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    for name in ["Base::new", "Wrapper::new", "Wrapper::same"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    verify_replay(&report);
    verify_helper_links(&report);
    check_equivalent_matching(&report, ["Wrapper::new", "Wrapper::same"]);
    reject_selected_constructor(function(&report, "Wrapper::new"));
    reject_raw_source(function(&report, "Wrapper::new"));
}

fn verify_helper_links(report: &Value) {
    let bundle = shared_evidence::wire(function(report, "Wrapper::new"));
    for name in ["Wrapper::new", "Wrapper::same"] {
        let index = shared_evidence::index(function(report, name));
        let links = bundle["links"][index].as_array().unwrap();
        assert_eq!(links.len(), 1, "{name}: exactly one captured helper");
        let callee = links[0]["function"].as_u64().unwrap() as usize;
        assert_eq!(bundle["functions"][callee]["name"], "Base::new");
    }
}

fn native_oracle(directory: &Path) {
    let source = directory.join("oracle.rs");
    let binary = directory.join("oracle");
    std::fs::write(&source, format!("{SOURCE}\n{ORACLE}")).unwrap();
    let compiled = Command::new("rustc")
        .args(["--test", "--edition=2021", "-C", "overflow-checks=on"])
        .arg(source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let executed = Command::new(binary).output().unwrap();
    assert!(
        executed.status.success(),
        "{}{}",
        String::from_utf8_lossy(&executed.stdout),
        String::from_utf8_lossy(&executed.stderr)
    );
}

fn verify_replay(report: &Value) {
    let unit = super::word_classifiers::verify_admissions(report);
    let analysis = unit.analysis();
    for name in ["Wrapper::new", "Wrapper::same"] {
        let index = shared_evidence::index(function(report, name));
        let facts = analysis.function(index).unwrap();
        let pair = unit.pair(index).unwrap();
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts)
        })
        .unwrap();
        for construct in ["map.result-wrap-ok", "map.result-wrap-err"] {
            assert_eq!(
                pair.pdg
                    .nodes
                    .iter()
                    .filter(|node| node.attributes.construct.as_deref() == Some(construct))
                    .count(),
                2,
                "{name}: {construct}"
            );
        }
        reject_swapped_result(pair, &facts);
    }
}

fn reject_swapped_result(
    pair: &rstyle_pdg::producer::Pair,
    facts: &impl rstyle_pdg::unit_store::EvidenceFacts,
) {
    let mut wire = serde_json::to_value(pair).unwrap();
    let error = wire["pdg"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["attributes"]["construct"] == "map.result-wrap-err")
        .unwrap();
    error["attributes"]["construct"] = "map.result-wrap-ok".into();
    let forged: rstyle_pdg::producer::Pair = serde_json::from_value(wire).unwrap();
    assert!(
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&forged.serialize().unwrap(), facts)
        })
        .is_err()
    );
    assert!(rstyle_ccgraph::admit_produced(&forged, facts).is_err());
}

fn reject_selected_constructor(entry: &Value) {
    shared_evidence::reject_edit(entry, |bundle, index| {
        let calls = bundle["functions"][index]["build"]["frontend_options"]
            ["rstyle.rust.call-instances"]["calls"]
            .as_array_mut()
            .unwrap();
        let call = calls
            .iter_mut()
            .find(|call| {
                call["requested"]["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("::map_err"))
            })
            .expect("selected core Result::map_err identity");
        call["selected"]["definition_identity"] = "forged-result-map".into();
    });
}

fn reject_raw_source(entry: &Value) {
    shared_evidence::reject_edit(entry, |bundle, _| {
        let captured = bundle["functions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find_map(|function| {
                function["build"]["frontend_options"]
                    .get_mut(rstyle_pdg::generated_sources::capture::KEY)
            })
            .expect("one retained generated-source unit table");
        let outcome = captured["outcomes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|outcome| outcome["observation"]["owner"]["path"] == "Wrapper::new")
            .expect("complete generated constructor source observation");
        let span = &mut outcome["observation"]["body"]["raw"]["span"];
        let start = span["start"].as_u64().unwrap();
        span["start"] = (start + 1).into();
    });
}
