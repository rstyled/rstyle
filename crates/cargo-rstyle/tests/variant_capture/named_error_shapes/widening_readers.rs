//! Complete original widening bodies compose before any admission expectation.
use super::*;
use rstyle_pdg::unit_store::{EvidenceFacts, UnitAnalysis};
use serde_json::json;

const NAME: &str = "try_read_u32_as_usize";
const FORWARD: &str = "pub fn forward_widening(slice: &[u8], what: &'static str) -> Result<(usize, usize), DeserializeError> { try_read_u32_as_usize(slice, what) }";
const INSTANCE: &str = "rstyle.rust.call-instances";
const ORIGINS: &str = "rstyle.data:widening-reader-origins";
const OUTER: &str = "/body/children/0/children/1";
const SUCCESS: &str = "/body/children/0/children/1/children/1/children/0/children/0/children/1";
const FAILURE: &str = "/body/children/0/children/1/children/1/children/0/children/1";

fn selection() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../eval/coverage-development/wire-usize-readers/selection.json"
    ))
    .unwrap()
}

fn selected<'a>(selection: &'a Value, name: &str) -> &'a str {
    selection["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == format!("util::wire::{name}"))
        .unwrap()["source"]
        .as_str()
        .unwrap()
}

const MUTATIONS: &[&str] = &[
    "capture-root",
    "capture-mode",
    "capture-storage",
    "capture-projection",
    "rust-call-input",
    "rust-call-output",
    "closure-definition",
    "callback-body",
    "parent-owner",
    "missing-call",
    "nested-call",
    "conversion-substitution",
    "conversion-definition",
    "method-substitution",
    "method-closure",
    "result-type",
];

fn source() -> String {
    let selected_sources = selection();
    let original = selected(&selected_sources, NAME);
    let constructors = ["buffer_too_small", "invalid_usize"]
        .map(|name| selected(&selected_sources, &format!("DeserializeError::{name}")));
    let prelude = format!(
        "use core::mem::size_of;\nimpl DeserializeError {{ {} {} }}",
        constructors[0], constructors[1]
    );
    let dependencies = ["check_slice_len", "read_u32", "try_read_u32", NAME]
        .map(|name| selected(&selected_sources, name).to_owned());
    let deferred = ["read_u16", "try_read_u16", "try_read_u16_as_usize"]
        .map(|name| selected(&selected_sources, name).to_owned());
    [schema(), prelude]
        .into_iter()
        .chain(dependencies)
        .chain([original.replace(NAME, "same_widening")])
        .chain([FORWARD.to_owned()])
        .chain(source_negatives(original))
        .chain(deferred)
        .collect::<Vec<_>>()
        .join("\n")
}

fn schema() -> String {
    // Keep the existing complete mixed error owner, including both generated
    // unused variants. Every selected reader/dependency body is byte-exact.
    SOURCE
        .split_once("pub fn arithmetic_overflow")
        .unwrap()
        .0
        .replace("ErrorKind", "DeserializeErrorKind")
        .replace("pub struct Error(", "pub struct DeserializeError(")
}

fn source_negatives(original: &str) -> Vec<String> {
    [
        ("wrong_count", "map(|n| (n, nr))", "map(|n| (n, 0))"),
        ("reversed", "map(|n| (n, nr))", "map(|n| (nr, n))"),
        (
            "wrong_message",
            "invalid_usize(what)",
            "invalid_usize(\"other\")",
        ),
        (
            "extra_work",
            "map(|n| (n, nr))",
            "map(|n| { let unused = 0usize; (n, nr) })",
        ),
        (
            "explicit_return",
            "map(|n| (n, nr))",
            "map(|n| { return (n, nr); })",
        ),
    ]
    .map(|(name, before, after)| original.replace(NAME, name).replace(before, after))
    .into_iter()
    .collect()
}

fn mutation_name(change: &str) -> String {
    format!("attack_{}", change.replace('-', "_"))
}

fn source_with_mutants() -> String {
    let selection = selection();
    let original = selected(&selection, NAME);
    std::iter::once(source())
        .chain(
            MUTATIONS
                .iter()
                .map(|change| original.replace(NAME, &mutation_name(change))),
        )
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn original_u32_widening_composes_saved_payloads_errors_and_scoped_callbacks() {
    // Independent contract: lengths0..3 return BufferTooSmall before reading;
    // otherwise native first-four-byte u32 widens exactly and returns count4.
    // Both errors preserve the original static message root. Neither tuple
    // reordering/count changes nor extra callback work belongs to this family.
    // The u16 blanket conversion has separate selection and never-error proof.
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}; handoff: {}",
        String::from_utf8_lossy(&output.stderr),
        report["handoff_failures"]
    );
    assert_frontier(&report);
    let store = projected_store(&report);
    let analysis = store.analysis();
    strict_replay(&store, &analysis, &report);
    reject_graph(
        &store,
        &analysis,
        shared_evidence::index(function(&report, NAME)),
    );
    reject_resealed_frontends();
    reject_source(function(&report, NAME));
}

fn assert_frontier(report: &Value) {
    for name in [
        "read_u32",
        "try_read_u32",
        NAME,
        "same_widening",
        "try_read_u16_as_usize",
    ] {
        let entry = function(report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    for name in [
        "wrong_count",
        "reversed",
        "wrong_message",
        "extra_work",
        "explicit_return",
    ] {
        let entry = function(report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
    // The callee publishes a verified guarded summary. Whole-result containment
    // in this exploratory caller still needs helper_references to represent
    // references inside Result error payloads; publication alone is insufficient.
    let caller = function(report, "forward_widening");
    assert_eq!(caller["outcome"], "excluded");
    assert!(
        caller["reasons"].as_array().unwrap().iter().any(|reason| {
            reason.as_str().is_some_and(|reason| {
                reason.contains("Missing verified shared helper output summary")
            })
        }),
        "{}",
        caller["reasons"]
    );
}

fn projected_store(report: &Value) -> rstyle_pdg::unit_store::UnitStore {
    let (raw, _) = shared_evidence::load(function(report, NAME));
    let originals = raw.functions().to_vec();
    let store = raw.with_slice_length_instances().unwrap();
    assert_eq!(store.functions(), originals);
    store
}

fn strict_replay(
    store: &rstyle_pdg::unit_store::UnitStore,
    analysis: &UnitAnalysis<'_>,
    report: &Value,
) {
    rstyle_pdg::verification_policy::strict(|| {
        for name in ["try_read_u16", "try_read_u32"] {
            let index = shared_evidence::index(function(report, name));
            assert_reader_source_reuse(store, analysis, index);
        }
        let imports: Vec<_> = [NAME, "same_widening"]
            .map(|name| {
                let index = shared_evidence::index(function(report, name));
                let facts = analysis.function(index).unwrap();
                assert_authority_reuse(store, &facts, index);
                let pair = store.pair(index).unwrap();
                rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts)
                    .unwrap();
                rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
                rstyle_ccgraph::admit_produced(pair, &facts).unwrap()
            })
            .into_iter()
            .collect();
        let matching =
            rstyle_ccgraph::evaluate_all(&imports, &rstyle_ccgraph::CcgraphConfig::default());
        assert!(!matching.incomplete, "{:?}", matching.diagnostics);
        assert_eq!(matching.pairs.len(), 1);
        assert_eq!(
            matching.pairs[0].decision,
            rstyle_ccgraph::Decision::Accepted
        );
        assert_eq!(matching.pairs[0].displayed_score.as_deref(), Some("1"));
    });
}

fn assert_authority_reuse(
    store: &rstyle_pdg::unit_store::UnitStore,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
    index: usize,
) {
    let first = facts.widening_reader_authority().unwrap();
    let repeated = facts.widening_reader_authority().unwrap();
    assert!(matches!(first, std::borrow::Cow::Borrowed(_)));
    assert!(std::ptr::eq(first.as_ref(), repeated.as_ref()));
    rstyle_pdg::verification_policy::generated(|| {
        let generated = facts.widening_reader_authority().unwrap();
        assert!(!std::ptr::eq(first.as_ref(), generated.as_ref()));
    });
    let repeated = facts.widening_reader_authority().unwrap();
    assert!(std::ptr::eq(first.as_ref(), repeated.as_ref()));
    let other = store.analysis();
    let other_facts = other.function(index).unwrap();
    let other_proof = other_facts.widening_reader_authority().unwrap();
    assert!(!std::ptr::eq(first.as_ref(), other_proof.as_ref()));
}

fn assert_reader_source_reuse(
    store: &rstyle_pdg::unit_store::UnitStore,
    analysis: &UnitAnalysis<'_>,
    index: usize,
) {
    let facts = analysis.function(index).unwrap();
    let first = facts.guarded_reader_source().unwrap().unwrap();
    let repeated = facts.guarded_reader_source().unwrap().unwrap();
    assert!(matches!(first, std::borrow::Cow::Borrowed(_)));
    assert!(std::ptr::eq(first.as_ref(), repeated.as_ref()));
    rstyle_pdg::verification_policy::generated(|| {
        let generated = facts.guarded_reader_source().unwrap().unwrap();
        assert!(!std::ptr::eq(first.as_ref(), generated.as_ref()));
    });
    let other = store.analysis();
    let other_facts = other.function(index).unwrap();
    let other_proof = other_facts.guarded_reader_source().unwrap().unwrap();
    assert!(!std::ptr::eq(first.as_ref(), other_proof.as_ref()));
}

fn diagnostic_mutants() -> rstyle_pdg::unit_store::UnitStore {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source_with_mutants()).unwrap();
    let snapshots = tempfile::tempdir().unwrap();
    let output = diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle"))
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
            ])
            .current_dir(directory.path())
            .env("RSTYLE_DRIVER", driver())
            .env("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY", "1")
            .env("RSTYLE_DIAGNOSTIC_SHARED_SOURCES", "1")
            .env("RSTYLE_DIAGNOSTIC_FACTS_DIR", snapshots.path())
            .env_remove("CARGO_TARGET_DIR"),
    )
    .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert!(
        report["functions"].as_array().unwrap().is_empty(),
        "{report}"
    );
    let files: Vec<_> = std::fs::read_dir(snapshots.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.to_string_lossy()
                .ends_with("-pre-helper-certification.json")
        })
        .collect();
    assert_eq!(
        files.len(),
        1,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let facts =
        rstyle_pdg::unit_store::read_diagnostic_snapshot(&std::fs::read(&files[0]).unwrap())
            .unwrap();
    rstyle_pdg::unit_store::UnitStore::capture(facts).unwrap()
}

fn wire_index(bundle: &Value, name: &str) -> usize {
    let indices: Vec<_> = bundle["functions"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter_map(|(index, function)| (function["name"] == name).then_some(index))
        .collect();
    assert_eq!(indices.len(), 1, "unique captured identity: {name}");
    indices[0]
}

fn reject_resealed_frontends() {
    let raw = diagnostic_mutants();
    let mut bytes = Vec::new();
    raw.write_bundle(&mut bytes).unwrap();
    let mut bundle: Value = serde_json::from_slice(&bytes).unwrap();
    let selection = selection();
    let original = selected(&selection, NAME);
    let controls = [NAME, "same_widening"].map(|name| wire_index(&bundle, name));
    let mut mutants = Vec::new();
    for &change in MUTATIONS {
        let name = mutation_name(change);
        let index = wire_index(&bundle, &name);
        // Raw diagnostics supply no admission authority. Instead independently
        // establish byte-exact original source and distinct compiler identities.
        assert_eq!(
            raw.functions()[index].source.bytes,
            original.replace(NAME, &name)
        );
        assert_ne!(
            raw.functions()[index].body.hir,
            raw.functions()[controls[0]].body.hir
        );
        assert!(
            bundle["roots"][index].is_null(),
            "unexpected raw root: {change}"
        );
        mutate_frontend(&mut bundle["functions"][index], change);
        mutants.push((change, index));
    }
    let raw = import_resealed(bundle).unwrap();
    let store = raw.with_slice_length_instances().unwrap();
    check_frontend_replay(&store, controls, mutants);
}

fn check_frontend_replay(
    store: &rstyle_pdg::unit_store::UnitStore,
    controls: [usize; 2],
    mutants: Vec<(&str, usize)>,
) {
    let analysis = store.analysis();
    rstyle_pdg::verification_policy::strict(|| {
        for index in controls {
            let facts = analysis.function(index).unwrap();
            let pair = store
                .pair(index)
                .expect("batch must retain admitted control");
            rstyle_pdg::producer::verify(pair, &facts).unwrap();
            rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
        }
        for (change, index) in mutants {
            let facts = analysis.function(index).unwrap();
            assert_ne!(
                rstyle_pdg::classify(&facts).status,
                rstyle_pdg::EligibilityVerdict::Eligible,
                "independent mutation: {change}"
            );
            assert!(
                store.pair(index).is_none(),
                "mutant acquired a pair: {change}"
            );
        }
    });
}

fn mutate_frontend(facts: &mut Value, change: &str) {
    match change {
        "capture-root" | "capture-mode" | "capture-storage" | "capture-projection"
        | "rust-call-input" | "rust-call-output" | "closure-definition" => {
            mutate_environment(facts, change);
        }
        "result-type" => facts["signature"]["return_ty"]["normalized"] = "usize".into(),
        _ => mutate_inventory(&mut facts["build"]["frontend_options"][INSTANCE], change),
    }
}

fn mutate_environment(facts: &mut Value, change: &str) {
    let path = match change {
        "capture-root" | "rust-call-input" => SUCCESS,
        "rust-call-output" => FAILURE,
        _ => OUTER,
    };
    let node = facts.pointer_mut(path).unwrap();
    let mut environment: Value =
        serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
    match change {
        "capture-root" => environment["captures"][0]["root"]["local"] = 999999.into(),
        "capture-mode" => environment["captures"][0]["mode"] = "Value".into(),
        "capture-storage" => {
            environment["stored_type"] =
                json!({"status":"Captured", "ty":{"kind":"Tuple","evidence":[]}})
        }
        "capture-projection" => {
            environment["captures"][0]["projections"] = json!([{
                "kind":"deref", "before":{"status":"Unsupported","reason":"forged"},
                "after":{"status":"Unsupported","reason":"forged"}
            }])
        }
        "rust-call-input" => {
            environment["signature_inputs"][0] =
                json!({"status":"Captured","ty":{"kind":"Scalar","evidence":"u32"}})
        }
        "rust-call-output" => {
            environment["signature_output"] =
                json!({"status":"Captured","ty":{"kind":"Scalar","evidence":"usize"}})
        }
        "closure-definition" => environment["definition_identity"] = "forged".into(),
        _ => unreachable!(),
    }
    node["res"]["description"] = serde_json::to_string(&environment).unwrap().into();
}

fn mutate_inventory(inventory: &mut Value, change: &str) {
    let outer = &mut inventory["callbacks"][0];
    match change {
        "callback-body" => outer["body"]["local"] = 999999.into(),
        "parent-owner" => outer["callbacks"][0]["parent_callback"]["local"] = 999999.into(),
        "missing-call" => {
            outer["calls"].as_array_mut().unwrap().pop();
        }
        "nested-call" => outer["callbacks"][0]["calls"] = json!([outer["calls"][0].clone()]),
        "conversion-substitution" | "conversion-definition" => {
            mutate_conversion(outer, change);
        }
        "method-substitution" | "method-closure" => mutate_method(outer, change),
        _ => unreachable!(),
    }
}

fn mutate_conversion(outer: &mut Value, change: &str) {
    let call = outer["calls"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|call| {
            call["requested"]["path"]
                .as_str()
                .unwrap()
                .ends_with("TryFrom::try_from")
        })
        .unwrap();
    if change == "conversion-substitution" {
        call["requested_arguments"][1]["identity"]["name"] = "u64".into();
    } else {
        call["selected"]["definition_identity"] = "forged".into();
    }
}

fn mutate_method(outer: &mut Value, change: &str) {
    let call = outer["calls"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|call| {
            call["requested"]["path"]
                .as_str()
                .unwrap()
                .ends_with("::map")
        })
        .unwrap();
    if change == "method-substitution" {
        call["requested_arguments"][0]["identity"]["name"] = "u64".into();
    } else {
        call["requested_arguments"][3]["identity_status"] = "known".into();
    }
    call["selected_arguments"] = call["requested_arguments"].clone();
}

fn import_resealed(bundle: Value) -> Result<rstyle_pdg::unit_store::UnitStore, String> {
    let bytes = serde_json::to_vec(&bundle).unwrap();
    let seal = rstyle_pdg::unit_store::BundleManifest {
        schema: "rstyle.shared-unit.manifest/2".into(),
        checksum: rstyle_pdg::unit_store::transport_checksum(&bytes),
        bytes: bytes.len() as u64,
    };
    let verified = seal.verify_owned(bytes).unwrap();
    rstyle_pdg::unit_store::UnitStore::from_bundle(verified.bytes())
}

fn reject_source(entry: &Value) {
    let mut bundle = shared_evidence::wire(entry);
    let index = shared_evidence::index(entry);
    mutate_source(&mut bundle, index);
    match import_resealed(bundle).and_then(|raw| raw.with_slice_length_instances()) {
        Ok(store) => {
            let analysis = store.analysis();
            let facts = analysis.function(index).unwrap();
            assert_ne!(
                rstyle_pdg::classify(&facts).status,
                rstyle_pdg::EligibilityVerdict::Eligible,
                "source mutation"
            );
        }
        Err(error) => assert!(!error.is_empty()),
    }
}

fn reject_graph(
    store: &rstyle_pdg::unit_store::UnitStore,
    analysis: &UnitAnalysis<'_>,
    index: usize,
) {
    let facts = analysis.function(index).unwrap();
    let original = store.pair(index).unwrap();
    for change in [
        "origin",
        "read",
        "operand",
        "guard",
        "order",
        "entry",
        "exit",
        "historical",
    ] {
        let mut pair = original.clone();
        mutate_pair(&mut pair, change);
        let serialized = pair.serialize().unwrap();
        rstyle_pdg::verification_policy::strict(|| {
            assert!(
                rstyle_pdg::producer::verify_serialized(&serialized, &facts).is_err(),
                "{change}"
            );
            assert!(
                rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
                "{change}"
            );
        });
    }
}

fn mutate_pair(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    match change {
        "guard" => reverse_guard(pair),
        "order" => reorder_capture(pair),
        "entry" | "exit" => bypass_boundary(pair, change),
        "historical" => {
            pair.pdg.profile.version = "3.126.0".into();
            pair.cfg.profile.version = "3.126.0".into();
        }
        _ => {
            let mut pdg = serde_json::to_value(&pair.pdg).unwrap();
            mutate_graph(&mut pdg, change);
            pair.pdg = serde_json::from_value(pdg).unwrap();
        }
    }
}

fn bypass_boundary(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    if change == "entry" {
        let selector = cfg_construct(pair, "outer-is-ok");
        let outgoing: Vec<_> = pair
            .cfg
            .edges
            .iter_mut()
            .filter(|edge| edge.source == pair.cfg.entry_node_id)
            .collect();
        assert_eq!(outgoing.len(), 1);
        outgoing.into_iter().next().unwrap().target = selector;
    } else {
        let [exit] = pair.cfg.exit_node_ids.as_slice() else {
            panic!("expected one exit")
        };
        let incoming: Vec<_> = pair
            .cfg
            .edges
            .iter_mut()
            .filter(|edge| edge.target == *exit)
            .collect();
        assert_eq!(incoming.len(), 1);
        incoming.into_iter().next().unwrap().kind = rstyle_pdg::CfgEdgeKind::False;
    }
}

fn cfg_construct(pair: &rstyle_pdg::producer::Pair, suffix: &str) -> String {
    let construct = format!("data.widening-reader.{suffix}");
    let node = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some(construct.as_str()))
        .unwrap();
    rstyle_pdg::read_operation_map(&pair.pdg.extensions)
        .unwrap()
        .iter()
        .find(|entry| entry.pdg_node_id == node.id)
        .unwrap()
        .cfg_node_id
        .clone()
}

fn reverse_guard(pair: &mut rstyle_pdg::producer::Pair) {
    let selector = cfg_construct(pair, "outer-is-ok");
    let mut changed = 0;
    for edge in &mut pair.cfg.edges {
        if edge.source == selector {
            edge.kind = match edge.kind {
                rstyle_pdg::CfgEdgeKind::True => rstyle_pdg::CfgEdgeKind::False,
                rstyle_pdg::CfgEdgeKind::False => rstyle_pdg::CfgEdgeKind::True,
                _ => continue,
            };
            changed += 1;
        }
    }
    assert_eq!(changed, 2);
}

fn normal_predecessor(pair: &rstyle_pdg::producer::Pair, target: &str) -> String {
    let incoming: Vec<_> = pair
        .cfg
        .edges
        .iter()
        .filter(|edge| edge.kind == rstyle_pdg::CfgEdgeKind::Normal && edge.target == target)
        .collect();
    let [edge] = incoming.as_slice() else {
        panic!("ambiguous normal predecessor of {target}")
    };
    edge.source.clone()
}

fn reorder_capture(pair: &mut rstyle_pdg::producer::Pair) {
    let environment = cfg_construct(pair, "outer-environment");
    let selector = cfg_construct(pair, "outer-is-ok");
    let reader = normal_predecessor(pair, &environment);
    let message = normal_predecessor(pair, &reader);
    let mut changed = 0;
    for edge in &mut pair.cfg.edges {
        if edge.kind != rstyle_pdg::CfgEdgeKind::Normal {
            continue;
        }
        let target = if edge.source == message {
            &environment
        } else if edge.source == environment {
            &reader
        } else if edge.source == reader {
            &selector
        } else {
            continue;
        };
        edge.target = target.clone();
        changed += 1;
    }
    assert_eq!(changed, 3);
}

fn mutate_graph(pdg: &mut Value, change: &str) {
    if change == "operand" {
        let edge = pdg["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|edge| edge["subkind"] == "value_operand" && edge["operand_position"] == 1)
            .unwrap();
        edge["operand_position"] = 0.into();
        return;
    }
    let nodes = pdg["nodes"].as_array_mut().unwrap();
    if change == "origin" {
        nodes
            .iter_mut()
            .find(|node| node["annotations"].get(ORIGINS).is_some())
            .unwrap()["annotations"]
            .as_object_mut()
            .unwrap()
            .remove(ORIGINS);
    } else {
        nodes
            .iter_mut()
            .find(|node| {
                node["attributes"]["reads"]
                    .as_array()
                    .is_some_and(|reads| reads.iter().any(|read| read["location_id"].is_string()))
            })
            .unwrap()["attributes"]["reads"] = json!([]);
    }
}

fn mutate_source(bundle: &mut Value, index: usize) {
    let reference = &bundle["functions"][index]["source"];
    let snapshot = reference["snapshot"].as_u64().unwrap() as usize;
    let start = reference["range"]["start"].as_u64().unwrap() as usize;
    let end = reference["range"]["end"].as_u64().unwrap() as usize;
    let original = bundle["sources"][snapshot].as_str().unwrap();
    let changed = original[start..end].replace("map(|n| (n, nr))", "map(|n| (nr, n))");
    assert_ne!(&original[start..end], changed);
    let digest = rstyle_pdg::source_identity::checksum(changed.as_bytes());
    let mut snapshot_bytes = original.to_string();
    snapshot_bytes.replace_range(start..end, &changed);
    bundle["sources"][snapshot] = snapshot_bytes.into();
    bundle["functions"][index]["source"]["digest"] = digest.clone().into();
    bundle["functions"][index]["build"]["frontend_options"]["source_correspondence"]["source_digest"] =
        digest.into();
}

#[test]
fn widening_reader_does_not_hide_cleanup_in_an_unused_error_variant() {
    let directory = clone_workspace();
    let source = source()
        + "\nmod hidden { use crate::FirstError as Alias; impl Drop for Alias { fn drop(&mut self) {} } }";
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in [NAME, "same_widening"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
}

#[path = "u16_widening_readers.rs"]
mod u16_widening_readers;
