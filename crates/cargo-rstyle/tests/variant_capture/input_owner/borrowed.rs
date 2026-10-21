//! Slot reads and returned slice referents compose through complete owning scopes.
use super::*;

const CHAIN: &str = r#"
impl<'h> Input<'h> {
    pub fn boundary(&self, i: usize) -> bool { is_boundary(self.get_haystack(), i) }
}
pub fn is_boundary(bytes: &[u8], i: usize) -> bool {
    match bytes.get(i) {
        None => i == bytes.len(),
        Some(&b) => b <= 0x7f || b >= 0xc0,
    }
}
pub fn wrapper<'h>(input: &Input<'h>) -> &'h [u8] { input.get_haystack() }
pub fn saved<'h>(a: &Input<'h>, b: &Input<'h>) -> &'h [u8] {
    let mut selected = a;
    let view = selected.get_haystack(); selected = b; view
}
pub fn later(a: &Input<'_>, b: &Input<'_>, i: usize) -> bool {
    let mut selected = a;
    is_boundary(selected.get_haystack(), { selected = b; i })
}
pub fn choose<'h>(a: &Input<'h>, b: &Input<'h>, pick: bool) -> &'h [u8] {
    let selected = if pick { a } else { b }; selected.get_haystack()
}
pub fn nested<'h>(a: &Input<'h>) -> &'h [u8] { wrapper(a) }
pub fn indexed(a: &Input<'_>, i: usize) -> u8 { a.get_haystack()[i] }
pub fn indexed_caller(a: &Input<'_>, i: usize) -> u8 { indexed(a, i) }
pub fn nested_reference<'a, 'h>(a: &'a Input<'h>) -> &'a &'h [u8] { &a.haystack }
pub fn nested_reference_caller<'a, 'h>(a: &'a Input<'h>) -> &'a &'h [u8] { nested_reference(a) }
"#;

fn source() -> String {
    format!("{}{CHAIN}", passive::source())
}

#[test]
fn stored_slices_compose_with_checked_consumers_saved_receivers_and_finite_alternatives() {
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
        if Path::new(String::from_utf8_lossy(&library.stdout).trim()).is_dir() {
            check_context(target);
        }
    }
}

fn check_context(target: &str) {
    for no_std in [false, true] {
        let (_directory, report) = capture_context(&source(), no_std, Some(target));
        for name in [
            "Input::get_haystack",
            "Input::boundary",
            "is_boundary",
            "wrapper",
            "saved",
            "later",
            "choose",
            "nested",
            "indexed",
            "indexed_caller",
        ] {
            let entry = function(&report, name);
            assert_eq!(
                entry["outcome"], "admitted",
                "{target} {name}: {}",
                entry["reasons"]
            );
        }
        let negative = function(&report, "nested_reference_caller");
        assert_eq!(negative["outcome"], "excluded", "{}", negative["reasons"]);
        super::super::word_classifiers::verify_admissions(&report);
        check_locations(&report);
        for flag in [
            "capture_complete",
            "assessment_complete",
            "pair_enumeration_complete",
            "pair_evaluation_complete",
        ] {
            assert_eq!(report[flag], true, "{flag}");
        }
        assert_matching(&report, "wrapper", "nested", "Accepted");
    }
}

fn assert_matching(report: &Value, first: &str, second: &str, expected: &str) {
    let left = &function(report, first)["key"];
    let right = &function(report, second)["key"];
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (&pair["first"] == left && &pair["second"] == right)
                || (&pair["first"] == right && &pair["second"] == left)
        })
        .unwrap();
    assert_eq!(pair["decision"], expected);
}

fn check_locations(report: &Value) {
    let (unit, index) = data_unit(function(report, "Input::get_haystack"));
    let pair = unit.pair(index).unwrap();
    assert_eq!(pair.pdg.memory_locations.len(), 1);
    let slot = &pair.pdg.memory_locations[0].extensions[rstyle_pdg::data_documents::LOCATION_KEY];
    assert_eq!(slot["parameter"], 0);
    assert_eq!(slot["path"].as_array().unwrap().len(), 2);
    assert_eq!(slot["path"][0]["kind"], "SharedReferent");
    assert_eq!(slot["path"][1]["name"], "haystack");
    for name in ["saved", "later"] {
        let entry = function(report, name);
        let pair = unit.pair(shared_evidence::index(entry)).unwrap();
        if name == "saved" {
            let returned = pair
                .pdg
                .nodes
                .iter()
                .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
                .unwrap();
            let origins = &returned.annotations[rstyle_pdg::data_origins::KEY];
            assert_eq!(origins.as_array().unwrap().len(), 1);
            assert_eq!(origins[0]["parameter"], 0);
        }
        for location in &pair.pdg.memory_locations {
            let record = &location.extensions[rstyle_pdg::data_documents::LOCATION_KEY];
            assert_eq!(record["parameter"], 0, "{name}: {record}");
        }
    }
    let wire = shared_evidence::wire(function(report, "wrapper"));
    let helper = stored_helper(&wire);
    let output = &helper["summary"]["reference_outputs"][0];
    assert_eq!(output["parameter"], 0);
    assert_eq!(output["path"].as_array().unwrap().len(), 3);
    assert_eq!(output["path"][2]["kind"], "SharedReferent");
    assert_eq!(helper["summary"]["memory_reads"][0]["path"], slot["path"]);
}

fn stored_helper(bundle: &Value) -> &Value {
    bundle["helpers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|helper| helper["summary"]["identity"] == "rstyle.helper.stored-slice-result/1")
        .unwrap()
}

#[test]
fn stored_slice_summaries_reject_resealed_slot_referent_read_and_receiver_corruption() {
    let (_directory, report) = capture(&source(), false);
    let entry = function(&report, "later");
    assert_eq!(entry["outcome"], "admitted", "{}", entry["reasons"]);
    for mutation in [
        "slot_missing",
        "slot_referent",
        "output_slot",
        "output_extra",
        "field",
        "parameter",
        "identity",
        "effect",
    ] {
        let mut bundle = shared_evidence::wire(entry);
        let helper = bundle["helpers"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|helper| helper["summary"]["identity"] == "rstyle.helper.stored-slice-result/1")
            .unwrap();
        corrupt_summary(&mut helper["summary"], mutation);
        super::super::float_kernels::reject_resealed(bundle, mutation);
    }
    reject_conditional_read(entry);
    reject_document_edits(function(&report, "Input::get_haystack"));
}

fn reject_conditional_read(entry: &Value) {
    let mut bundle = shared_evidence::wire(entry);
    let helper = bundle["helpers"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|helper| {
            helper["summary"]["memory_reads"]
                .as_array()
                .is_some_and(|reads| {
                    reads.iter().any(|read| {
                        read["path"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|step| step["kind"] == "CheckedElement")
                    })
                })
        })
        .unwrap();
    helper["summary"]["memory_reads"]
        .as_array_mut()
        .unwrap()
        .retain(|read| {
            !read["path"]
                .as_array()
                .unwrap()
                .iter()
                .any(|step| step["kind"] == "CheckedElement")
        });
    super::super::float_kernels::reject_resealed(bundle, "missing conditional byte read");
}

fn corrupt_summary(summary: &mut Value, mutation: &str) {
    match mutation {
        "slot_missing" => summary["memory_reads"] = serde_json::json!([]),
        "slot_referent" => summary["memory_reads"][0]["path"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"kind":"SharedReferent"})),
        "output_slot" => {
            summary["reference_outputs"][0]["path"]
                .as_array_mut()
                .unwrap()
                .pop();
        }
        "output_extra" => summary["reference_outputs"][0]["path"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"kind":"SharedReferent"})),
        "field" => summary["reference_outputs"][0]["path"][1]["name"] = "span".into(),
        "parameter" => summary["reference_outputs"][0]["parameter"] = 1.into(),
        "identity" => summary["identity"] = "rstyle.helper.shared-return/1".into(),
        "effect" => summary["memory_effects"] = false.into(),
        _ => unreachable!(),
    }
}

fn reject_document_edits(entry: &Value) {
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["locations", "reads", "policy"] {
        let captured = unit.pair(index).unwrap();
        let mut pair = rstyle_pdg::producer::Pair {
            pdg: captured.pdg.clone(),
            cfg: captured.cfg.clone(),
        };
        match mutation {
            "locations" => pair.pdg.memory_locations.clear(),
            "reads" => {
                for node in &mut pair.pdg.nodes {
                    node.attributes.reads.clear();
                }
            }
            "policy" => {
                pair.pdg.profile.version = "3.68.0".into();
                pair.cfg.profile.version = "3.68.0".into();
            }
            _ => unreachable!(),
        }
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn distinct_same_typed_stored_fields_cannot_be_substituted_by_resealing() {
    let source = r#"
        pub struct Views<'h> { first: &'h [u8], second: &'h [u8] }
        impl<'h> Views<'h> {
            pub fn first(&self) -> &'h [u8] { self.first }
            pub fn second(&self) -> &'h [u8] { self.second }
        }
        pub fn first<'h>(v: &Views<'h>) -> &'h [u8] { v.first() }
        pub fn second<'h>(v: &Views<'h>) -> &'h [u8] { v.second() }
    "#;
    let (_directory, report) = capture(source, false);
    for entry in report["functions"].as_array().unwrap() {
        assert_eq!(entry["outcome"], "admitted", "{}", entry["reasons"]);
    }
    super::super::word_classifiers::verify_admissions(&report);
    let entry = exact_function(&report, "first");
    let mut bundle = shared_evidence::wire(entry);
    let helper = bundle["helpers"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|helper| helper["summary"]["reference_outputs"][0]["path"][1]["name"] == "first")
        .unwrap();
    for key in ["reference_outputs", "memory_reads"] {
        helper["summary"][key][0]["path"][1]["name"] = "second".into();
        helper["summary"][key][0]["path"][1]["position"] = 1.into();
    }
    super::super::float_kernels::reject_resealed(bundle, "different same-typed slot and referent");
}
