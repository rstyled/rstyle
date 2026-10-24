//! Checked original conversions admit; unproved tagged reference callers stay excluded.
use super::float_kernels::function;
use super::*;
use rstyle_pdg::concrete_types::{InterfaceObservations, Observation, TypeTree};

const SOURCE: &str = r#"
#![no_std]
mod component_range { pub struct ComponentRange { unrelated: usize } }
mod error {
    mod component_range {
        #[derive(Clone, Copy)]
        pub struct ComponentRange { name: &'static str, is_conditional: bool }
    }
    mod different_variant {
        #[derive(Clone, Copy)]
        pub struct DifferentVariant;
    }
    pub use component_range::ComponentRange;
    pub use different_variant::DifferentVariant;
}
use error::{ComponentRange, DifferentVariant};
#[derive(Clone, Copy)]
pub enum TryFromParsed { Empty, ComponentRange(error::ComponentRange) }
pub struct Outer { checked: TryFromParsed }
impl From<error::ComponentRange> for TryFromParsed {
    fn from(v: error::ComponentRange) -> Self { Self::ComponentRange(v) }
}
impl TryFrom<TryFromParsed> for error::ComponentRange {
    type Error = error::DifferentVariant;
    fn try_from(err: TryFromParsed) -> Result<Self, Self::Error> {
        match err {
            TryFromParsed::ComponentRange(err) => Ok(err),
            _ => Err(error::DifferentVariant),
        }
    }
}
pub fn wrap(value: ComponentRange) -> TryFromParsed { TryFromParsed::ComponentRange(value) }
pub fn unwrap(value: TryFromParsed) -> Result<ComponentRange, DifferentVariant> {
    match value { TryFromParsed::ComponentRange(payload) => Ok(payload), _ => Err(DifferentVariant) }
}
pub fn borrowed(value: &Outer) -> bool { matches!(value.checked, TryFromParsed::Empty) }
"#;

#[test]
fn tagged_string_payloads_are_captured_without_silent_reference_omission_or_admission() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["wrap", "unwrap", "borrowed"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
        let (store, index) = shared_evidence::load(entry);
        let facts = &store.functions()[index];
        let interface = facts
            .build
            .frontend_options
            .data::<InterfaceObservations>()
            .unwrap();
        interface.validate().unwrap();
        let Observation::Captured { ty: parameter } = &interface.parameters[0] else {
            panic!("Missing complete {name} input observation");
        };
        let Observation::Captured { ty: result } = &interface.result else {
            panic!("Missing complete {name} result observation");
        };
        verify_types(name, parameter, result);
    }
    verify_source_probes(&report);
}

fn verify_types(name: &str, parameter: &TypeTree, result: &TypeTree) {
    use rstyle_pdg::data_types::TypeToken;
    let tagged = if name == "wrap" { result } else { parameter };
    assert!(rstyle_pdg::data_origins::shared(&TypeToken::from_tree(
        tagged
    )));
    if name == "unwrap" {
        let TypeTree::Result { ok, err } = result else {
            panic!("Missing exact Result observation");
        };
        assert!(matches!(ok.as_ref(), TypeTree::Struct { fields, .. } if fields.len() == 2));
        let TypeTree::Struct {
            nominal,
            source,
            fields,
        } = err.as_ref()
        else {
            panic!("Error must retain its actual nominal unit struct");
        };
        assert_eq!(
            nominal.definition_path.rsplit("::").next(),
            Some("DifferentVariant")
        );
        assert!(fields.is_empty());
        let syntax: syn::ItemStruct = syn::parse_str(source).unwrap();
        assert!(matches!(syntax.fields, syn::Fields::Unit));
    }
}

fn verify_source_probes(report: &Value) {
    let entries: Vec<_> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| {
            entry["facts"]["name"]
                .as_str()
                .is_some_and(|name| name.ends_with("::from") || name.ends_with("::try_from"))
        })
        .collect();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        assert_eq!(entry["outcome"], "admitted", "{}", entry["reasons"]);
        let (store, index) = shared_evidence::load(entry);
        let facts = &store.functions()[index];
        rstyle_pdg::nominal_boundary::resolution::probe_copy_conversion_source(facts)
            .unwrap_or_else(|error| panic!("{}: {error}", facts.name));
        let mut changed = facts.clone();
        changed.signature.is_unsafe = true;
        assert!(
            rstyle_pdg::nominal_boundary::resolution::probe_copy_conversion_source(&changed)
                .is_err()
        );
    }
}

#[test]
fn direct_local_nominals_do_not_traverse_a_parent_module_value_namesake() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        mod views {
            mod parse {
                #[derive(Clone, Copy)]
                pub struct Location { byte: u32 }
                impl Location {
                    pub fn same(self) -> Self { self }
                    pub fn duplicate(self) -> Self { self }
                }
                pub fn parse() -> u32 { 0 }
            }
            pub use self::parse::parse;
        }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in [
        "views::parse::Location::same",
        "views::parse::Location::duplicate",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
        shared_evidence::load(entry);
    }
}

#[test]
fn tagged_conversion_helpers_preserve_saved_arguments_and_scoped_alternatives() {
    let directory = clone_workspace();
    let source = forwarding_source();
    verify_forwarding_values(directory.path(), &source);
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "stderr:{} stdout:{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    for name in ["forward", "forward_same", "inverse", "inverse_same"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
        let (store, index) = shared_evidence::load(entry);
        let analysis = store.analysis();
        let facts = analysis.function(index).unwrap();
        let pair = store.pair(index).unwrap();
        rstyle_pdg::producer::verify(pair, &facts).unwrap();
        rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
        assert_eq!(pair.pdg.nodes.len(), 4);
        assert!(
            pair.pdg
                .extensions
                .contains_key("rstyle.data:copy-tagged-call-contract")
        );
    }
    reject_tagged_summary(function(&report, "inverse"));
    check_tagged_matching(&report);
    reject_tagged_forwarding_graphs(function(&report, "inverse"));
    for name in ["wrong_actual", "ignored", "wrap", "unwrap", "borrowed"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
}

fn verify_forwarding_values(directory: &Path, source: &str) {
    let declaration = "pub struct ComponentRange { name: &'static str, is_conditional: bool }";
    let oracle_source = source.replace(declaration, &format!("{declaration}\n{FORWARDING_ORACLE}"));
    let path = directory.join("oracle.rs");
    let binary = directory.join("oracle");
    std::fs::write(&path, oracle_source).unwrap();
    let compile = Command::new("rustc")
        .args(["--edition=2021", "--test"])
        .arg(&path)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let output = Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

fn reject_tagged_forwarding_graphs(entry: &Value) {
    let (store, index) = shared_evidence::load(entry);
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let original = store.pair(index).unwrap();
    for mutation in [
        "call",
        "actual",
        "binding",
        "parameter",
        "cases",
        "guard",
        "role",
        "value_edge",
        "definition",
        "disconnected",
        "bypass",
        "extra_node",
        "return_form",
        "return_effect",
        "call_effect",
        "legacy",
        "unscoped",
    ] {
        let mut pair = rstyle_pdg::producer::Pair {
            pdg: original.pdg.clone(),
            cfg: original.cfg.clone(),
        };
        corrupt_tagged_graph(&mut pair, mutation);
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

fn corrupt_tagged_graph(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    match mutation {
        "call" | "actual" | "binding" | "parameter" | "cases" | "guard" => {
            let contract = pair
                .pdg
                .extensions
                .get_mut("rstyle.data:copy-tagged-call-contract")
                .unwrap();
            corrupt_tagged_contract(contract, mutation);
        }
        "disconnected" => pair.cfg.edges.clear(),
        "bypass" => pair.cfg.edges[0].target = pair.cfg.exit_node_ids[0].clone(),
        "extra_node" => pair.pdg.nodes.push(pair.pdg.nodes[0].clone()),
        "unscoped" => {
            pair.pdg.extensions.insert(
                "rstyle.data:copy-tagged-value-contract".into(),
                serde_json::json!({}),
            );
        }
        _ => corrupt_tagged_edges(pair, mutation),
    }
}

fn corrupt_tagged_node(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    use rstyle_pdg::NodeCategory;
    let category = if mutation == "call_effect" {
        NodeCategory::Call
    } else {
        NodeCategory::Return
    };
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.category == category)
        .unwrap();
    match mutation {
        "role" => {
            node.annotations
                .insert("rstyle.data:copy-tagged-call-role".into(), "input".into());
        }
        "return_form" => {
            node.attributes
                .extensions
                .insert("compat.rust.hir-r0:return-form".into(), "explicit".into());
        }
        "return_effect" | "call_effect" => node.attributes.defines.push("invented".into()),
        "legacy" => {
            node.annotations.insert(
                "rstyle.data:reference-origins".into(),
                serde_json::json!([]),
            );
        }
        _ => panic!("Unknown mutation {mutation}"),
    }
}

fn check_tagged_matching(report: &Value) {
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    for (first, second) in [("forward", "forward_same"), ("inverse", "inverse_same")] {
        let first = &function(report, first)["key"];
        let second = &function(report, second)["key"];
        let pair = report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pair| {
                (&pair["first"] == first && &pair["second"] == second)
                    || (&pair["first"] == second && &pair["second"] == first)
            })
            .unwrap();
        assert_eq!(pair["decision"], "Accepted", "{pair}");
    }
}

fn reject_tagged_summary(entry: &Value) {
    for mutation in ["missing", "guard", "empty_error", "effects", "legacy"] {
        shared_evidence::reject_edit(entry, |bundle, _| {
            let summary = &mut bundle["helpers"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|helper| {
                    helper["summary"]["tagged_output"]["output_cases"]
                        .as_array()
                        .is_some_and(|cases| cases.len() == 2)
                })
                .unwrap()["summary"];
            match mutation {
                "missing" => {
                    summary.as_object_mut().unwrap().remove("tagged_output");
                }
                "guard" => {
                    summary["tagged_output"]["alternatives"][0]["condition"] =
                        serde_json::json!({"kind":"always"})
                }
                "empty_error" => {
                    summary["tagged_output"]["output_cases"]
                        .as_array_mut()
                        .unwrap()
                        .pop();
                }
                "effects" => summary["may_panic"] = true.into(),
                "legacy" => summary["reference_outputs"] = serde_json::json!([]),
                _ => unreachable!(),
            }
        });
    }
}

fn forwarding_source() -> String {
    format!(
        "{SOURCE}\n{}",
        r#"
        pub fn forward(value: ComponentRange) -> TryFromParsed {
            TryFromParsed::from(value)
        }
        pub fn forward_same(value: ComponentRange) -> TryFromParsed {
            TryFromParsed::from(value)
        }
        pub fn inverse(value: TryFromParsed) -> Result<ComponentRange, DifferentVariant> {
            ComponentRange::try_from(value)
        }
        pub fn inverse_same(value: TryFromParsed) -> Result<ComponentRange, DifferentVariant> {
            ComponentRange::try_from(value)
        }
        pub fn wrong_actual(first: ComponentRange, second: ComponentRange) -> TryFromParsed {
            TryFromParsed::from(second)
        }
        pub fn ignored(value: ComponentRange) -> TryFromParsed {
            let _ = TryFromParsed::from(value); TryFromParsed::Empty
        }
    "#
    )
}

const FORWARDING_ORACLE: &str = r#"
        #[test]
        fn forwarding_value_oracle() {
            for (name, is_conditional) in [("hour", false), ("minute", true), ("", false)] {
                let value = ComponentRange { name, is_conditional };
                match crate::forward(value) {
                    crate::TryFromParsed::ComponentRange(payload) => {
                        assert_eq!(payload.name, name);
                        assert_eq!(payload.is_conditional, is_conditional);
                    }
                    _ => panic!("From must retain the payload"),
                }
                match crate::inverse(crate::forward(value)) {
                    Ok(payload) => {
                        assert_eq!(payload.name, name);
                        assert_eq!(payload.is_conditional, is_conditional);
                    }
                    Err(_) => panic!("Selected payload must succeed"),
                }
            }
            assert!(matches!(crate::inverse(crate::TryFromParsed::Empty), Err(_)));
        }
    "#;

fn corrupt_tagged_contract(contract: &mut Value, mutation: &str) {
    match mutation {
        "call" | "actual" => contract[mutation]["local"] = 99999.into(),
        "binding" => contract["caller_binding"]["local"] = 99999.into(),
        "parameter" => contract["caller_parameter"] = 1.into(),
        "cases" => {
            contract["callee"]["output_cases"]
                .as_array_mut()
                .unwrap()
                .pop();
        }
        "guard" => {
            contract["callee"]["alternatives"][0]["condition"] =
                serde_json::json!({"kind":"always"})
        }
        _ => unreachable!(),
    }
}

fn corrupt_tagged_edges(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    use rstyle_pdg::NodeCategory;
    match mutation {
        "value_edge" => {
            let returned = pair
                .pdg
                .nodes
                .iter()
                .find(|node| node.category == NodeCategory::Return)
                .unwrap();
            pair.pdg
                .edges
                .iter_mut()
                .find(|edge| edge.target == returned.id && edge.subkind == "value_operand")
                .unwrap()
                .source = pair.pdg.nodes[0].id.clone();
        }
        "definition" => {
            pair.pdg
                .edges
                .iter_mut()
                .find(|edge| edge.subkind == "reaching_definition")
                .unwrap()
                .definition_id = Some("invented".into());
        }
        _ => corrupt_tagged_node(pair, mutation),
    }
}
