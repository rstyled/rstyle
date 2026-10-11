//! Vendored-fixture integrity tests: every selector resolves.
//!
//! These tests load the pinned `rust-r0-extraction.json` snapshot and check
//! that the independent expectation side is coherent: format markers, case
//! inventory, selector resolvability, overlap legality and source identities.
//! They run without any producer and without rustc.

use rstyle_r0fixture::fixture::{FixturePack, load_vendored_pack, parse_pack};
use std::collections::BTreeSet;

/// The pinned snapshot embedded at compile time: exact bytes, no IO effect.
fn test_pack() -> FixturePack {
    parse_pack(include_bytes!(
        "../../../vendor-compat/rust-r0/fixtures/rust-r0-extraction.json"
    ))
}

#[test]
fn vendored_file_loads_from_disk() {
    assert_eq!(load_vendored_pack().cases.len(), EXPECTED_IDS.len());
}

const EXPECTED_IDS: [&str; 23] = [
    "straightline-integer-input-and-tail",
    "two-arm-reassignment-reaching-set",
    "short-circuit-and-saved-left",
    "short-circuit-or-outcomes",
    "early-return-controls-continuation",
    "returning-arm-does-not-execute-continuation-declaration",
    "if-value-effectful-declaration-site",
    "saved-left-value-before-branching-operand",
    "nested-if-merges-share-site-with-distinct-availability",
    "rhs-first-primitive-compound-write",
    "ordinary-binary-old-value-snapshot",
    "parameter-shadow-initializer-reads-old-binding",
    "empty-unit-body-retains-unused-interface",
    "unit-binding-retains-definition-not-value-operands",
    "explicit-unit-literal-return",
    "unreachable-supported-source-preserved",
    "unsupported-unit-comparison-not-builtin",
    "unsupported-direct-call",
    "unsupported-call-after-return",
    "unsupported-short-circuit-macro",
    "unsupported-nested-item-owner",
    "unsupported-unnamed-parameter",
    "unsupported-reference-interface",
];

#[test]
fn pack_markers_match_the_pinned_profile() {
    let pack = test_pack();

    assert_eq!(
        pack.fixture_format.id,
        "compat.rust.hir-r0.fixture-expectations"
    );
    assert_eq!(pack.fixture_format.version, "0.1.0");
    assert_eq!(pack.extraction_profile.id, "compat.rust.hir-r0");
    assert_eq!(pack.extraction_profile.version, "0.1.0");
    assert_eq!(pack.schema_targets.cfg, "0.2.0");
    assert_eq!(pack.schema_targets.pdg, "0.1.0");
    assert_eq!(pack.configuration.edition, "2021");
    assert_eq!(pack.cases.len(), EXPECTED_IDS.len());
}

#[test]
fn case_inventory_matches_the_pinned_pack() {
    let pack = test_pack();
    let ids: Vec<&str> = pack.cases.iter().map(|case| case.id.as_str()).collect();

    assert_eq!(ids, EXPECTED_IDS);
}

#[test]
fn cases_are_unrun_exact_sources() {
    let pack = test_pack();

    for case in &pack.cases {
        assert_eq!(case.execution_status, "not_run", "case {}", case.id);
        assert!(!case.source.is_empty(), "case {}", case.id);
    }
}

#[test]
fn cases_have_no_trailing_newline() {
    let pack = test_pack();

    for case in &pack.cases {
        assert!(!case.source.ends_with('\n'), "case {}", case.id);
    }
}

#[test]
fn every_selector_in_every_case_resolves() {
    let pack = test_pack();
    let mut failures = Vec::new();

    for case in &pack.cases {
        for selector in case.all_selectors() {
            if selector.resolve(&case.source).is_none() {
                failures.push(format!(
                    "{}: selector {:?} occurrence {} has no match",
                    case.id, selector.0, selector.1
                ));
            }
        }

        for node in case.expected.nodes.clone().unwrap_or_default() {
            if node.1 == "merge" {
                assert!(
                    node.2.is_none(),
                    "{}: merge node {} must use a null selector",
                    case.id,
                    node.0
                );
            } else {
                assert!(
                    node.2.is_some(),
                    "{}: operation node {} needs a source selector",
                    case.id,
                    node.0
                );
            }
        }
    }

    assert!(
        failures.is_empty(),
        "unresolved selectors:\n{}",
        failures.join("\n")
    );
}

#[test]
fn resolved_spans_slice_back_to_selector_text() {
    let pack = test_pack();

    for case in &pack.cases {
        for node in case.expected.nodes.clone().unwrap_or_default() {
            let Some(selector) = node.2 else { continue };
            let span = selector.resolve(&case.source).expect("resolves");

            // Single-token selectors slice to their token; multi-token spans
            // cover the run from first token start to last token end.
            assert!(
                span.0 < span.1 && span.1 <= case.source.len(),
                "{}: node {} has an empty span",
                case.id,
                node.0
            );
            assert!(
                case.source.is_char_boundary(span.0) && case.source.is_char_boundary(span.1),
                "{}: node {} span splits a character",
                case.id,
                node.0
            );
        }
    }

    let straight = pack
        .cases
        .iter()
        .find(|case| case.id == "straightline-integer-input-and-tail")
        .expect("straightline case");
    let span = straight
        .resolved_node_spans()
        .remove("dx")
        .flatten()
        .expect("dx resolves");
    assert_eq!(&straight.source[span.0..span.1], "let x = a + 1;");
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[test]
fn source_identities_are_unique_content_hashes() {
    let pack = test_pack();
    let mut identities = BTreeSet::new();

    for case in &pack.cases {
        let identity = case.source_identity();

        assert!(identity.starts_with("sha256:"), "case {}", case.id);
        assert_eq!(identity.len(), "sha256:".len() + 64, "case {}", case.id);
        assert!(
            identities.insert(identity),
            "duplicate identity: {}",
            case.id
        );
    }
}

#[test]
fn source_digests_cover_exact_bytes() {
    let pack = test_pack();

    for case in &pack.cases {
        // The digest covers the exact bytes: appending a newline changes it.
        let mut altered = case.source.clone();
        altered.push('\n');

        assert_ne!(
            case.source_identity(),
            format!("sha256:{}", sha256_hex(altered.as_bytes())),
            "case {}",
            case.id
        );
    }
}

#[test]
fn negative_cases_require_visible_diagnostics() {
    let pack = test_pack();
    let negative_ids = [
        "unsupported-unit-comparison-not-builtin",
        "unsupported-direct-call",
        "unsupported-call-after-return",
        "unsupported-short-circuit-macro",
        "unsupported-nested-item-owner",
        "unsupported-unnamed-parameter",
        "unsupported-reference-interface",
    ];

    for id in negative_ids {
        let case = pack.cases.iter().find(|case| case.id == id).expect("case");
        let expected = &case.expected;

        assert_eq!(expected.eligibility, "ineligible", "case {id}");
        assert_eq!(
            expected.conformance.as_deref(),
            Some("non_conformant"),
            "case {id}"
        );
        assert!(expected.nodes.is_none(), "case {id} has no remainder graph");
        assert!(
            expected.cfg_edges.is_none(),
            "case {id} has no remainder graph"
        );

        let diagnostics = expected.diagnostics.clone().unwrap_or_default();
        assert!(!diagnostics.is_empty(), "case {id} explains the rejection");

        for diagnostic in &diagnostics {
            assert!(!diagnostic.capability.is_empty(), "case {id}");
            assert!(!diagnostic.reason.is_empty(), "case {id}");
            assert!(
                diagnostic.selector.resolve(&case.source).is_some(),
                "case {id}"
            );
        }
    }
}

#[test]
fn accepted_cases_have_exact_graph_structure() {
    let pack = test_pack();

    for case in pack
        .cases
        .iter()
        .filter(|case| case.expected.eligibility == "eligible_under_verified_normal_flow_profile")
    {
        let expected = &case.expected;

        assert_eq!(expected.exact_graph, Some(true), "case {}", case.id);
        assert!(expected.nodes.is_some(), "case {}", case.id);
        assert!(expected.definitions.is_some(), "case {}", case.id);
        assert!(expected.cfg_edges.is_some(), "case {}", case.id);
        assert!(expected.pdg_edges.is_some(), "case {}", case.id);
        assert!(!expected.forbidden.is_empty(), "case {}", case.id);
        assert!(expected.reference_count.is_some(), "case {}", case.id);
    }
}

#[test]
fn accepted_symbol_spans_slice_to_declared_names() {
    let pack = test_pack();

    for case in pack
        .cases
        .iter()
        .filter(|case| case.expected.eligibility == "eligible_under_verified_normal_flow_profile")
    {
        let symbols = case.expected.symbols.clone().unwrap_or_default();

        for symbol in &symbols {
            let span = symbol.4.resolve(&case.source).expect("symbol resolves");
            assert_eq!(&case.source[span.0..span.1], symbol.1, "case {}", case.id);
        }
    }
}
