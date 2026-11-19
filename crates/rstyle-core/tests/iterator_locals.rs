//! Unchanged production captures recover formal scans across one immutable local.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::compose::{Analyzed, Composed, Composer, Composition};
use rstyle_core::runtime_bounds::{
    self, Assessment, Bound, Callee, Field, Limits, Measure, Outcome, Program, Root,
};
use rstyle_core::wording::Wording;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

struct Case {
    document: &'static str,
    provenance: &'static str,
    owner: &'static str,
    input: &'static str,
    field: &'static str,
    field_index: u32,
}

const PAIRS: Case = Case {
    document: include_str!("../src/cpg/testdata/iterator-locals-pairs-production.cpg.json"),
    provenance: include_str!(
        "../src/cpg/testdata/iterator-locals-pairs-production.provenance.json"
    ),
    owner: "rstyle_core::align::{impl#5}::pairs",
    input: "len(self.fwd)#0",
    field: "fwd",
    field_index: 0,
};
const CLASSES: Case = Case {
    document: include_str!("../src/cpg/testdata/iterator-locals-classes-production.cpg.json"),
    provenance: include_str!(
        "../src/cpg/testdata/iterator-locals-classes-production.provenance.json"
    ),
    owner: "rstyle_core::cluster::classes_of",
    input: "len(pdg.nodes)#0",
    field: "nodes",
    field_index: 0,
};
const PRODUCERS: Case = Case {
    document: include_str!(
        "../src/cpg/testdata/iterator-locals-data-producers-production.cpg.json"
    ),
    provenance: include_str!(
        "../src/cpg/testdata/iterator-locals-data-producers-production.provenance.json"
    ),
    owner: "rstyle_core::normalize::data_producers",
    input: "len(pdg.edges)#0",
    field: "edges",
    field_index: 1,
};
const CONSUMERS: Case = Case {
    document: include_str!(
        "../src/cpg/testdata/iterator-locals-data-consumers-production.cpg.json"
    ),
    provenance: include_str!(
        "../src/cpg/testdata/iterator-locals-data-consumers-production.provenance.json"
    ),
    owner: "rstyle_core::normalize::data_consumers",
    input: "len(pdg.edges)#0",
    field: "edges",
    field_index: 1,
};

fn checksum(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn saved(case: &Case) -> Value {
    let document: Value = serde_json::from_str(case.document).unwrap();
    let provenance: Value = serde_json::from_str(case.provenance).unwrap();
    assert_eq!(
        checksum(case.document.as_bytes()),
        provenance["fixture_checksum"]
    );
    assert_eq!(provenance["owner"], case.owner);
    let functions = document["unit"]["functions"].as_array().unwrap();
    let retained = provenance["retained_functions"].as_array().unwrap();
    assert_eq!(functions.len(), retained.len());
    for declaration in retained {
        let function = functions
            .iter()
            .find(|function| function["id"] == declaration["function"])
            .unwrap();
        assert_eq!(function["definition"], declaration["definition"]);
        assert_eq!(
            checksum(&serde_json::to_vec(function).unwrap()),
            declaration["checksum"]
        );
    }
    for source in document["unit"]["files"].as_array().unwrap() {
        assert_eq!(
            checksum(source["content"].as_str().unwrap().as_bytes()),
            source["digest"]
        );
    }
    assert_eq!(
        document["coverage"]["capabilities"]["implicit-calls"]["status"],
        "unsupported"
    );
    document
}

fn programs(document: Value) -> BTreeMap<String, Program> {
    let keys: BTreeMap<_, _> = document["unit"]["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|definition| {
            let identity = definition["stable_identity"].as_str().unwrap().to_owned();
            let compilation = definition["compilation"].as_str().unwrap_or_default();
            (identity.clone(), format!("{compilation}/{identity}"))
        })
        .collect();
    rstyle_core::cpg::run_bounds(vec![document])
        .unwrap()
        .into_iter()
        .map(|(id, input)| {
            let Input::Captured { program } = input else {
                panic!("{id}: {input:?}")
            };
            (keys[&id].clone(), program)
        })
        .collect()
}

fn key<'a>(programs: &'a BTreeMap<String, Program>, owner: &str) -> &'a str {
    programs.keys().find(|key| key.ends_with(owner)).unwrap()
}

fn local(program: &Program) -> Assessment {
    let outcome = runtime_bounds::analyze(program);
    let Outcome::Proven(assessment) = outcome else {
        panic!("{outcome:?}")
    };
    assessment
}

fn analyzed(case: &Case, callbacks: usize) -> (BTreeMap<String, Program>, Assessment) {
    let programs = programs(saved(case));
    let program = &programs[key(&programs, case.owner)];
    let assessment = local(program);
    let bound = Bound::Input(case.input.into());
    assert_eq!(assessment.loop_body_entries, bound);
    assert_eq!(assessment.loops.len(), 1);
    assert_eq!(assessment.calls.len(), callbacks);
    assert!(assessment.calls.iter().all(|call| call.count == bound));
    assert_eq!(program.extents.len(), 1);
    let extent = &program.extents[0];
    assert_eq!(extent.input, case.input);
    assert_eq!(extent.root, Root::Formal(0));
    assert_eq!(
        extent.fields,
        vec![Field {
            index: case.field_index,
            name: case.field.into()
        }]
    );
    assert_eq!(program.formals[0].measure, Measure::Shared);
    (programs, assessment)
}

fn composition(programs: &BTreeMap<String, Program>, owner: &str) -> Composed {
    let mut composer = Composer::new(
        |id: &str| {
            programs.get(id).map(|program| Analyzed {
                program: program.clone(),
                outcome: runtime_bounds::analyze(program),
            })
        },
        Limits::default(),
    );
    composer.composed(key(programs, owner), Wording::of("rust"))
}

fn proven(programs: &BTreeMap<String, Program>, case: &Case) -> Composition {
    let outcome = composition(programs, case.owner);
    let Composed::Proven(composed) = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(composed.loop_body_entries, Bound::Input(case.input.into()));
    assert!(composed.opaque_factors.is_empty());
    composed
}

fn resolved_callback(assessment: &Assessment, suffix: &str) {
    assert!(
        assessment.calls.iter().any(|call| {
            matches!(&call.target, Callee::Resolved(identity) if identity.ends_with(suffix))
        }),
        "{assessment:?}"
    );
}

#[test]
fn exact_pairs_capture_recovers_a_formal_scan_with_the_option_callback_inlined() {
    let (programs, _) = analyzed(&PAIRS, 0);
    let composed = proven(&programs, &PAIRS);
    assert_eq!(composed.workspace_invocations, Bound::Const(0));
    // The nested Option::map uses the existing at-most-once callback model;
    // its callback constructs a tuple and contains no further calls or loops.
    assert_eq!(composed.library_steps, Bound::Const(0));
}

fn exact_edge_scan(case: &Case) {
    let (programs, assessment) = analyzed(case, 1);
    resolved_callback(&assessment, "rstyle_core::facts::{impl#110}::eq");
    let helper = local(&programs[key(&programs, "rstyle_core::facts::{impl#110}::eq")]);
    assert_eq!(helper.loop_body_entries, Bound::Const(0));
    assert_eq!(helper.calls.len(), 2);
    assert!(
        helper
            .calls
            .iter()
            .all(|call| matches!(call.target, Callee::Library(_)) && call.count == Bound::Const(1))
    );
    let composed = proven(&programs, case);
    assert_eq!(
        composed.workspace_invocations,
        Bound::Input(case.input.into())
    );
    assert!(
        composed
            .resolved_callees
            .iter()
            .any(|callee| callee.ends_with("rstyle_core::facts::{impl#110}::eq"))
    );
    assert!(!composed.library_steps.is_complete());
}

#[test]
fn exact_data_producers_capture_preserves_the_deferred_filter_callback_count() {
    exact_edge_scan(&PRODUCERS);
}

#[test]
fn exact_data_consumers_capture_preserves_the_deferred_filter_callback_count() {
    exact_edge_scan(&CONSUMERS);
}

fn contains(bound: &Bound, input: &str) -> bool {
    match bound {
        Bound::Input(name) => name == input,
        Bound::Add(items) | Bound::Mul(items) | Bound::Max(items) => {
            items.iter().any(|item| contains(item, input))
        }
        Bound::Log(value) => contains(value, input),
        _ => false,
    }
}

#[test]
fn exact_classes_capture_recovers_the_scan_and_keeps_its_real_helper_opaque() {
    let (programs, assessment) = analyzed(&CLASSES, 2);
    resolved_callback(&assessment, "rstyle_core::facts::{impl#101}::eq");
    resolved_callback(&assessment, "rstyle_core::wl::call_class");
    let helper = &programs[key(&programs, "rstyle_core::shape::normalize")];
    assert!(matches!(
        runtime_bounds::analyze(helper),
        Outcome::Unknown { .. }
    ));
    let outcome = composition(&programs, CLASSES.owner);
    let Composed::Partial(composed) = outcome else {
        panic!("{outcome:?}")
    };
    assert!(contains(&composed.loop_body_entries, CLASSES.input));
    assert!(!composed.loop_body_entries.is_complete());
    assert!(!composed.workspace_invocations.is_complete());
    assert!(composed.opaque_factors.iter().any(|factor| {
        factor.site == "shape::normalize" && factor.reason.contains("Local bounds")
    }));
    assert!(
        composed
            .opaque_factors
            .iter()
            .all(|factor| !factor.reason.contains("No captured summary"))
    );
}
