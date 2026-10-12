//! Real pinned rustc runs against independent source/eligibility expectations.

use rstyle_extraction_fixtures::fixture::FixtureCase;
use rstyle_pdg::{EligibilityVerdict, HirFnFacts, classify};
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static RUN: AtomicU64 = AtomicU64::new(0);

#[path = "support/oracle.rs"]
mod oracle;

fn directory() -> PathBuf {
    let serial = RUN.fetch_add(1, Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("rstyle-hir-{}-{serial}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("create isolated rustc run");

    directory
}

fn compile(source: &str) -> (Output, Option<Vec<HirFnFacts>>) {
    let directory = directory();
    let input = directory.join("input.rs");
    let facts = directory.join("facts.json");
    std::fs::write(&input, source).expect("write exact source");
    let output = invoke(&input, &facts);
    let parsed = std::fs::read(facts)
        .ok()
        .map(|bytes| serde_json::from_slice(&bytes).expect("read frontend records"));
    std::fs::remove_dir_all(directory).expect("remove isolated run");

    (output, parsed)
}

fn invoke(input: &std::path::Path, facts: &std::path::Path) -> Output {
    invocation(input, facts)
        .output()
        .expect("run pinned rustc driver")
}

fn invocation(input: &std::path::Path, facts: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rstyle-driver"));
    command
        .arg("rustc")
        .args([
            "--crate-name",
            "extraction_fixture",
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "--emit",
            "metadata",
            "-C",
            "overflow-checks=yes",
            "-C",
            "debug-assertions=yes",
            "-C",
            "panic=unwind",
            "-C",
            "opt-level=0",
            "-A",
            "warnings",
        ])
        .arg("-o")
        .arg(input.with_extension("rmeta"))
        .arg(input)
        .env("RSTYLE_HIR_FACTS_FILE", facts);

    command
}

fn auxiliary(case: &FixtureCase) -> &'static str {
    match case.id.as_str() {
        "unsupported-direct-call" => "\nfn g(a: i32) -> i32 { a }",
        "unsupported-call-after-return" => "\nfn g() -> i32 { 0 }",
        "unsupported-nested-item-owner" => "\nfn g() {}",
        _ => "",
    }
}

fn check_case(case: &FixtureCase) {
    let (output, facts) = compile(&format!("{}{}", case.source, auxiliary(case)));
    assert!(
        output.status.success(),
        "{}: {}",
        case.id,
        String::from_utf8_lossy(&output.stderr)
    );
    let facts = facts.expect("successful compiler must emit evidence");
    let function = facts.iter().find(|f| f.name == "f").expect("assessed f");
    assert_eq!(
        function.source.bytes, case.source,
        "{} exact bytes",
        case.id
    );
    assert_eq!(function.source.digest, case.source_identity());
    assert!(function.source_scan_complete, "{} source scan", case.id);
    let verdict = classify(function);
    let expected = if case.expected.eligibility == "ineligible" {
        EligibilityVerdict::Ineligible
    } else {
        EligibilityVerdict::Eligible
    };
    assert_eq!(
        verdict.status, expected,
        "{}: {:?}",
        case.id, verdict.diagnostics
    );

    check_diagnostics(case, &verdict.diagnostics);
    if expected == EligibilityVerdict::Eligible {
        check_lowering(case, function);
    }
}

fn check_lowering(case: &FixtureCase, facts: &HirFnFacts) {
    let graph = rstyle_pdg::producer::lower(facts).expect("eligible lowering");
    let expected = case.expected.nodes.as_deref().expect("node inventory");
    assert_eq!(
        graph.nodes.len(),
        expected.len(),
        "{} node inventory: {:?}",
        case.id,
        graph.nodes
    );
    let aliases = node_aliases(case, &graph);
    let mut actual: Vec<_> = graph
        .transfers
        .iter()
        .map(|e| {
            (
                aliases
                    .get(&e.source)
                    .cloned()
                    .unwrap_or_else(|| e.source.clone()),
                aliases
                    .get(&e.target)
                    .cloned()
                    .unwrap_or_else(|| e.target.clone()),
                e.kind.clone(),
            )
        })
        .collect();
    let mut oracle: Vec<_> = case
        .expected
        .cfg_edges
        .as_deref()
        .expect("CFG oracle")
        .iter()
        .map(|e| (e.0.clone(), e.1.clone(), e.2.clone()))
        .collect();
    actual.sort();
    oracle.sort();

    assert_eq!(actual, oracle, "{} CFG transfers", case.id);
    oracle::check_bindings(case, facts, &graph, &aliases);
    check_dependence(case, &graph, &aliases);
    check_emission(case, facts);
}

fn check_dependence(
    case: &FixtureCase,
    graph: &rstyle_pdg::producer::Graph,
    aliases: &std::collections::BTreeMap<String, String>,
) {
    let analysis = rstyle_pdg::producer::analyze(graph).expect("normal-flow analysis");
    let oracle = case.expected.pdg_edges.as_ref().expect("PDG oracle");
    let mut expected: Vec<_> = expected_data(oracle)
        .into_iter()
        .chain(expected_flow(oracle))
        .collect();
    let mut actual: Vec<_> = analysis
        .edges
        .iter()
        .map(|e| {
            (
                e.subkind.clone(),
                aliases[&e.source].clone(),
                aliases[&e.target].clone(),
                e.position,
                e.outcome.clone(),
            )
        })
        .collect();
    expected.sort();
    actual.sort();

    assert_eq!(actual, expected, "{} exact dependence multiset", case.id);
    check_value_sites(case, graph, aliases);
}

type EdgeExpectation = (String, String, String, Option<u64>, Option<String>);

fn expected_data(
    oracle: &rstyle_extraction_fixtures::fixture::PdgEdgesExpect,
) -> Vec<EdgeExpectation> {
    let values = oracle.value_operand.iter().map(|e| {
        (
            "value_operand".to_string(),
            e.0.clone(),
            e.1.clone(),
            Some(u64::from(e.2)),
            None,
        )
    });
    let reaching = oracle.reaching_definition.iter().map(|e| {
        (
            "reaching_definition".to_string(),
            e.0.clone(),
            e.1.clone(),
            Some(0),
            None,
        )
    });
    let alternatives = oracle.value_alternative.iter().map(|e| {
        (
            "value_alternative".to_string(),
            e.0.clone(),
            e.1.clone(),
            Some(u64::from(e.2)),
            Some(e.3.clone()),
        )
    });

    values.chain(reaching).chain(alternatives).collect()
}

fn expected_flow(
    oracle: &rstyle_extraction_fixtures::fixture::PdgEdgesExpect,
) -> Vec<EdgeExpectation> {
    let execution = oracle.execution.iter().map(|e| {
        (
            "nearest_dominating_operation".to_string(),
            e.0.clone(),
            e.1.clone(),
            None,
            None,
        )
    });
    let control = oracle.control.iter().map(|e| {
        (
            "postdominance".to_string(),
            e.0.clone(),
            e.1.clone(),
            None,
            Some(e.2.clone()),
        )
    });

    execution.chain(control).collect()
}

fn check_emission(case: &FixtureCase, facts: &HirFnFacts) {
    let pair = rstyle_pdg::producer::produce(facts).expect("captured evidence");
    rstyle_pdg::producer::verify(&pair, facts).expect("source/build/semantic verification");
    let serialized = pair.serialize().expect("paired serialization");
    let pair = rstyle_pdg::producer::verify_serialized(&serialized, facts)
        .expect("exact serialized handoff");
    check_schemas(case, &pair);
    check_admission(case, facts, &pair);
    check_wire_facts(case, &pair);
}

fn check_schemas(case: &FixtureCase, pair: &rstyle_pdg::producer::Pair) {
    let schemas = [
        (
            include_str!("../../../vendor-compat/rust-r0/pdg.schema.json"),
            serde_json::to_value(&pair.pdg).expect("PDG document"),
        ),
        (
            include_str!("../../../vendor-compat/rust-r0/cfg.schema.json"),
            serde_json::to_value(&pair.cfg).expect("CFG"),
        ),
    ];
    for (schema, document) in schemas {
        let schema: serde_json::Value = serde_json::from_str(schema).expect("pinned schema");
        let validator = jsonschema::validator_for(&schema).expect("schema compiles");
        let errors: Vec<_> = validator
            .iter_errors(&document)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{} schema: {errors:?}", case.id);
    }
}

fn check_admission(case: &FixtureCase, facts: &HirFnFacts, pair: &rstyle_pdg::producer::Pair) {
    let bytes =
        std::collections::BTreeMap::from([("source".to_string(), case.source.as_bytes().to_vec())]);
    let context = rstyle_pdg::ValidationContext::CompilerEvidence {
        pointer_width_bits: facts.build.pointer_width.value.expect("width") as u8,
    };
    let report = rstyle_pdg::validate(&pair.pdg, &context, &bytes);
    assert_eq!(
        report.matching,
        rstyle_pdg::Eligibility::Eligible,
        "{}: {:?}",
        case.id,
        report.issues
    );
    let admitted =
        rstyle_ccgraph::admission::admit_strict(&rstyle_ccgraph::admission::StrictBundle {
            pdg: &pair.pdg,
            cfg: &pair.cfg,
            source_bytes: &bytes,
            ctx: &context,
        });
    assert!(admitted.is_ok(), "{} admission: {admitted:?}", case.id);
    let produced = rstyle_ccgraph::admit_produced(pair, facts).expect("captured compiler handoff");
    assert!(
        produced
            .nodes
            .iter()
            .filter(|n| n.category == rstyle_pdg::NodeCategory::Merge)
            .all(|n| n.value_site_reachable)
    );
}

fn check_wire_facts(case: &FixtureCase, pair: &rstyle_pdg::producer::Pair) {
    assert_eq!(
        pair.pdg.function.reference_count.value,
        case.expected.reference_count.map(u64::from)
    );
    if let Some(status) = &case.expected.cfg_evidence_status {
        assert_eq!(
            serde_json::to_value(pair.cfg.extraction_evidence.status)
                .expect("status")
                .as_str(),
            Some(status.as_str()),
            "{} panic assessment",
            case.id
        );
    }
}

fn check_value_sites(
    case: &FixtureCase,
    graph: &rstyle_pdg::producer::Graph,
    aliases: &std::collections::BTreeMap<String, String>,
) {
    let expected = case.expected.value_sites.as_deref().unwrap_or_default();
    assert_eq!(
        graph.value_sites.len(),
        expected.len(),
        "{} value sites",
        case.id
    );
    for site in &graph.value_sites {
        let oracle = expected
            .iter()
            .find(|s| s.pdg_node_id == aliases[&site.pdg_node_id])
            .expect("site alias");
        assert_eq!(
            aliases[&site.controller_pdg_node_id],
            oracle.controller_pdg_node_id
        );
        assert_eq!(
            aliases[site.cfg_successor_node_id.as_ref().expect("continuation")],
            oracle.cfg_successor_node_id
        );
        assert_eq!(
            guard_aliases(&site.guards, aliases),
            expected_guards(&oracle.annotations)
        );
        for alternative in &site.alternatives {
            let expected = oracle
                .alternatives
                .iter()
                .find(|a| a.outcome == alternative.outcome)
                .expect("outcome");
            assert_eq!(
                aliases[&alternative.value_pdg_node_id],
                expected.value_pdg_node_id
            );
            let mut frontier: Vec<_> = alternative
                .completion_cfg_node_ids
                .iter()
                .map(|id| aliases[id].clone())
                .collect();
            let mut oracle_frontier = expected.completion_cfg_node_ids.clone();
            frontier.sort();
            oracle_frontier.sort();
            assert_eq!(frontier, oracle_frontier);
            assert_eq!(
                guard_aliases(&alternative.guards, aliases),
                expected_guards(&expected.annotations)
            );
        }
    }
}

fn expected_guards(
    annotations: &rstyle_extraction_fixtures::fixture::GuardAnnotations,
) -> Vec<Vec<(String, String)>> {
    annotations
        .guards
        .iter()
        .map(|term| {
            term.iter()
                .map(|atom| (atom.controller_pdg_node_id.clone(), atom.outcome.clone()))
                .collect()
        })
        .collect()
}

fn guard_aliases(
    guard: &rstyle_pdg::producer::Guard,
    aliases: &std::collections::BTreeMap<String, String>,
) -> Vec<Vec<(String, String)>> {
    guard
        .iter()
        .map(|term| {
            term.iter()
                .map(|atom| {
                    (
                        aliases[&atom.controller_pdg_node_id].clone(),
                        atom.outcome.clone(),
                    )
                })
                .collect()
        })
        .collect()
}

fn node_aliases(
    case: &FixtureCase,
    graph: &rstyle_pdg::producer::Graph,
) -> std::collections::BTreeMap<String, String> {
    let mut aliases = std::collections::BTreeMap::new();
    for expected in case.expected.nodes.as_deref().expect("inventory") {
        let span = expected.2.as_ref().and_then(|s| s.resolve(&case.source));
        let candidates: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| {
                let category = serde_json::to_value(n.category).expect("category");
                category.as_str() == Some(&expected.1)
                    && span.is_none_or(|(start, end)| {
                        n.span.start as usize == start && n.span.end as usize == end
                    })
            })
            .filter(|n| merge_matches(case, &expected.0, n))
            .collect();
        assert_eq!(
            candidates.len(),
            1,
            "{} alias {} span {span:?}: {candidates:?}",
            case.id,
            expected.0
        );
        aliases.insert(candidates[0].id.clone(), expected.0.clone());
    }

    aliases
}

fn merge_matches(case: &FixtureCase, alias: &str, node: &rstyle_pdg::producer::Operation) -> bool {
    if node.category != rstyle_pdg::NodeCategory::Merge {
        return true;
    }
    let expected = case
        .expected
        .value_sites
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|s| s.pdg_node_id == alias)
        .expect("merge site");
    let (start, end) = expected
        .construct_selector
        .resolve(&case.source)
        .expect("merge construct selector");

    node.span.start as usize == start && node.span.end as usize == end
}

fn check_diagnostics(case: &FixtureCase, found: &[rstyle_pdg::HirDiagnostic]) {
    for expected in case.expected.diagnostics.as_deref().unwrap_or_default() {
        let (start, end) = expected
            .selector
            .resolve(&case.source)
            .expect("independent selector");
        assert!(
            found.iter().any(|d| d.capability == expected.capability
                && d.span.start as usize == start
                && d.span.end as usize == end),
            "{} missing {} at {start}..{end}: {found:?}",
            case.id,
            expected.capability
        );
    }
}

#[test]
fn all_pinned_sources_run_through_real_rustc() {
    let pack = rstyle_extraction_fixtures::parse_pack(include_bytes!(
        "../../../vendor-compat/rust-r0/fixtures/rust-r0-extraction.json"
    ));

    for case in &pack.cases {
        check_case(case);
    }
}

#[test]
fn cfg_disabled_call_is_still_rejected() {
    let (output, facts) = compile("fn f() { #[cfg(any())] g(); }\nfn g() {}");
    assert!(output.status.success());
    let functions = facts.expect("compiler evidence");
    let result = classify(&functions[0]);

    assert_eq!(result.status, EligibilityVerdict::Ineligible);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.capability == "rust.call-summaries" && d.snippet == "g()")
    );
}

#[test]
fn compilation_failure_never_emits_eligible_facts() {
    let (output, facts) = compile("fn f() -> i32 { unresolved } ");

    assert!(!output.status.success());
    assert!(facts.is_none());
}

#[test]
fn original_utf8_crlf_bytes_and_spans_are_preserved() {
    let source = "fn f(é: i32) -> i32 {\r\n    // café\r\n    é + é\r\n}";
    let (output, facts) = compile(&format!("\u{feff}// prefix\r\n{source}"));
    assert!(output.status.success(), "{:?}", output.stderr);
    let facts = facts.expect("captured source");
    let function = &facts[0];
    assert_eq!(function.source.bytes, source);
    assert_eq!(function.fn_span.start, "\u{feff}// prefix\r\n".len() as u32);
    assert_eq!(
        function.fn_span.end - function.fn_span.start,
        source.len() as u32
    );
    let pair = rstyle_pdg::producer::produce(function).expect("exact-byte extraction");
    for node in &pair.pdg.nodes {
        if let Some(span) = &node.source_span {
            assert!(source.get(span.start as usize..span.end as usize).is_some());
        }
    }
    rstyle_ccgraph::admit_produced(&pair, function).expect("strict admission");
}

#[test]
fn unsupported_owned_source_never_produces_a_supported_remainder() {
    let sources = [
        "fn f() { #[cfg(any())] let x = 1f32; }",
        "fn f() { #[cfg(any())] let x = 'x'; }",
        "fn f() { #[cfg(any())] let x = \"x\"; }",
        "fn f() { return; let _x = || 1; }",
        "fn f() { return; let _x = 1 as i64; }",
        "fn f() { fn nested() {} }",
        "fn f() { let _x = async {}; }",
        "fn f() { let _x = stringify!(a); }",
        "fn f(x: (i32, i32)) {}",
        "fn f<T>() {}",
        "fn f() -> bool { () == () }",
    ];
    for source in sources {
        let (output, facts) = compile(source);
        assert!(output.status.success(), "{source}: {:?}", output.stderr);
        let facts = facts.expect("successful compiler evidence");
        let function = &facts[0];
        assert_eq!(
            classify(function).status,
            EligibilityVerdict::Ineligible,
            "{source}"
        );
        assert!(rstyle_pdg::producer::produce(function).is_err(), "{source}");
    }
}

#[test]
fn strict_handoff_rejects_tampering_and_missing_evidence() {
    let (_, facts) = compile("fn f(mut x: i32, y: i32) -> i32 { x += y; x }");
    let facts = &facts.expect("captured facts")[0];
    let baseline = rstyle_pdg::producer::produce(facts).expect("baseline");
    let encoded = baseline.serialize().expect("serialize");
    assert_eq!(encoded.pdg_bytes, baseline.serialize().unwrap().pdg_bytes);
    assert_eq!(encoded.cfg_bytes, baseline.serialize().unwrap().cfg_bytes);
    for mutation in 0..5 {
        let mut pair = rstyle_pdg::producer::produce(facts).unwrap();
        match mutation {
            0 => {
                pair.pdg.edges.pop();
            }
            1 => {
                pair.pdg.edges.push(pair.pdg.edges[0].clone());
            }
            2 => {
                pair.cfg.edges.pop();
            }
            3 => {
                pair.pdg.extensions.remove(rstyle_pdg::OPERATION_MAP_KEY);
            }
            _ => {
                pair.pdg.sources[0].content_identity.value = Some("sha256:0000".to_string());
            }
        }
        assert!(
            rstyle_ccgraph::admit_produced(&pair, facts).is_err(),
            "mutation {mutation}"
        );
    }
    let mut missing = facts.clone();
    missing.source_scan_complete = false;
    assert!(rstyle_pdg::producer::produce(&missing).is_err());
    let mut changed = facts.clone();
    changed.build.overflow_checks.value = Some(false);
    assert!(rstyle_ccgraph::admit_produced(&baseline, &changed).is_err());
}

#[test]
fn graph_identity_and_array_order_do_not_change_admission() {
    let (_, facts) = compile("fn f(a: bool, b: bool, c: bool) -> bool { a && (b || c) }");
    let facts = &facts.expect("captured facts")[0];
    let baseline = rstyle_pdg::producer::produce(facts).unwrap();
    let mut pdg = serde_json::to_value(&baseline.pdg).unwrap();
    let mut cfg = serde_json::to_value(&baseline.cfg).unwrap();
    let mut names = std::collections::BTreeMap::new();
    for document in [&pdg, &cfg] {
        for key in ["nodes", "symbols", "definitions", "edges"] {
            for record in document[key].as_array().into_iter().flatten() {
                if let Some(id) = record["id"].as_str() {
                    names.insert(id.to_string(), format!("renamed:{id}"));
                }
                for operation in record["operations"].as_array().into_iter().flatten() {
                    let id = operation["id"].as_str().unwrap();
                    names.insert(id.to_string(), format!("renamed:{id}"));
                }
            }
        }
    }
    rename_strings(&mut pdg, &names);
    rename_strings(&mut cfg, &names);
    for document in [&mut pdg, &mut cfg] {
        for key in ["nodes", "symbols", "definitions", "edges"] {
            if let Some(array) = document
                .get_mut(key)
                .and_then(serde_json::Value::as_array_mut)
            {
                array.reverse();
            }
        }
    }
    let renamed = rstyle_pdg::producer::Pair {
        pdg: serde_json::from_value(pdg).unwrap(),
        cfg: serde_json::from_value(cfg).unwrap(),
    };
    rstyle_pdg::producer::verify(&renamed, facts)
        .expect("unchanged baseline under known correspondence");
    rstyle_ccgraph::admit_produced(&renamed, facts).expect("invariant strict admission");
}

fn rename_strings(
    value: &mut serde_json::Value,
    names: &std::collections::BTreeMap<String, String>,
) {
    match value {
        serde_json::Value::String(text) => {
            if let Some(replacement) = names.get(text) {
                *text = replacement.clone();
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                rename_strings(value, names);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                rename_strings(value, names);
            }
        }
        _ => {}
    }
}

#[test]
fn aliases_mutable_parameters_and_unit_values_cross_strict_admission() {
    let sources = [
        "type Scalar = i32; fn f(mut x: Scalar) -> Scalar { x += 1; x + x }",
        "fn f(mut u: ()) { u = (); let x = u; x; }",
        "fn f(a: bool) -> i32 { if a { return 1; } else { return 2; } }",
        "fn f(x: i32) -> i32 { let x = x; { let x = x + 1; x }; x }",
        "fn f() -> i32 { -2147483648i32 }",
    ];
    for source in sources {
        let (output, facts) = compile(source);
        assert!(output.status.success(), "{source}: {:?}", output.stderr);
        let facts = &facts.expect("captured facts")[0];
        let pair = rstyle_pdg::producer::produce(facts).expect(source);
        rstyle_ccgraph::admit_produced(&pair, facts).expect(source);
        if source.starts_with("type Scalar") {
            assert_eq!(facts.signature.params[0].ty.spelling, "Scalar");
            assert_eq!(
                facts.signature.params[0].ty.normalized.as_deref(),
                Some("i32")
            );
            assert_eq!(pair.pdg.function.reference_count.value, Some(1));
        }
        if source.starts_with("fn f(mut u") {
            assert_eq!(pair.pdg.nodes.len(), 7);
            assert!(
                pair.pdg
                    .edges
                    .iter()
                    .all(|e| !matches!(e.subkind.as_str(), "value_operand" | "value_alternative"))
            );
            assert_eq!(
                pair.pdg
                    .edges
                    .iter()
                    .filter(|e| e.subkind == "reaching_definition")
                    .count(),
                2
            );
        }
        if source.contains("-2147483648") {
            assert_eq!(
                pair.cfg.extraction_evidence.status,
                rstyle_pdg::cfg::ExtractionStatus::Complete
            );
        }
    }
}

#[test]
fn conditionally_removed_unresolved_facts_remain_unknown() {
    let (_, facts) = compile("fn f() { #[cfg(any())] let x = UNRESOLVED; }");
    let function = &facts.expect("compiler evidence")[0];
    assert_eq!(classify(function).status, EligibilityVerdict::Unknown);
    assert!(rstyle_pdg::producer::produce(function).is_err());
}

#[test]
fn missing_semantic_build_values_cannot_receive_certification() {
    let (_, facts) = compile("fn f(a: i32) -> i32 { a }");
    let facts = &facts.expect("captured facts")[0];
    let baseline = rstyle_pdg::producer::produce(facts).unwrap();
    for missing in 0..4 {
        let mut incomplete = facts.clone();
        match missing {
            0 => incomplete.build.frontend_options.clear(),
            1 => incomplete.build.overflow_checks.value = None,
            2 => incomplete.build.debug_assertions.value = None,
            _ => incomplete.build.rustc_commit.value = None,
        }
        assert!(rstyle_pdg::producer::produce(&incomplete).is_err());
        assert!(rstyle_ccgraph::admit_produced(&baseline, &incomplete).is_err());
    }
}

#[test]
fn independent_value_validator_rejects_future_guard_authorization() {
    let (_, facts) = compile(
        "fn f(a: bool, b: bool, c: bool) -> bool { let x = a && b; if c { x } else { x } }",
    );
    let facts = &facts.expect("captured facts")[0];
    let pair = rstyle_pdg::producer::produce(facts).unwrap();
    let mut document = serde_json::to_value(&pair.pdg).unwrap();
    let sites = &document["extensions"]["compat.rust.hir-r0:value-sites"];
    let merge = sites[0]["pdg_node_id"].clone();
    let future = sites[1]["controller_pdg_node_id"].clone();
    let guard = serde_json::json!([[{"controller_pdg_node_id": future, "outcome": "true"}]]);
    document["extensions"]["compat.rust.hir-r0:value-sites"][0]["annotations"]["compat.rust.hir-r0:guards"] =
        guard.clone();
    let node = document["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == merge)
        .unwrap();
    node["annotations"]["compat.rust.hir-r0:guards"] = guard;
    let pdg = serde_json::from_value(document).unwrap();
    assert!(rstyle_pdg::validate_value_sites(&pdg, &pair.cfg).is_err());
}

#[test]
fn eager_scalar_operator_inventory_and_operand_positions_are_independent() {
    for op in [
        "+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>", "==", "!=", "<", "<=", ">", ">=",
    ] {
        let result = if matches!(op, "==" | "!=" | "<" | "<=" | ">" | ">=") {
            "bool"
        } else {
            "i32"
        };
        let source = format!("fn f(a: i32, b: i32) -> {result} {{ a {op} b }}");
        check_eager_operator(&source, op);
    }
    for ty in [
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
    ] {
        check_eager_operator(&format!("fn f(a: {ty}, b: {ty}) -> {ty} {{ a + b }}"), "+");
    }
    for op in ["&", "|", "^", "==", "!=", "<", "<=", ">", ">="] {
        check_eager_operator(
            &format!("fn f(a: bool, b: bool) -> bool {{ a {op} b }}"),
            op,
        );
    }
}

fn check_eager_operator(source: &str, operator: &str) {
    let (output, facts) = compile(source);
    assert!(output.status.success(), "{source}: {:?}", output.stderr);
    let facts = &facts.expect("captured facts")[0];
    let graph = rstyle_pdg::producer::lower(facts).expect(source);
    let expected = [
        rstyle_pdg::NodeCategory::FormalInput,
        rstyle_pdg::NodeCategory::FormalInput,
        rstyle_pdg::NodeCategory::Read,
        rstyle_pdg::NodeCategory::Read,
        rstyle_pdg::NodeCategory::Computation,
        rstyle_pdg::NodeCategory::Return,
    ];
    assert_eq!(
        graph.nodes.iter().map(|n| n.category).collect::<Vec<_>>(),
        expected
    );
    assert_eq!(graph.nodes[4].operator.as_deref(), Some(operator));
    assert_eq!(
        graph.nodes[4].operands,
        vec![graph.nodes[2].id.clone(), graph.nodes[3].id.clone()]
    );
    assert_eq!(graph.transfers.len(), 5);
    let pair = rstyle_pdg::producer::produce(facts).expect(source);
    assert_eq!(pair.pdg.edges.len(), 8);
    assert_eq!(pair.pdg.function.reference_count.value, Some(2));
    rstyle_ccgraph::admit_produced(&pair, facts).expect(source);
}

#[test]
fn compound_updates_keep_rhs_first_and_semantic_operand_positions() {
    for op in ["+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>="] {
        let source = format!("fn f(mut a: i32, b: i32) -> i32 {{ a {op} b; a }}");
        let (_, facts) = compile(&source);
        let facts = &facts.expect("compiler evidence")[0];
        let graph = rstyle_pdg::producer::lower(facts).unwrap();
        assert_eq!(graph.nodes.len(), 8);
        assert_eq!(operation_text(&graph.nodes[2], facts), "b");
        assert_eq!(operation_text(&graph.nodes[3], facts), "a");
        assert_eq!(
            graph.nodes[4].operator.as_deref(),
            Some(op.trim_end_matches('='))
        );
        assert_eq!(
            graph.nodes[4].operands,
            [graph.nodes[3].id.clone(), graph.nodes[2].id.clone()]
        );
        assert_eq!(
            graph.nodes[5].category,
            rstyle_pdg::NodeCategory::Assignment
        );
        assert_eq!(graph.transfers.len(), 7);
        let pair = rstyle_pdg::producer::produce(facts).unwrap();
        assert_eq!(pair.pdg.edges.len(), 12);
        rstyle_ccgraph::admit_produced(&pair, facts).unwrap();
    }
}

fn operation_text<'a>(node: &rstyle_pdg::producer::Operation, facts: &'a HirFnFacts) -> &'a str {
    &facts.source.bytes[node.span.start as usize..node.span.end as usize]
}

#[test]
fn unary_operations_preserve_their_input_snapshot() {
    for (ty, op) in [
        ("i8", "-"),
        ("i16", "-"),
        ("i32", "-"),
        ("i64", "-"),
        ("i128", "-"),
        ("isize", "-"),
        ("i32", "!"),
        ("u32", "!"),
        ("bool", "!"),
    ] {
        let (_, facts) = compile(&format!("fn f(a: {ty}) -> {ty} {{ {op}a }}"));
        let facts = &facts.expect("compiler evidence")[0];
        let graph = rstyle_pdg::producer::lower(facts).unwrap();
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.nodes[1].category, rstyle_pdg::NodeCategory::Read);
        assert_eq!(graph.nodes[2].operator.as_deref(), Some(op));
        assert_eq!(graph.nodes[2].operands, [graph.nodes[1].id.clone()]);
        assert_eq!(graph.transfers.len(), 4);
        let pair = rstyle_pdg::producer::produce(facts).unwrap();
        assert_eq!(pair.pdg.edges.len(), 5);
        rstyle_ccgraph::admit_produced(&pair, facts).unwrap();
    }
}

#[test]
fn serialized_manifest_policies_guards_and_frontiers_are_checked() {
    let (_, facts) = compile("fn f(a: bool, b: bool) -> bool { a && b }");
    let facts = &facts.expect("captured facts")[0];
    let baseline = rstyle_pdg::producer::produce(facts).unwrap();
    for mutation in 0..8 {
        let mut serialized = baseline.serialize().unwrap();
        if mutation == 0 {
            serialized.manifest.documents[0].sha256 = "sha256:wrong".to_string();
            assert!(rstyle_pdg::producer::verify_serialized(&serialized, facts).is_err());
            continue;
        }
        let mut document: serde_json::Value =
            serde_json::from_slice(&serialized.pdg_bytes).unwrap();
        mutate_wire_document(&mut document, mutation);
        if let Ok(pdg) = serde_json::from_value::<rstyle_pdg::PdgDocument>(document) {
            let cfg = serde_json::from_slice(&serialized.cfg_bytes).unwrap();
            assert!(
                rstyle_ccgraph::admit_produced(&rstyle_pdg::producer::Pair { pdg, cfg }, facts)
                    .is_err(),
                "mutation {mutation}"
            );
        } else {
            assert!(
                matches!(mutation, 1 | 2),
                "unexpected decode failure {mutation}"
            );
        }
    }
}

fn mutate_wire_document(document: &mut serde_json::Value, mutation: usize) {
    match mutation {
        1 => {
            document["edges"][0]["policy"]["version"] = "0.0.0".into();
        }
        2 => {
            document["function"]["reference_count"]
                .as_object_mut()
                .unwrap()
                .remove("policy");
        }
        3 => {
            document["extensions"]["compat.rust.hir-r0:value-sites"][0]["annotations"]["compat.rust.hir-r0:guards"] =
                serde_json::json!([]);
        }
        4 => {
            document["extensions"]["compat.rust.hir-r0:value-sites"][0]["alternatives"][0]["completion_cfg_node_ids"] =
                serde_json::json!([]);
        }
        5 => {
            document["extraction_evidence"]["capabilities"]["node_extraction"]["status"] =
                "not_applicable".into();
        }
        6 => {
            document["definitions"][0]["node_id"] = document["definitions"][1]["node_id"].clone();
        }
        _ => {
            let definition = document["definitions"][1]["id"].clone();
            let edge = document["edges"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|edge| edge["subkind"] == "reaching_definition")
                .unwrap();
            edge["definition_id"] = definition;
        }
    }
}

#[test]
fn opt_in_driver_artifacts_cross_the_serialized_strict_handoff() {
    let dir = directory();
    let input = dir.join("input.rs");
    let facts_path = dir.join("facts.json");
    let documents = dir.join("documents");
    std::fs::write(&input, "fn f(a: i32) -> i32 { a }\nfn rejected() { f(1); }").unwrap();
    let output = invocation(&input, &facts_path)
        .env("RSTYLE_HIR_DOCUMENT_DIR", &documents)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    let facts: Vec<HirFnFacts> =
        serde_json::from_slice(&std::fs::read(&facts_path).unwrap()).unwrap();
    let serialized = rstyle_pdg::producer::SerializedPair {
        pdg_bytes: std::fs::read(documents.join("function-0.pdg.json")).unwrap(),
        cfg_bytes: std::fs::read(documents.join("function-0.cfg.json")).unwrap(),
        manifest: serde_json::from_slice(
            &std::fs::read(documents.join("function-0.manifest.json")).unwrap(),
        )
        .unwrap(),
    };
    let pair = rstyle_pdg::producer::verify_serialized(&serialized, &facts[0]).unwrap();
    rstyle_ccgraph::admit_produced(&pair, &facts[0]).unwrap();
    assert!(!documents.join("function-1.pdg.json").exists());
    let rejected: serde_json::Value = serde_json::from_slice(
        &std::fs::read(documents.join("function-1.assessment.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(rejected["eligibility"], "Ineligible");
    assert_eq!(rejected["matching_admission"], "not_evaluated");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn source_identifiers_and_file_navigation_are_independent_of_module_paths() {
    let source = "mod one { pub fn r#type(x: i32) -> i32 { x } }\nmod two { pub fn r#type(x: i32) -> i32 { x } }";
    let (output, facts) = compile(source);
    assert!(output.status.success());
    let facts = facts.unwrap();
    assert_eq!(facts.len(), 2);
    assert_ne!(facts[0].name, facts[1].name);
    for function in &facts {
        assert_eq!(function.build.frontend_options["source_identifier"], "type");
        assert!(
            PathBuf::from(
                function.build.frontend_options["source_file"]
                    .as_str()
                    .unwrap()
            )
            .is_absolute()
        );
        let span = function.fn_span;
        assert_eq!(
            &source[span.start as usize..span.end as usize],
            function.source.bytes
        );
        let pair = rstyle_pdg::producer::produce(function).unwrap();
        assert_eq!(
            pair.pdg.sources[0].path,
            function.build.frontend_options["source_file"]
                .as_str()
                .unwrap()
        );
        assert_eq!(pair.cfg.sources[0].path, pair.pdg.sources[0].path);
        let imported = rstyle_ccgraph::admit_produced(&pair, function).unwrap();
        assert_eq!(imported.name.as_deref(), Some("type"));
        assert_eq!(
            pair.pdg.function.qualified_name.known_value(),
            Some(function.name.as_str())
        );
    }
}

#[test]
fn name_feature_preserves_the_original_unicode_identifier_token() {
    let identifier = "cafe\u{301}";
    let source = format!("pub fn {identifier}(x: i32) -> i32 {{ x }}");
    let (output, facts) = compile(&source);
    assert!(output.status.success());
    let facts = facts.unwrap();
    assert_eq!(
        facts[0].build.frontend_options["source_identifier"],
        identifier
    );
    let pair = rstyle_pdg::producer::produce(&facts[0]).unwrap();
    assert_eq!(
        rstyle_ccgraph::admit_produced(&pair, &facts[0])
            .unwrap()
            .name
            .as_deref(),
        Some(identifier)
    );
}

fn compile_selected(source: &str, profile: &str) -> Vec<HirFnFacts> {
    compile_selected_settings(source, profile, &[])
}

fn compile_selected_settings(source: &str, profile: &str, settings: &[&str]) -> Vec<HirFnFacts> {
    let directory = directory();
    let input = directory.join("input.rs");
    let facts = directory.join("facts.json");
    std::fs::write(&input, source).unwrap();
    let output = invocation(&input, &facts)
        .args(settings)
        .env("RSTYLE_HIR_EXTRACTION_PROFILE", profile)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let captured = serde_json::from_slice(&std::fs::read(facts).unwrap()).unwrap();
    std::fs::remove_dir_all(directory).unwrap();

    captured
}

#[test]
fn local_const_profile_preserves_body_operations_and_const_evidence() {
    let bodies = [
        "fn f(x: i32) -> i32 { x }",
        "fn f(x: bool) -> bool { if x { false } else { true } }",
        "fn f(x: ()) { let y = x; y; }",
        "fn f(mut x: u8, b: bool) -> u8 { if b { x = 1; } x }",
    ];
    for source in bodies {
        let ordinary = compile_selected(source, "hir-const-v1").remove(0);
        let qualified_source = format!("const {source}");
        let qualified = compile_selected(&qualified_source, "hir-const-v1").remove(0);
        assert!(qualified.signature.is_const);
        assert!(!ordinary.signature.is_const);
        assert_eq!(qualified.source.bytes, qualified_source);
        assert_eq!(qualified.fn_span.start, 0);
        assert_eq!(qualified.fn_span.end as usize, qualified_source.len());
        assert_const_graph_equivalence(&ordinary, &qualified);
        let pair = rstyle_pdg::producer::produce(&qualified).unwrap();
        assert_eq!(pair.pdg.profile.id, "rstyle.rust.hir-coverage");
        let serialized = pair.serialize().unwrap();
        rstyle_pdg::producer::verify_serialized(&serialized, &qualified).unwrap();
        rstyle_ccgraph::admit_produced(&pair, &qualified).unwrap();
        let mut changed = qualified.clone();
        changed.signature.is_const = false;
        assert!(rstyle_pdg::producer::verify_serialized(&serialized, &changed).is_err());
        assert_local_wire_schema(&pair);
    }
}

#[test]
fn const_expansion_keeps_r0_and_deferred_source_closed() {
    let sources = [
        "const fn f(x: i32) -> i32 { x as i32 }",
        "const fn f() -> i32 { const { 1 } }",
        "const fn f() -> i32 { const X: i32 = 1; X }",
        "const fn f(x: i32) -> i32 { return x; x as i32 }",
        "const fn f(x: i32) -> i32 { g(x) } const fn g(x: i32) -> i32 { x }",
    ];
    for source in sources {
        let facts = compile_selected(source, "hir-const-v1").remove(0);
        assert_eq!(classify(&facts).status, EligibilityVerdict::Ineligible);
        assert!(rstyle_pdg::producer::produce(&facts).is_err());
    }
    let source = "const fn f(x: i32) -> i32 { x }";
    let baseline = compile_selected(source, "r0").remove(0);
    assert_eq!(classify(&baseline).status, EligibilityVerdict::Ineligible);
    let unknown = compile_selected(source, "hir-unregistered-v1").remove(0);
    assert_eq!(classify(&unknown).status, EligibilityVerdict::Unknown);
}

#[test]
fn local_const_import_rejects_missing_or_tampered_policy_and_frontend_evidence() {
    use rstyle_pdg::extraction_profile::EVIDENCE_KEY;
    let facts = compile_selected("const fn f(x: i32) -> i32 { x }", "hir-const-v1").remove(0);
    for mutation in 0..6 {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        match mutation {
            0 => {
                pair.pdg.extensions.remove(EVIDENCE_KEY);
            }
            1 => {
                pair.cfg.profile.version = "0.2.0".into();
            }
            2 => {
                pair.pdg.extensions.get_mut(EVIDENCE_KEY).unwrap()["capabilities"] =
                    serde_json::json!([]);
            }
            3 => {
                pair.pdg.extensions.get_mut(EVIDENCE_KEY).unwrap()["frontend"]["signature"]["is_const"] =
                    serde_json::json!(false);
            }
            4 => {
                pair.pdg.nodes[0].evidence.policy = Some(rstyle_pdg::ProfileId::hir_r0());
            }
            _ => {
                pair.cfg.edges.pop();
            }
        }
        let sources = pair
            .pdg
            .sources
            .iter()
            .map(|s| (s.id.clone(), facts.source.bytes.as_bytes().to_vec()))
            .collect();
        let context = rstyle_pdg::ValidationContext::CompilerEvidence {
            pointer_width_bits: 64,
        };
        let bundle = rstyle_ccgraph::StrictBundle {
            pdg: &pair.pdg,
            cfg: &pair.cfg,
            source_bytes: &sources,
            ctx: &context,
        };
        assert!(
            rstyle_ccgraph::admit_strict(&bundle).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn disclosed_historical_const_regressions_cross_serialized_admission() {
    for source in [
        include_str!("../../../eval/ccgraph-v2/corpus/lz4/sample/src/lib.rs"),
        include_str!("../../../eval/ccgraph-v2/corpus/time/sample/src/lib.rs"),
    ] {
        let facts = compile_selected(source, "hir-const-v1");
        for function in facts.iter().filter(|f| f.name.contains("old")) {
            assert!(function.signature.is_const);
            let pair = rstyle_pdg::producer::produce(function).unwrap();
            let serialized = pair.serialize().unwrap();
            let decoded = rstyle_pdg::producer::verify_serialized(&serialized, function).unwrap();
            rstyle_ccgraph::admit_produced(&decoded, function).unwrap();
        }
        for function in facts.iter().filter(|f| f.name.contains("current")) {
            assert_eq!(classify(function).status, EligibilityVerdict::Ineligible);
        }
    }
}

#[test]
fn integer_cast_inventory_and_dependencies_cover_all_scalar_widths() {
    let types = [
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
    ];
    let mut source = String::new();
    let mut expected = Vec::new();
    for input in types {
        for result in types {
            let name = format!("f{}", expected.len());
            source.push_str(&format!(
                "fn {name}(x: {input}) -> {result} {{ x as {result} }}\n"
            ));
            expected.push((name, input, result));
        }
    }
    let facts = compile_selected(&source, "hir-casts-v1");
    for (name, input, result) in expected {
        let function = facts.iter().find(|f| f.name == name).unwrap();
        assert_cast_graph(function, input, result);
        let pair = rstyle_pdg::producer::produce(function).unwrap();
        let evidence = &pair.pdg.extensions["rstyle.hir-coverage:integer-casts"];
        assert_eq!(evidence.as_array().unwrap().len(), 1);
        assert_eq!(evidence[0]["operand_type"], input);
        assert_eq!(evidence[0]["result_type"], result);
        assert_eq!(
            evidence[0]["target_pointer_width"],
            function.build.pointer_width.value.unwrap()
        );
        let serialized = pair.serialize().unwrap();
        rstyle_pdg::producer::verify_serialized(&serialized, function).unwrap();
        rstyle_ccgraph::admit_produced(&pair, function).unwrap();
    }
}

#[test]
fn nested_cast_values_in_writes_branches_and_returns_are_retained() {
    let source = "fn f(mut x: i64) -> u8 { x = (x as i8 as i32) as i64; if (x as u16) > 255 { return x as u8; } (x + 1) as u8 }";
    let facts = compile_selected(source, "hir-casts-v1").remove(0);
    let graph = rstyle_pdg::producer::lower(&facts).unwrap();
    let casts: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| {
            n.category == rstyle_pdg::NodeCategory::Computation && n.original_kind == "expr_cast"
        })
        .collect();
    assert_eq!(casts.len(), 6);
    assert_eq!(
        casts.iter().map(|n| n.ty.as_str()).collect::<Vec<_>>(),
        ["i8", "i32", "i64", "u16", "u8", "u8"]
    );
    for cast in casts {
        assert_eq!(cast.operands.len(), 1);
        assert!(
            facts.source.bytes[cast.span.start as usize..cast.span.end as usize].contains(" as ")
        );
    }
    let pair = rstyle_pdg::producer::produce(&facts).unwrap();
    rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
    for profile in ["r0", "hir-const-v1"] {
        assert_eq!(
            classify(&compile_selected(source, profile)[0]).status,
            EligibilityVerdict::Ineligible
        );
    }
}

#[test]
fn integer_casts_reject_other_domains_and_tampered_facts() {
    for source in [
        "fn f(x: i32) -> f64 { x as f64 }",
        "fn f(x: f64) -> i32 { x as i32 }",
        "fn f(x: *const u8) -> usize { x as usize }",
        "fn f(x: &i32) -> *const i32 { x as *const i32 }",
        "fn f(x: bool) -> u8 { x as u8 }",
        "fn f(x: i32) -> i32 { return x; (x as f64) as i32 }",
    ] {
        let facts = compile_selected(source, "hir-casts-v1").remove(0);
        assert_eq!(classify(&facts).status, EligibilityVerdict::Ineligible);
    }
}

#[test]
fn integer_casts_reject_tampered_facts_and_documents() {
    let facts = compile_selected("fn f(x: i32) -> u8 { x as u8 }", "hir-casts-v1").remove(0);
    let mut changed = facts.clone();
    changed.body.children[0]
        .dispatch
        .as_mut()
        .unwrap()
        .operand_tys = vec!["u32".into()];
    assert_eq!(classify(&changed).status, EligibilityVerdict::Unknown);
    for mutation in 0..4 {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        match mutation {
            0 => {
                pair.pdg
                    .extensions
                    .get_mut("rstyle.hir-coverage:integer-casts")
                    .unwrap()[0]["target_pointer_width"] = serde_json::json!(32);
            }
            1 => {
                pair.pdg.nodes[2].attributes.operator = Some(rstyle_pdg::StringFact::known("+"));
            }
            2 => {
                pair.pdg
                    .edges
                    .retain(|e| !(e.source == "n1" && e.target == "n2"));
            }
            _ => {
                pair.cfg
                    .nodes
                    .iter_mut()
                    .flat_map(|n| &mut n.operations)
                    .find(|o| o.original_kind == "expr_cast")
                    .unwrap()
                    .original_kind = "expr_path".into();
            }
        }
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "mutation {mutation}"
        );
    }
    let mut serialized = rstyle_pdg::producer::produce(&facts)
        .unwrap()
        .serialize()
        .unwrap();
    serialized.manifest.documents[0].sha256 = "sha256:0000".into();
    assert!(rstyle_pdg::producer::verify_serialized(&serialized, &facts).is_err());
}

#[test]
fn scalar_match_has_independent_operation_flow_and_merge_expectations() {
    let facts = compile_selected(
        "fn f(x: i32) -> i32 { match x { 1 => 7, _ => 9 } }",
        "hir-match-v1",
    )
    .remove(0);
    let graph = rstyle_pdg::producer::lower(&facts).unwrap();
    assert_literal_match_inventory(&graph);
    assert_eq!(graph.nodes[2].operands, ["n1"]);
    assert_eq!(graph.nodes[2].construct.as_deref(), Some("match.literal"));
    assert_eq!(graph.nodes[2].literal.as_deref(), Some("1"));
    assert_eq!(graph.value_sites.len(), 1);
    let site = &graph.value_sites[0];
    assert_eq!(site.selection_kind.as_deref(), Some("ordered-match/1"));
    assert_eq!(
        site.alternatives
            .iter()
            .map(|a| (
                a.outcome.as_str(),
                a.value_pdg_node_id.as_str(),
                a.completion_cfg_node_ids.as_slice()
            ))
            .collect::<Vec<_>>(),
        [
            ("arm:0", "n3", ["n3".to_string()].as_slice()),
            ("arm:1", "n5", ["n5".to_string()].as_slice())
        ]
    );
    let analysis = rstyle_pdg::producer::analyze(&graph).unwrap();
    assert_eq!(
        analysis
            .edges
            .iter()
            .filter(|e| e.family == rstyle_pdg::EdgeFamily::Control)
            .map(|e| (e.source.as_str(), e.target.as_str(), e.outcome.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("n2", "n3", Some("true")),
            ("n2", "n4", Some("false")),
            ("n2", "n5", Some("false"))
        ]
    );
    let pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let serialized = pair.serialize().unwrap();
    rstyle_pdg::producer::verify_serialized(&serialized, &facts).unwrap();
    rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
}

#[test]
fn scalar_match_keeps_writes_returns_and_unreachable_source() {
    for source in [
        "fn f(x: bool) -> i32 { match x { true => 1, _ => 2 } }",
        "fn f(x: i32) -> i32 { match x { -1 => 0, 1 => 2, _ => 3 } }",
        "fn f(x: i32) -> i32 { match x { 1 => return 7, _ => return 9 } }",
        "fn f(x: i32, mut y: i32) -> i32 { match x { 1 => { y = 2; }, _ => { y = 3; } }; y }",
        "fn f(x: i32, mut y: i32) -> i32 { match x { 1 => { let z = y; y = z; }, _ => {} }; y }",
        "fn f(x: i32, b: bool) -> i32 { match x { 1 => { if b { 2 } else { 3 } }, _ => 4 } }",
        "fn f(x: i32) -> i32 { return x; match x { 1 => 2, _ => 3 } }",
    ] {
        let facts = compile_selected(source, "hir-match-v1").remove(0);
        let pair = rstyle_pdg::producer::produce(&facts)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
    }
    for source in [
        "fn f(x: i32) -> i32 { match x { y => y } }",
        "fn f(x: i32) -> i32 { match x { 1 | 2 => 0, _ => 1 } }",
        "fn f(x: i32) -> i32 { match x { 1..=2 => 0, _ => 1 } }",
        "fn f(x: bool) -> i32 { match x { true => 0, false => 1 } }",
        "fn f(x: i32) -> i32 { match x { 1 if x > 0 => 0, _ => 1 } }",
        "fn f(x: i32) -> i32 { match x { _ => 0, 1 => g(x) } } fn g(x: i32) -> i32 { x }",
        "fn f(x: &i32) -> i32 { match x { &1 => 0, _ => 1 } }",
    ] {
        let facts = compile_selected(source, "hir-match-v1").remove(0);
        assert_eq!(
            classify(&facts).status,
            EligibilityVerdict::Ineligible,
            "{source}"
        );
    }
}

#[test]
fn ordered_match_guards_fall_through_once_and_preserve_arm_values() {
    let source = "fn f(x: i32, b: bool) -> i32 { match x { 1 if b => 7, 1 if !b => 8, _ => 9 } }";
    let facts = compile_selected(source, "hir-guards-v1").remove(0);
    let graph = rstyle_pdg::producer::lower(&facts).unwrap();
    assert_guard_fallthrough(&graph);
    let selectors: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.construct.as_deref() == Some("match.literal"))
        .collect();
    let guards: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.construct.as_deref() == Some("match.guard"))
        .collect();
    let site = graph.value_sites.last().unwrap();
    assert_eq!(site.alternatives.len(), 3);
    assert_eq!(site.alternatives[0].guards[0].len(), 2);
    assert_eq!(
        site.alternatives[0].guards[0]
            .iter()
            .map(|a| (a.controller_pdg_node_id.as_str(), a.outcome.as_str()))
            .collect::<Vec<_>>(),
        [
            (selectors[0].id.as_str(), "true"),
            (guards[0].id.as_str(), "true")
        ]
    );
    let pair = rstyle_pdg::producer::produce(&facts).unwrap();
    rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
    let serialized = pair.serialize().unwrap();
    rstyle_pdg::producer::verify_serialized(&serialized, &facts).unwrap();
    let baseline = compile_selected(source, "hir-match-v1").remove(0);
    assert_eq!(classify(&baseline).status, EligibilityVerdict::Ineligible);
}

#[test]
fn guard_subset_and_tampered_fallthrough_are_fail_closed() {
    for source in [
        "fn f(x: i32, b: bool) -> i32 { match x { _ if b && x > 0 => 1, _ => 2 } }",
        "fn f(x: i32, b: bool, mut y: i32) -> i32 { match x { 1 if b => { y = 1; }, _ => { y = 2; } }; y }",
        "fn f(x: i32, b: bool) -> i32 { match x { 1 if if b { x > 0 } else { false } => return 7, _ => 9 } }",
    ] {
        let facts = compile_selected(source, "hir-guards-v1").remove(0);
        let pair =
            rstyle_pdg::producer::produce(&facts).unwrap_or_else(|e| panic!("{source}: {e:?}"));
        rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
    }
    for source in [
        "fn f(x: i32, mut b: bool) -> i32 { match x { 1 if { b = false; b } => 7, _ => 9 } }",
        "fn f(x: i32) -> i32 { match x { 1 if (x as u32) > 0 => 7, _ => 9 } }",
        "fn f(x: i32) -> i32 { match x { 1 if g(x) => 7, _ => 9 } } fn g(x: i32) -> bool { x > 0 }",
        "fn f(x: i32) -> i32 { match x { 1 if true => 7, _ => 9, 2 if g(x) => 3 } } fn g(x: i32) -> bool { x > 0 }",
    ] {
        let facts = compile_selected(source, "hir-guards-v1").remove(0);
        assert_eq!(classify(&facts).status, EligibilityVerdict::Ineligible);
    }
    let facts = compile_selected(
        "fn f(x: i32, b: bool) -> i32 { match x { 1 if b => 7, _ => 9 } }",
        "hir-guards-v1",
    )
    .remove(0);
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    let guard = pair
        .pdg
        .nodes
        .iter()
        .find(|n| n.attributes.construct.as_deref() == Some("match.guard"))
        .unwrap()
        .id
        .clone();
    pair.cfg
        .edges
        .retain(|e| !(e.source == guard && e.kind == rstyle_pdg::CfgEdgeKind::False));
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
}

#[test]
fn conditional_source_correspondence_keeps_removed_resolution_unknown() {
    let supported = [
        "/// documented\n#[inline]\npub const fn f(x: i32) -> i32 { x }",
        "fn f(x: i32) -> i32 { #[cfg(all())] let y = x; y }",
        "fn f(x: i32) -> i32 { #[cfg(all())] { let y = x; y; } x }",
        "fn f(x: bool) -> i32 { match x { true => 1, _ => 2 } }",
    ];
    for source in supported {
        let facts = compile_selected(source, "hir-conditional-v1").remove(0);
        assert_eq!(
            classify(&facts).status,
            EligibilityVerdict::Eligible,
            "{source}: {:?}",
            classify(&facts)
        );
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts).unwrap();
        rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
    }
    for source in [
        "fn f() { #[cfg(any())] let x = 1; }",
        "fn f() { #[cfg(any())] let x = unresolved; }",
        "fn f() { #[cfg(any())] { #[cfg(all())] let x = 1; } }",
    ] {
        let facts = compile_selected(source, "hir-conditional-v1").remove(0);
        assert_eq!(classify(&facts).status, EligibilityVerdict::Unknown);
    }
    for source in [
        "/// documented\nfn f() { #[cfg(any())] missing(); }",
        "fn f() { #[cfg(any())] unavailable!(); }",
        "fn f() { #[cfg(any())] const X: i32 = 1; }",
    ] {
        let facts = compile_selected(source, "hir-conditional-v1").remove(0);
        assert_eq!(classify(&facts).status, EligibilityVerdict::Ineligible);
    }
}

#[test]
fn conditional_evidence_inventory_cfg_and_resolution_tampering_are_rejected() {
    let facts = compile_selected(
        "fn f(x: i32) -> i32 { #[cfg(all())] let y = x; y }",
        "hir-conditional-v1",
    )
    .remove(0);
    assert_eq!(classify(&facts).status, EligibilityVerdict::Eligible);
    for mutation in 0..5 {
        let mut changed = facts.clone();
        match mutation {
            0 => {
                changed
                    .build
                    .frontend_options
                    .remove(rstyle_pdg::source_evidence::KEY);
            }
            1 => {
                changed
                    .build
                    .frontend_options
                    .get_mut(rstyle_pdg::source_evidence::KEY)
                    .unwrap()["entries"] = serde_json::json!([]);
            }
            2 => {
                changed.build.cfgs.push("feature=\"different\"".into());
            }
            3 => {
                changed
                    .build
                    .frontend_options
                    .get_mut(rstyle_pdg::source_evidence::KEY)
                    .unwrap()["entries"][0]["hir"] = serde_json::json!(null);
            }
            _ => {
                changed
                    .build
                    .frontend_options
                    .get_mut(rstyle_pdg::source_evidence::KEY)
                    .unwrap()["source_digest"] = serde_json::json!("sha256:0000");
            }
        }
        assert_eq!(
            classify(&changed).status,
            EligibilityVerdict::Unknown,
            "mutation {mutation}"
        );
        assert!(rstyle_pdg::producer::produce(&changed).is_err());
    }
}

#[test]
fn scalar_loop_cfg_and_reaching_definitions_match_independent_fixed_point_expectations() {
    let facts = compile_selected(
        "fn f(mut x: i32, b: bool) -> i32 { while b { x = 1; } x }",
        "hir-loops-v1",
    )
    .remove(0);
    assert_eq!(
        classify(&facts).status,
        EligibilityVerdict::Eligible,
        "{:?}",
        classify(&facts)
    );
    let graph = rstyle_pdg::producer::lower(&facts).unwrap();
    assert_loop_inventory(&graph);
    let analysis = rstyle_pdg::producer::analyze_cyclic(&graph).unwrap();
    assert_loop_control(&analysis);
    assert_loop_execution(&analysis);
    let x = &facts.signature.params[0].symbol;
    assert_eq!(
        analysis.definition_in["n6"][x]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["d0", "d5"]
    );
    assert_eq!(
        analysis.definition_out["n5"][x]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["d5"]
    );
    let pair = rstyle_pdg::producer::produce(&facts).unwrap();
    rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts).unwrap();
    rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
    let sources = pair
        .pdg
        .sources
        .iter()
        .map(|s| (s.id.clone(), facts.source.bytes.as_bytes().to_vec()))
        .collect();
    let context = rstyle_pdg::ValidationContext::CompilerEvidence {
        pointer_width_bits: 64,
    };
    rstyle_ccgraph::admit_strict(&rstyle_ccgraph::StrictBundle {
        pdg: &pair.pdg,
        cfg: &pair.cfg,
        source_bytes: &sources,
        ctx: &context,
    })
    .unwrap();
}

#[test]
fn scalar_loops_cover_exits_nested_ownership_and_divergence_without_fake_edges() {
    for source in [
        "fn f(mut x: i32, b: bool) -> i32 { while b { x += 1; if x > 2 { break; } continue; } x }",
        "fn f(b: bool) -> i32 { loop { if b { return 1; } continue; } }",
        "fn f(b: bool) { loop { while b { break; } if b { break; } } }",
        "fn f() -> i32 { loop {} }",
        "fn f() { while true {} }",
        "fn f(mut x: i32) -> i32 { while false { x = 1; } x }",
        "fn f(x: i32) -> i32 { loop { break; } x }",
    ] {
        let facts = compile_selected(source, "hir-loops-v1").remove(0);
        assert_eq!(
            classify(&facts).status,
            EligibilityVerdict::Eligible,
            "{source}: {:?}",
            classify(&facts)
        );
        let pair =
            rstyle_pdg::producer::produce(&facts).unwrap_or_else(|e| panic!("{source}: {e:?}"));
        rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
        if source.contains("loop {}") || source.contains("while true {}") {
            assert!(
                !rstyle_pdg::reachable_vertices(&pair.cfg).contains(&pair.cfg.exit_node_ids[0])
            );
            assert!(
                !pair
                    .cfg
                    .edges
                    .iter()
                    .any(|edge| pair.cfg.exit_node_ids.contains(&edge.target))
            );
        }
    }
    for source in [
        "fn f(b: bool) { 'outer: loop { if b { break 'outer; } } }",
        "fn f() -> i32 { loop { break 1; } }",
        "fn f(b: bool) -> i32 { while b {} if b { 1 } else { 2 } }",
        "fn f(a: bool, b: bool) { while a && b {} }",
        "fn f() { while false { missing(); } } fn missing() {}",
        "fn f() { for x in 0..3 { let y = x; } }",
    ] {
        let facts = compile_selected(source, "hir-loops-v1").remove(0);
        assert_eq!(
            classify(&facts).status,
            EligibilityVerdict::Ineligible,
            "{source}"
        );
    }
}

#[test]
fn cyclic_serialized_verifier_rejects_back_edges_and_definition_set_tampering() {
    let facts = compile_selected(
        "fn f(mut x: i32, b: bool) -> i32 { while b { x = 1; } x }",
        "hir-loops-v1",
    )
    .remove(0);
    for mutation in 0..4 {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        match mutation {
            0 => {
                pair.cfg
                    .edges
                    .retain(|edge| edge.loop_role != Some(rstyle_pdg::LoopRole::BackEdge));
            }
            1 => {
                let edge = pair
                    .cfg
                    .edges
                    .iter()
                    .find(|edge| edge.loop_role == Some(rstyle_pdg::LoopRole::BackEdge))
                    .unwrap()
                    .clone();
                pair.cfg.edges.push(edge);
            }
            2 => {
                pair.pdg.edges.retain(|edge| {
                    !(edge.target == "n6" && edge.definition_id.as_deref() == Some("d5"))
                });
            }
            _ => {
                pair.pdg
                    .extensions
                    .get_mut("rstyle.hir-coverage:cyclic-flow-policy")
                    .unwrap()["virtual_executable_exits"] = serde_json::json!(true);
            }
        }
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "mutation {mutation}"
        );
        if mutation == 2 {
            assert!(rstyle_pdg::validate_value_sites(&pair.pdg, &pair.cfg).is_err());
        }
    }
}

#[test]
fn source_correspondence_preserves_parentheses_unicode_and_original_crlf_spans() {
    for source in [
        "/// docs\npub const fn f(x: usize) -> usize { 16 + 4 + (x as u64 * 110 / 100) as usize }",
        "fn f(é: i32) -> i32 { #[cfg(all())] let λ = ((é)); (λ + 1) }",
        "fn f(x: i32) -> i32 {\r\n    #[cfg(all())] let y = x;\r\n    (y + 1)\r\n}",
    ] {
        let facts = compile_selected(source, "hir-conditional-v1").remove(0);
        assert_eq!(
            classify(&facts).status,
            EligibilityVerdict::Eligible,
            "{source}: {:?}",
            classify(&facts)
        );
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
    }
}

#[test]
fn primitive_summaries_preserve_receiver_argument_order_and_independent_dependencies() {
    for (signed, unsigned) in [("i32", "u32"), ("i64", "u64")] {
        let source = format!(
            "fn f(x: {signed}, y: {unsigned}) -> {unsigned} {{ x.unsigned_abs().wrapping_mul(y) }}"
        );
        let facts = compile_selected(&source, "hir-calls-v1").remove(0);
        assert_eq!(
            classify(&facts).status,
            EligibilityVerdict::Eligible,
            "{:?}",
            classify(&facts)
        );
        let graph = rstyle_pdg::producer::lower(&facts).unwrap();
        assert_eq!(
            graph
                .nodes
                .iter()
                .map(|n| n.original_kind.as_str())
                .collect::<Vec<_>>(),
            [
                "pat_binding",
                "pat_binding",
                "expr_path",
                "expr_method_call",
                "expr_path",
                "expr_method_call",
                "expr_method_call"
            ]
        );
        assert_eq!(graph.nodes[3].operands, ["n2"]);
        assert_eq!(graph.nodes[5].operands, ["n3", "n4"]);
        assert_eq!(graph.nodes[3].category, rstyle_pdg::NodeCategory::Call);
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts).unwrap();
        rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
        assert_primitive_summary_inventory(&pair);
        let r0 = compile_selected(&source, "r0").remove(0);
        assert_eq!(classify(&r0).status, EligibilityVerdict::Ineligible);
    }
}

#[test]
fn primitive_summaries_fail_closed_on_neighbors_and_callee_or_build_tampering() {
    for source in [
        "fn f(x: i16) -> u16 { x.unsigned_abs() }",
        "fn f(x: u32) -> u32 { x.wrapping_add(1) }",
        "fn f(x: i32) -> i32 { x.abs() }",
        "struct S; impl S { fn unsigned_abs(self) -> u32 { 1 } } fn f(x: S) -> u32 { x.unsigned_abs() }",
    ] {
        let facts = compile_selected(source, "hir-calls-v1");
        assert!(
            facts
                .iter()
                .filter(|f| f.name == "f")
                .all(|f| classify(f).status == EligibilityVerdict::Ineligible),
            "{source}"
        );
    }
}

#[test]
fn primitive_summaries_reject_tampered_build_callee_and_registry() {
    let facts =
        compile_selected("fn f(x: i32) -> u32 { x.unsigned_abs() }", "hir-calls-v1").remove(0);
    for mutation in 0..4 {
        let mut changed = facts.clone();
        match mutation {
            0 => changed.build.rustc_commit = rstyle_pdg::StringFact::known("wrong"),
            1 => {
                changed.build.target_triple =
                    rstyle_pdg::StringFact::known("x86_64-unknown-linux-gnu")
            }
            2 => change_call_provenance(&mut changed.body),
            _ => change_call_result_type(&mut changed.body),
        }
        assert_eq!(
            classify(&changed).status,
            EligibilityVerdict::Ineligible,
            "mutation {mutation}"
        );
    }
    for field in [
        "version",
        "memory_effects",
        "may_panic",
        "source_sha256",
        "always_returns",
    ] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        pair.pdg
            .extensions
            .get_mut(rstyle_pdg::call_summaries::KEY)
            .unwrap()[0]["summary"][field] = serde_json::json!("tampered");
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "{field}"
        );
    }
}

fn change_call_provenance(node: &mut rstyle_pdg::HirNode) {
    if node.kind == "expr_method_call" {
        node.res.as_mut().unwrap().description = "{}".into();
    }
    for child in &mut node.children {
        change_call_provenance(child);
    }
}

fn change_call_result_type(node: &mut rstyle_pdg::HirNode) {
    if node.kind == "expr_method_call" {
        node.dispatch.as_mut().unwrap().result_ty = "u64".into();
    }
    for child in &mut node.children {
        change_call_result_type(child);
    }
}

fn assert_const_graph_equivalence(ordinary: &HirFnFacts, qualified: &HirFnFacts) {
    let ordinary_graph = rstyle_pdg::producer::lower(ordinary).unwrap();
    let qualified_graph = rstyle_pdg::producer::lower(qualified).unwrap();
    assert_eq!(ordinary_graph.transfers, qualified_graph.transfers);
    assert_eq!(
        ordinary_graph.value_sites.len(),
        qualified_graph.value_sites.len()
    );
    let semantic = |g: &rstyle_pdg::producer::Graph| {
        g.nodes
            .iter()
            .map(|n| {
                (
                    n.category,
                    n.original_kind.clone(),
                    n.ty.clone(),
                    n.operator.clone(),
                    n.operands.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(semantic(&ordinary_graph), semantic(&qualified_graph));
    assert_eq!(
        rstyle_pdg::producer::analyze(&ordinary_graph)
            .unwrap()
            .edges,
        rstyle_pdg::producer::analyze(&qualified_graph)
            .unwrap()
            .edges
    );
}

fn assert_local_wire_schema(pair: &rstyle_pdg::producer::Pair) {
    for (schema, document) in [
        (
            include_str!("../../../vendor-compat/rust-r0/pdg.schema.json"),
            serde_json::to_value(&pair.pdg).unwrap(),
        ),
        (
            include_str!("../../../vendor-compat/rust-r0/cfg.schema.json"),
            serde_json::to_value(&pair.cfg).unwrap(),
        ),
    ] {
        let schema = serde_json::from_str(schema).unwrap();
        assert!(
            jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&document)
        );
    }
}

fn assert_cast_graph(function: &HirFnFacts, input: &str, result: &str) {
    let graph = rstyle_pdg::producer::lower(function).unwrap();
    assert_eq!(graph.nodes.len(), 4);
    assert_eq!(
        graph
            .nodes
            .iter()
            .map(|n| n.original_kind.as_str())
            .collect::<Vec<_>>(),
        ["pat_binding", "expr_path", "expr_cast", "expr_cast"]
    );
    assert_eq!(graph.nodes[1].ty, input);
    assert_eq!(graph.nodes[2].ty, result);
    assert_eq!(graph.nodes[2].operator.as_deref(), Some("as"));
    assert_eq!(graph.nodes[2].operands, ["n1"]);
    assert_eq!(graph.nodes[3].operands, ["n2"]);
    assert_eq!(
        graph
            .transfers
            .iter()
            .map(|e| (e.source.as_str(), e.target.as_str(), e.kind.as_str()))
            .collect::<Vec<_>>(),
        [
            ("E", "n1", "normal"),
            ("n1", "n2", "normal"),
            ("n2", "n3", "normal"),
            ("n3", "X", "return")
        ]
    );
    assert_cast_data(&graph);
}

fn assert_cast_data(graph: &rstyle_pdg::producer::Graph) {
    let analysis = rstyle_pdg::producer::analyze(graph).unwrap();
    assert_eq!(
        analysis
            .edges
            .iter()
            .filter(|e| e.family == rstyle_pdg::EdgeFamily::Data)
            .map(|e| (e.source.as_str(), e.target.as_str(), e.position))
            .collect::<Vec<_>>(),
        [
            ("n0", "n1", Some(0)),
            ("n1", "n2", Some(0)),
            ("n2", "n3", Some(0))
        ]
    );
}

fn assert_literal_match_inventory(graph: &rstyle_pdg::producer::Graph) {
    assert_eq!(
        graph
            .nodes
            .iter()
            .map(|n| n.original_kind.as_str())
            .collect::<Vec<_>>(),
        [
            "pat_binding",
            "expr_path",
            "pat_literal",
            "expr_lit",
            "pat_wild",
            "expr_lit",
            "rstyle.hir-coverage.match_merge",
            "expr_match"
        ]
    );
    assert_eq!(
        graph
            .transfers
            .iter()
            .map(|e| (e.source.as_str(), e.target.as_str(), e.kind.as_str()))
            .collect::<Vec<_>>(),
        [
            ("E", "n1", "normal"),
            ("n1", "n2", "normal"),
            ("n2", "n3", "true"),
            ("n2", "n4", "false"),
            ("n3", "n7", "normal"),
            ("n4", "n5", "normal"),
            ("n5", "n7", "normal"),
            ("n7", "X", "return")
        ]
    );
}

fn assert_guard_fallthrough(graph: &rstyle_pdg::producer::Graph) {
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.category == rstyle_pdg::NodeCategory::Read && n.ty == "i32")
            .count(),
        1
    );
    let selectors: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.construct.as_deref() == Some("match.literal"))
        .collect();
    let guards: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.construct.as_deref() == Some("match.guard"))
        .collect();
    assert_eq!(selectors.len(), 2);
    assert_eq!(guards.len(), 2);
    assert_eq!(selectors[0].operands, selectors[1].operands);
    let next = &selectors[1].id;
    assert!(
        graph
            .transfers
            .iter()
            .any(|e| e.source == selectors[0].id && &e.target == next && e.kind == "false")
    );
    assert!(
        graph
            .transfers
            .iter()
            .any(|e| e.source == guards[0].id && &e.target == next && e.kind == "false")
    );
}

fn assert_loop_inventory(graph: &rstyle_pdg::producer::Graph) {
    assert_eq!(
        graph
            .nodes
            .iter()
            .map(|n| n.original_kind.as_str())
            .collect::<Vec<_>>(),
        [
            "pat_binding",
            "pat_binding",
            "expr_path",
            "expr_while",
            "expr_lit",
            "expr_assign",
            "expr_path",
            "expr_path"
        ]
    );
    assert_loop_transfers(graph);
}

fn assert_loop_control(analysis: &rstyle_pdg::producer::Analysis) {
    let mut control: Vec<_> = analysis
        .edges
        .iter()
        .filter(|edge| edge.family == rstyle_pdg::EdgeFamily::Control)
        .map(|edge| {
            (
                edge.source.as_str(),
                edge.target.as_str(),
                edge.outcome.as_deref(),
            )
        })
        .collect();
    control.sort();
    assert_eq!(
        control,
        [
            ("n3", "n2", Some("true")),
            ("n3", "n3", Some("true")),
            ("n3", "n4", Some("true")),
            ("n3", "n5", Some("true")),
            ("n3", "n6", Some("false")),
            ("n3", "n7", Some("false"))
        ]
    );
}

fn assert_loop_execution(analysis: &rstyle_pdg::producer::Analysis) {
    let mut execution: Vec<_> = analysis
        .edges
        .iter()
        .filter(|edge| edge.family == rstyle_pdg::EdgeFamily::Execution)
        .map(|edge| (edge.source.as_str(), edge.target.as_str()))
        .collect();
    execution.sort();
    assert_eq!(
        execution,
        [
            ("n2", "n3"),
            ("n3", "n4"),
            ("n3", "n6"),
            ("n4", "n5"),
            ("n6", "n7")
        ]
    );
}

fn assert_loop_transfers(graph: &rstyle_pdg::producer::Graph) {
    assert_eq!(
        {
            let mut edges = graph
                .transfers
                .iter()
                .map(|e| {
                    (
                        e.source.as_str(),
                        e.target.as_str(),
                        e.kind.as_str(),
                        e.loop_role.as_deref(),
                    )
                })
                .collect::<Vec<_>>();
            edges.sort();
            edges
        },
        [
            ("E", "n2", "normal", None),
            ("n2", "n3", "normal", None),
            ("n3", "n4", "true", None),
            ("n3", "n6", "false", Some("loop_exit")),
            ("n4", "n5", "normal", None),
            ("n5", "n2", "normal", Some("back_edge")),
            ("n6", "n7", "normal", None),
            ("n7", "X", "return", None)
        ]
    );
}

fn assert_primitive_summary_inventory(pair: &rstyle_pdg::producer::Pair) {
    let summaries = pair.pdg.extensions[rstyle_pdg::call_summaries::KEY]
        .as_array()
        .unwrap();
    assert_eq!(summaries.len(), 2);
    assert_eq!(
        summaries[0]["summary"]["overflow_behavior"],
        "unsigned-magnitude-including-min"
    );
    assert_eq!(
        summaries[1]["summary"]["overflow_behavior"],
        "modulo-result-width"
    );
    assert_eq!(
        summaries[1]["operand_pdg_node_ids"],
        serde_json::json!(["n3", "n4"])
    );
}

#[test]
fn primitive_summaries_pin_sources_and_accept_both_overflow_settings() {
    use sha2::{Digest, Sha256};
    let output = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let sysroot = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    for summary in rstyle_pdg::call_summaries::registry() {
        let source = std::fs::read(
            sysroot
                .join("lib/rustlib/src/rust")
                .join(&summary.source_path),
        )
        .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(source)),
            summary.source_sha256
        );
        assert!(!summary.memory_effects && !summary.mutates_arguments && !summary.may_panic);
        assert!(summary.always_returns);
    }
    for settings in [
        vec!["-C", "overflow-checks=yes", "-C", "panic=unwind"],
        vec!["-C", "overflow-checks=no", "-C", "panic=abort"],
    ] {
        let facts = compile_selected_settings(
            "fn f() -> u32 { (-2147483648i32).unsigned_abs().wrapping_mul(4294967295u32) }",
            "hir-calls-v1",
            &settings,
        )
        .remove(0);
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
        let mut changed = facts.clone();
        changed.build.overflow_checks.value = Some(!facts.build.overflow_checks.value.unwrap());
        assert!(rstyle_pdg::producer::verify(&pair, &changed).is_err());
    }
}

#[test]
fn expanded_pair_compatibility_rejects_mixed_profiles_and_build_settings() {
    let source = "fn f(x: i32) -> i32 { x }";
    let r0 = compile_selected(source, "r0").remove(0);
    let local = compile_selected(source, "hir-calls-v1").remove(0);
    let altered =
        compile_selected_settings(source, "hir-calls-v1", &["-C", "overflow-checks=no"]).remove(0);
    for other in [r0, altered] {
        let functions = [&local, &other]
            .into_iter()
            .map(|facts| {
                rstyle_ccgraph::admit_produced(
                    &rstyle_pdg::producer::produce(facts).unwrap(),
                    facts,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let mut functions = functions;
        functions[0].key = "a".into();
        functions[1].key = "b".into();
        let report =
            rstyle_ccgraph::evaluate_all(&functions, &rstyle_ccgraph::CcgraphConfig::default());
        assert_eq!(report.pairs.len(), 1);
        assert_ne!(report.pairs[0].decision, rstyle_ccgraph::Decision::Accepted);
        assert!(matches!(
            report.pairs[0].stages[0].outcome,
            rstyle_ccgraph::StageOutcome::Ineligible { .. }
        ));
    }
}
