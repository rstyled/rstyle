//! Checked nominal construction composes constants, selected TryFrom and forwarding.
use super::float_kernels::function;
use super::*;

const SOURCE: &str = r#"
#![no_std]
#[derive(Copy, Clone)]
pub struct Index(u32);
pub struct LimitError { attempted: u64 }
impl Index {
    pub const LIMIT: Index = Index::make(core::i32::MAX as usize - 1);
    pub const SECOND: Index = Index::make(core::i32::MAX as usize - 2);
    pub const fn make(value: usize) -> Index { Index(value as u32) }
    pub fn value(&self) -> usize { self.0 as usize }
    pub fn new(value: usize) -> Result<Index, LimitError> { Index::try_from(value) }
}
impl TryFrom<usize> for Index {
    type Error = LimitError;
    fn try_from(value: usize) -> Result<Index, LimitError> {
        if value > Index::LIMIT.value() {
            return Err(LimitError { attempted: value as u64 });
        }
        Ok(Index::make(value))
    }
}
pub fn checked(value: usize) -> Result<Index, LimitError> { Index::new(value) }
pub fn same(value: usize) -> Result<Index, LimitError> { Index::new(value) }
pub fn limit() -> usize { Index::LIMIT.value() }
pub fn second() -> usize { Index::SECOND.value() }
"#;

const ORACLE: &str = r#"
#[test]
fn independent_checked_boundaries() {
    assert_eq!(limit(), 2_147_483_646);
    assert_eq!(second(), 2_147_483_645);
    for value in [0, 1, 2_147_483_645, 2_147_483_646, 2_147_483_647,
                  2_147_483_648, u32::MAX as usize, usize::MAX] {
        assert_eq!(Index::make(value).value(), (value as u32) as usize);
        for result in [Index::try_from(value), Index::new(value), checked(value), same(value)] {
            match result {
                Ok(index) => {
                    assert!(value <= 2_147_483_646);
                    assert_eq!(index.value(), value);
                }
                Err(error) => {
                    assert!(value > 2_147_483_646);
                    assert_eq!(error.attempted, value as u64);
                }
            }
        }
    }
}
"#;

#[test]
fn checked_index_construction_composes_through_selected_tryfrom() {
    let directory = clone_workspace();
    native_oracle(directory.path());
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    verify_chain(&report);
    verify_helper_chain(&report);
    verify_constants(&report);
    verify_pairs(&report);
    check_equivalent_matching(&report, ["checked", "same"]);
    reject_controls(&report);
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

fn verify_chain(report: &Value) {
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    for name in [
        "Index::make",
        "Index::value",
        "Index::new",
        "checked",
        "same",
        "limit",
        "second",
        "<Index as core::convert::TryFrom<usize>>::try_from",
    ] {
        let entry = function(report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    let selection = selected_tryfrom(function(report, "Index::new"));
    assert_eq!(selection["requested"]["crate"], "core");
    assert_eq!(
        selection["requested"]["path"],
        "core::convert::TryFrom::try_from"
    );
    let implementation = function(report, selection["selected"]["path"].as_str().unwrap());
    assert_eq!(
        implementation["facts"]["build"]["frontend_options"]["definition_identity"],
        selection["selected"]["definition_identity"]
    );
}

fn selected_tryfrom(entry: &Value) -> Value {
    let inventory = &entry["facts"]["build"]["frontend_options"]["rstyle.rust.call-instances"];
    assert_eq!(inventory["complete"], true);
    inventory["calls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|call| call["requested"]["path"] == "core::convert::TryFrom::try_from")
        .expect("compiler-selected TryFrom implementation")
        .clone()
}

fn verify_helper_chain(report: &Value) {
    for (caller, expected) in [
        ("checked", "Index::new"),
        ("same", "Index::new"),
        (
            "Index::new",
            "<Index as core::convert::TryFrom<usize>>::try_from",
        ),
        ("limit", "Index::value"),
        ("second", "Index::value"),
    ] {
        let entry = function(report, caller);
        let bundle = shared_evidence::wire(entry);
        let links = bundle["links"][shared_evidence::index(entry)]
            .as_array()
            .unwrap();
        assert_eq!(links.len(), 1, "{caller}");
        let callee = links[0]["function"].as_u64().unwrap() as usize;
        assert_eq!(bundle["functions"][callee]["name"], expected, "{caller}");
    }
}

fn verify_constants(report: &Value) {
    for (name, expected, offset) in [("limit", "2147483646", "1"), ("second", "2147483645", "2")] {
        let mut body = function(report, name)["facts"]["body"].clone();
        let node = resolution_mut(&mut body, "resolved_product_constant").unwrap();
        let record: Value =
            serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
        assert_eq!(record["field"]["value"], expected);
        assert_eq!(record["initializer"]["offset"], offset);
        assert_eq!(
            record["initializer"]["core_constant"]["definition"]["path"],
            "core::i32::MAX"
        );
        assert_eq!(
            record["initializer"]["core_constant"]["scalar"]["value"],
            "2147483647"
        );
        assert_eq!(record["constructor"]["definition"]["path"], "Index::make");
    }
}

fn verify_pairs(report: &Value) {
    let unit = super::word_classifiers::verify_admissions(report);
    let analysis = unit.analysis();
    for entry in report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["outcome"] == "admitted")
    {
        let index = shared_evidence::index(entry);
        let facts = analysis.function(index).unwrap();
        let pair = unit.pair(index).unwrap();
        assert_eq!(pair.pdg.profile.version, "3.106.0");
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts)
        })
        .unwrap();
    }
}

fn reject_controls(report: &Value) {
    let checked = function(report, "checked");
    for corruption in ["signature", "source", "unit_invocation", "unit_target"] {
        shared_evidence::reject_helper(checked, corruption);
    }
    let implementation = function(report, "<Index as core::convert::TryFrom<usize>>::try_from");
    for attack in ["downgrade", "error_tag", "error_operand"] {
        reject_graph(implementation, attack);
    }
    for attack in ["definition_hash", "crate", "path"] {
        reject_variant_identity(implementation, attack);
    }
    reject_selected_identity(function(report, "Index::new"));
}

fn reject_graph(entry: &Value, attack: &str) {
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut wire = serde_json::to_value(original).unwrap();
    corrupt_graph(&mut wire, attack);
    let forged: rstyle_pdg::producer::Pair = serde_json::from_value(wire).unwrap();
    let serialized = forged.serialize().unwrap();
    assert!(
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&serialized, &facts)
        })
        .is_err(),
        "{attack}"
    );
    assert!(
        rstyle_ccgraph::admit_produced(&forged, &facts).is_err(),
        "{attack}"
    );
}

fn corrupt_graph(pair: &mut Value, attack: &str) {
    if attack == "downgrade" {
        pair["pdg"]["profile"]["version"] = "3.97.0".into();
        pair["cfg"]["profile"]["version"] = "3.97.0".into();
        return;
    }
    let error = pair["pdg"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["attributes"]["construct"] == "data.result-err")
        .unwrap();
    if attack == "error_tag" {
        error["attributes"]["construct"] = "data.result-ok".into();
        error["annotations"]["rstyle.data:concrete-result"]["variant"] = "ok".into();
        return;
    }
    let target = error["id"].clone();
    let operand = pair["pdg"]["edges"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|edge| edge["subkind"] == "value_operand" && edge["target"] == target)
        .unwrap();
    operand["operand_position"] = 1.into();
}

fn reject_variant_identity(entry: &Value, field: &str) {
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut wire = serde_json::to_value(&*facts).unwrap();
    let node = resolution_mut(&mut wire["body"], "resolved_result_variant").unwrap();
    let mut identity: Value =
        serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
    identity[field] = "forged-core-identity".into();
    node["res"]["description"] = serde_json::to_string(&identity).unwrap().into();
    let changed: rstyle_pdg::HirFnFacts = serde_json::from_value(wire).unwrap();
    let serialized = unit.pair(index).unwrap().serialize().unwrap();
    assert!(
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&serialized, &changed)
        })
        .is_err(),
        "{field}"
    );
    shared_evidence::reject_facts(entry, &changed);
}

fn reject_selected_identity(entry: &Value) {
    shared_evidence::reject_edit(entry, |bundle, index| {
        let calls = bundle["functions"][index]["build"]["frontend_options"]
            ["rstyle.rust.call-instances"]["calls"].as_array_mut().unwrap();
        let call = calls
            .iter_mut()
            .find(|call| call["requested"]["path"] == "core::convert::TryFrom::try_from")
            .unwrap();
        call["selected"]["definition_identity"] = "forged-local-implementation".into();
    });
}

fn resolution_mut<'a>(node: &'a mut Value, kind: &str) -> Option<&'a mut Value> {
    if node["res"]["kind"] == kind {
        return Some(node);
    }
    node["children"]
        .as_array_mut()?
        .iter_mut()
        .find_map(|child| resolution_mut(child, kind))
}
