//! Complete checked byte access / Option / captured shared-helper composition.
mod source_bindings;
use super::float_kernels::{function, reject_resealed};
use super::*;

const SOURCE: &str = r#"
#[derive(Clone, Copy)]
pub struct StateID(u32);
#[derive(Clone, Copy)]
pub struct Transition { pub start: u8, pub end: u8, pub next: StateID }
#[derive(Clone, Copy)]
pub struct Unit(UnitKind);
#[derive(Clone, Copy)]
enum UnitKind { U8(u8), EOI(u16) }
impl Unit {
    pub fn as_u8(self) -> Option<u8> {
        match self.0 { UnitKind::U8(b) => Some(b), UnitKind::EOI(_) => None }
    }
}
impl Transition {
    pub fn matches(&self, haystack: &[u8], at: usize) -> bool {
        haystack.get(at).map_or(false, |&b| self.matches_byte(b))
    }
    pub fn matches_unit(&self, unit: Unit) -> bool {
        unit.as_u8().map_or(false, |byte| self.matches_byte(byte))
    }
    pub fn matches_byte(&self, byte: u8) -> bool {
        self.start <= byte && byte <= self.end
    }
    pub fn same(&self, haystack: &[u8], at: usize) -> bool {
        haystack.get(at).map_or(false, |&b| self.matches_byte(b))
    }
    pub fn hidden_effect(&self, haystack: &[u8], at: usize) -> bool {
        haystack.get(at).map_or(false, |&b| { panic!("hidden"); self.matches_byte(b) })
    }
    pub fn extra_capture(&self, haystack: &[u8], at: usize, extra: bool) -> bool {
        haystack.get(at).map_or(false, |&b| self.matches_byte(b) && extra)
    }
    pub fn moving_capture(&self, haystack: &[u8], at: usize) -> bool {
        haystack.get(at).map_or(false, move |&b| self.matches_byte(b))
    }
}
"#;

fn capture(target: Option<&str>, no_std: bool) -> (TempDir, Value) {
    let directory = clone_workspace();
    let prefix = if no_std { "#![no_std]\n" } else { "" };
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{prefix}{SOURCE}"),
    )
    .unwrap();
    let mut flags = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        flags.extend(["--target", target]);
    }
    let (output, report) = clones(directory.path(), &flags);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn transition_callbacks_compose_checked_access_payloads_and_shared_helpers() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let library = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(library.status.success());
        if !Path::new(String::from_utf8_lossy(&library.stdout).trim()).is_dir() {
            continue;
        }
        for no_std in [false, true] {
            let (_directory, report) = capture(Some(target), no_std);
            for name in [
                "Unit::as_u8",
                "Transition::matches",
                "Transition::matches_unit",
                "Transition::matches_byte",
                "Transition::same",
            ] {
                let entry = function(&report, name);
                assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
            }
            for name in [
                "Transition::hidden_effect",
                "Transition::extra_capture",
                "Transition::moving_capture",
            ] {
                assert_eq!(function(&report, name)["outcome"], "excluded", "{name}");
            }
            super::word_classifiers::verify_admissions(&report);
            for flag in [
                "capture_complete",
                "assessment_complete",
                "pair_enumeration_complete",
                "pair_evaluation_complete",
            ] {
                assert_eq!(report[flag], true, "{flag}");
            }
            let left = &function(&report, "Transition::matches")["key"];
            let right = &function(&report, "Transition::same")["key"];
            assert!(report["matching"]["pairs"].as_array().unwrap().iter().any(
                |pair| ((&pair["first"] == left && &pair["second"] == right)
                    || (&pair["first"] == right && &pair["second"] == left))
                    && pair["decision"] == "Accepted"
            ));
        }
    }
}

#[test]
fn transition_callbacks_reject_resealed_core_capture_and_payload_changes() {
    let (_directory, report) = capture(None, false);
    for mutation in [
        "core_authority",
        "capture_mode",
        "missing_shared_formal",
        "projection_type",
        "capture_root",
        "tuple_storage",
        "payload_binding",
    ] {
        let entry = function(&report, "Transition::matches");
        let mut bundle = shared_evidence::wire(entry);
        let index = shared_evidence::index(entry);
        assert!(corrupt(&mut bundle["functions"][index]["body"], mutation));
        reject_resealed(bundle, mutation);
    }
    transition_callback_guards_reads_and_historical_policy_are_independently_required(&report);
}

fn corrupt(node: &mut Value, mutation: &str) -> bool {
    if mutation == "core_authority"
        && node["kind"] == "expr_method_call"
        && node["snippet"]
            .as_str()
            .is_some_and(|text| text.contains("map_or"))
    {
        let mut callee: Value =
            serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
        callee["definition_hash"] = "unregistered".into();
        node["res"]["description"] = serde_json::to_string(&callee).unwrap().into();
        return true;
    }
    if [
        "capture_mode",
        "missing_shared_formal",
        "projection_type",
        "capture_root",
        "tuple_storage",
    ]
    .contains(&mutation)
        && node["kind"] == "expr_closure"
    {
        corrupt_environment(node, mutation);
        return true;
    }
    if mutation == "payload_binding" && node["kind"] == "pat_shared_copy" {
        node["children"][0]["binding"] = "forged:payload".into();
        return true;
    }
    node["children"]
        .as_array_mut()
        .is_some_and(|children| children.iter_mut().any(|child| corrupt(child, mutation)))
}

fn corrupt_environment(node: &mut Value, mutation: &str) {
    let mut environment: Value =
        serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
    match mutation {
        "capture_mode" => environment["captures"][0]["mode"] = "Value".into(),
        "missing_shared_formal" => {
            environment.as_object_mut().unwrap().remove("shared_formal");
        }
        "projection_type" => {
            environment["shared_formal"]["projections"][0]["after"] =
                environment["shared_formal"]["projections"][0]["before"].clone();
        }
        "capture_root" => {
            let old = environment["captures"][0]["root"]["local"]
                .as_u64()
                .unwrap();
            environment["captures"][0]["root"]["local"] = (old + 1).into();
        }
        "tuple_storage" => {
            environment["stored_type"] = environment["shared_formal"]["root_type"].clone()
        }
        _ => unreachable!(),
    }
    node["res"]["description"] = serde_json::to_string(&environment).unwrap().into();
}

fn transition_callback_guards_reads_and_historical_policy_are_independently_required(
    report: &Value,
) {
    let entry = function(report, "Transition::matches");
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in [
        "some_outcome",
        "payload_reads",
        "false_path_payload",
        "false_path_call",
        "historical",
    ] {
        let original = unit.pair(index).unwrap();
        let mut graph = serde_json::json!({"pdg": original.pdg, "cfg": original.cfg});
        corrupt_graph(&mut graph, mutation);
        let pair = rstyle_pdg::producer::Pair {
            pdg: serde_json::from_value(graph["pdg"].clone()).unwrap(),
            cfg: serde_json::from_value(graph["cfg"].clone()).unwrap(),
        };
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{mutation}"
        );

        let mut bundle = shared_evidence::wire(entry);
        let root = bundle["roots"][index].as_u64().unwrap() as usize;
        corrupt_graph(&mut bundle["graphs"][root], mutation);
        reject_resealed(bundle, mutation);
    }
}

fn corrupt_graph(graph: &mut Value, mutation: &str) {
    if mutation == "historical" {
        graph["pdg"]["profile"]["version"] = "3.70.0".into();
        graph["cfg"]["profile"]["version"] = "3.70.0".into();
        return;
    }
    if corrupt_graph_nodes(graph, mutation) {
        return;
    }
    if mutation == "false_path_payload" {
        let edge = graph["cfg"]["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|edge| edge["kind"] == "true")
            .unwrap();
        edge["kind"] = "false".into();
        return;
    }
    if mutation == "false_path_call" {
        let nodes = graph["pdg"]["nodes"].as_array_mut().unwrap();
        let call = nodes
            .iter_mut()
            .find(|node| {
                node["category"] == "call" && node["attributes"]["construct"] != "data.option-get"
            })
            .unwrap();
        let call_id = call["id"].clone();
        let target = graph["pdg"]["extensions"]["compat.rust.hir-r0:operation-map"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["pdg_node_id"] == call_id)
            .unwrap()["cfg_node_id"]
            .clone();
        let edge = graph["cfg"]["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|edge| edge["kind"] == "false")
            .unwrap();
        edge["target"] = target;
        return;
    }
    unreachable!();
}

fn corrupt_graph_nodes(graph: &mut Value, mutation: &str) -> bool {
    let nodes = graph["pdg"]["nodes"].as_array_mut().unwrap();
    if mutation == "some_outcome" {
        let selector = nodes
            .iter_mut()
            .find(|node| node["attributes"]["construct"] == "callback.helper-option-some")
            .unwrap();
        assert_eq!(
            selector["attributes"]["outcomes"],
            serde_json::json!(["true", "false"])
        );
        selector["attributes"]["outcomes"] = serde_json::json!(["false"]);
        return true;
    }
    let payload = nodes
        .iter_mut()
        .find(|node| node["attributes"]["construct"] == "data.option-payload")
        .unwrap();
    if mutation == "payload_reads" {
        assert!(
            !payload["attributes"]["reads"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        payload["attributes"]["reads"] = serde_json::json!([]);
        return true;
    }
    false
}
