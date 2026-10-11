//! Real pinned rustc runs against independent source/eligibility expectations.

use rstyle_pdg::{EligibilityVerdict, R0FnFacts, classify};
use rstyle_r0fixture::fixture::FixtureCase;
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
    let directory = std::env::temp_dir().join(format!("rstyle-r0-{}-{serial}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("create isolated rustc run");

    directory
}

fn compile(source: &str) -> (Output, Option<Vec<R0FnFacts>>) {
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
            "r0fixture",
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
        .env("RSTYLE_R0_OUT_FILE", facts);

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

fn check_lowering(case: &FixtureCase, facts: &R0FnFacts) {
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

fn expected_data(oracle: &rstyle_r0fixture::fixture::PdgEdgesExpect) -> Vec<EdgeExpectation> {
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

fn expected_flow(oracle: &rstyle_r0fixture::fixture::PdgEdgesExpect) -> Vec<EdgeExpectation> {
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

fn check_emission(case: &FixtureCase, facts: &R0FnFacts) {
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
            rstyle_pdg::producer::pinned::pdg_value(&pair.pdg).expect("pinned PDG"),
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

fn check_admission(case: &FixtureCase, facts: &R0FnFacts, pair: &rstyle_pdg::producer::Pair) {
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
    annotations: &rstyle_r0fixture::fixture::GuardAnnotations,
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

fn check_diagnostics(case: &FixtureCase, found: &[rstyle_pdg::R0Diagnostic]) {
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
    let pack = rstyle_r0fixture::parse_pack(include_bytes!(
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

fn operation_text<'a>(node: &rstyle_pdg::producer::Operation, facts: &'a R0FnFacts) -> &'a str {
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
        if let Ok(pdg) =
            rstyle_pdg::producer::pinned::read_pdg(&serde_json::to_vec(&document).unwrap())
        {
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
        .env("RSTYLE_R0_DOCUMENT_DIR", &documents)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    let facts: Vec<R0FnFacts> =
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
