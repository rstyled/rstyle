use super::super::style_payload::{documents, snapshot};
use super::default_discovery::{dismiss, json, raw, saved};
use super::mining_options::{checked, remove_sources};
use super::*;

const BODY_CONSUMERS: &[&[&str]] = &[
    &[
        "--extraction-profile",
        "hir-data-v3",
        "--report-detail",
        "compact",
    ],
    &[
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory-only",
        "--compiler-patterns",
        "--report-detail",
        "full",
    ],
    &[
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory-only",
        "--compiler-patterns",
        "--source-rules",
        "--report-detail",
        "compact",
    ],
];

const FULL: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--compiler-advisory-only",
    "--compiler-patterns",
    "--compiler-pattern-scope",
    "authored",
    "--pattern-ledger",
    "rstyle-ignore.toml",
    "--report-detail",
    "compact",
];

fn metadata_contract(report: &Value, documents: &[rstyle_core::cpg::model::Document]) {
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    assert_eq!(proposals(report)["source_scope"]["kind"], "authored");
    let plan = Path::new(report["run_directory"].as_str().unwrap()).join("plan.json");
    let plan: Value = serde_json::from_slice(&std::fs::read(plan).unwrap()).unwrap();
    assert_eq!(plan["compiler_patterns"], true);
    assert_eq!(plan["style_inputs_only"], true);
    let functions = documents.iter().flat_map(|document| {
        document
            .unit
            .as_ref()
            .unwrap()
            .functions
            .as_deref()
            .unwrap()
    });
    assert_eq!(functions.count(), 10);
}

/// The graph the core derives from the documents is the one a full capture records.
fn original_graphs(documents: &[rstyle_core::cpg::model::Document], full: &Value) {
    use rstyle_core::cpg::records::{Demand, records};
    let bodies = rstyle_pdg::unit_store::read_diagnostic_snapshot_phase(
        &serde_json::to_vec(full).unwrap(),
        "compiler-advisory-only",
    )
    .unwrap();
    let demand = Demand {
        graphs: true,
        bounds: false,
    };
    let read: Vec<_> = documents
        .iter()
        .flat_map(|document| records(document, demand).unwrap())
        .collect();
    assert_eq!(read.len(), bodies.len());
    for record in &read {
        let original = bodies.iter().find(|fact| fact.name == record.name).unwrap();
        let key = rstyle_core::advisory::patterns::KEY;
        let recorded = original.build.frontend_options.get(key).unwrap();
        let graph = serde_json::to_value(record.graph.as_ref().unwrap()).unwrap();
        assert!(!graph["nodes"].as_array().unwrap().is_empty());
        assert_eq!(graph, recorded["graph"]);
        assert_eq!(record.source, original.source.bytes.as_str());
        assert_eq!(record.located.start as u32, original.fn_span.start);
        assert_eq!(record.located.end as u32, original.fn_span.end);
    }
}

fn proposal_parity(metadata: &Value, full: &Value) {
    assert_eq!(metadata["capture_failures"], serde_json::json!([]));
    assert_eq!(metadata["handoff_failures"], serde_json::json!([]));
    assert_eq!(full["capture_failures"], serde_json::json!([]));
    assert_eq!(full["handoff_failures"], serde_json::json!([]));
    assert_eq!(proposals(metadata)["input_complete"], true);
    assert_eq!(proposals(metadata), proposals(full));
    let evaluation = &proposals(metadata)["evaluation"];
    for (name, kind) in [
        ("dogs", "capability_set"),
        ("inspect_dog", "generic_fn"),
        ("tag_left", "parameterize"),
    ] {
        assert_eq!(candidate_at(evaluation, name).unwrap()["kind"], kind);
    }
    assert!(metadata["unit_contexts"].is_array());
    assert_eq!(metadata["unit_contexts"], full["unit_contexts"]);
    let builds = metadata["build_contexts"].as_array().unwrap();
    let originals = full["build_contexts"].as_array().unwrap();
    assert_eq!(builds.len(), originals.len());
    for (header, body) in builds.iter().zip(originals) {
        assert_eq!(header["builds"], body["builds"]);
    }
    let observed = metadata["observed_units"].as_array().unwrap();
    let original = full["observed_units"].as_array().unwrap();
    assert_eq!(observed.len(), original.len());
    for (header, body) in observed.iter().zip(original) {
        for field in ["target", "test", "extraction_profile"] {
            assert_eq!(header[field], body[field], "{field}");
        }
        assert_eq!(header["style_inputs_only"], true);
        assert_eq!(body["style_inputs_only"], false);
    }
}

fn offline_reviews(directory: &Path, metadata: &Value, full: &Value) {
    let output = saved(directory, metadata, "patterns", &["--format", "json"]);
    checked(&output, 0);
    let restored = json(&output);
    assert_eq!(restored["compiler_advisory"], metadata["compiler_advisory"]);
    assert_eq!(restored["capture_failures"], metadata["capture_failures"]);
    assert_eq!(restored["handoff_failures"], metadata["handoff_failures"]);
    let bytes = dismiss(directory, metadata);
    for original in [metadata, full] {
        let output = saved(directory, original, "patterns", &["--format", "json"]);
        checked(&output, 0);
        let hidden = json(&output);
        assert_eq!(proposals(&hidden)["evaluation"]["ignored_count"], 1);
        assert_eq!(
            proposals(&hidden)["evaluation"]["candidates"],
            proposals(metadata)["evaluation"]["candidates"]
        );
    }
    assert_eq!(
        std::fs::read(directory.join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
}

fn body_consumers_unavailable(directory: &Path, metadata: &Value) {
    for flags in BODY_CONSUMERS {
        let mut options = vec!["--format", "json"];
        options.extend_from_slice(flags);
        let output = saved(directory, metadata, "clones", &options);
        checked(&output, 2);
        let text = format!(
            "{}\n{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            [
                "style-facts",
                "compiler-style-inputs",
                "style inputs",
                "style-only",
                "full body",
                "original capture mode"
            ]
            .iter()
            .any(|reason| text.contains(reason)),
            "missing unavailable-payload explanation: {text}"
        );
    }
}

#[test]
fn default_pattern_metadata_preserves_original_graphs_proposals_and_offline_reviews() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), FAMILIES).unwrap();
    let output = raw(directory.path(), &[]);
    checked(&output, 0);
    let metadata = json(&output);
    let documents = documents(&metadata);
    metadata_contract(&metadata, &documents);
    let (output, full) = clones(directory.path(), FULL);
    checked(&output, 0);
    let full_wire = snapshot(&full);
    assert_eq!(full_wire["schema"], "rstyle.diagnostic.raw-facts/5");
    proposal_parity(&metadata, &full);
    original_graphs(&documents, &full_wire);
    remove_sources(directory.path());
    offline_reviews(directory.path(), &metadata, &full);
    body_consumers_unavailable(directory.path(), &metadata);
}
