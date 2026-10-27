use super::*;
use crate::dependency_namespaces::capture::{self, UnitCapture};
use serde_json::{Value, json};

fn facts() -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.build.frontend_options.clear();
    facts
}

fn compiled() -> Value {
    json!({"identity":"rstyle.rust.compiled-crate/1", "crate_name":"producer",
        "stable_crate_id":"producer-id", "instance":{"type_arguments":[], "const_arguments":[]}})
}

fn import() -> Import {
    let definition = |local: bool| {
        json!({"path":"prelude", "definition_identity":"prelude-id",
        "crate":if local {"consumer"} else {"producer"},
        "stable_crate_id":if local {"consumer-id"} else {"producer-id"}, "local":local})
    };
    let source = json!({"path":"/original/lib.rs", "digest":"xxh3-64:0000000000000000",
        "span":{"start":0,"end":1}});
    serde_json::from_value(json!({"site":{"definition":definition(true),
        "owner":definition(true),"declaration":source,"leaf":source,"scope":[],
        "path":["producer","prelude"],"binding":{"kind":"glob"},"public":false,
        "target":definition(false),"target_kind":"Mod"},
        "compiled_dependency":compiled(),"exports":[],"complete":false}))
    .unwrap()
}

fn consumer(import: &Import) -> UnitStore {
    let mut facts = facts();
    facts
        .build
        .frontend_options
        .insert_data(Arc::new(UnitCapture {
            identity: capture::IDENTITY.into(),
            imports: vec![import.clone()],
            declarations: Vec::new(),
            unsupported: Vec::new(),
        }));
    Builder::new(vec![facts]).unwrap().store
}

fn producer() -> UnitStore {
    let mut facts = facts();
    facts
        .build
        .frontend_options
        .insert("rstyle.rust.compiled-crate".into(), compiled());
    Builder::new(vec![facts]).unwrap().store
}

fn association(caller: &UnitStore, callee: &UnitStore, from: usize, to: usize) -> Value {
    json!({"identity":"rstyle.rust.compiled-dependency-association/1",
        "caller":{"unit":from,"namespace":caller.namespace},
        "callee":{"unit":to,"namespace":callee.namespace},
        "local_compiled_crate":compiled(),"compiled_dependency":compiled()})
}

#[test]
fn raw_observations_and_unassociated_owners_grant_no_namespace_view() {
    let import = import();
    let raw = consumer(&import);
    assert!(
        raw.functions()[0]
            .dependency_namespace(&import)
            .unwrap()
            .is_none()
    );
    let analysis = raw.analysis();
    assert!(
        analysis
            .function(0)
            .unwrap()
            .dependency_namespace(&import)
            .unwrap()
            .is_none()
    );
    let closure =
        DependencyClosure::from_units_with(vec![raw, producer()], &[], |_, _| Ok(())).unwrap();
    let analysis = closure.unit(0).unwrap().analysis();
    assert!(
        analysis
            .function(0)
            .unwrap()
            .dependency_namespace(&import)
            .unwrap()
            .is_none()
    );
}

#[test]
fn exact_checked_direct_edge_retains_source_owner_without_certification() {
    let import = import();
    let caller = consumer(&import);
    let callee = producer();
    let edge = association(&caller, &callee, 0, 1);
    let closure =
        DependencyClosure::from_units_with(vec![caller, callee], &[edge], |_, _| Ok(())).unwrap();
    let analysis = closure.unit(0).unwrap().analysis();
    let view = analysis.function(0).unwrap();
    let selected = view.dependency_namespace(&import).unwrap().unwrap();
    assert_eq!(selected.functions().len(), 1);
    assert!(
        selected.anonymous_prelude().is_err(),
        "an artifact edge cannot invent missing producer source observations"
    );
    assert_eq!(
        selected.functions()[0].build.frontend_options["rstyle.rust.compiled-crate"],
        compiled()
    );
    let mut changed = import.clone();
    changed.compiled_dependency["stable_crate_id"] = "different".into();
    assert!(view.dependency_namespace(&changed).is_err());
    let reference: &dyn EvidenceFacts = &view;
    assert!(
        (&reference)
            .dependency_namespace(&import)
            .unwrap()
            .is_some()
    );
}

#[test]
fn transitive_edge_and_failed_artifact_check_cannot_supply_direct_authority() {
    let import = import();
    let caller = consumer(&import);
    let middle = producer();
    let callee = producer();
    let edge = association(&middle, &callee, 1, 2);
    let closure =
        DependencyClosure::from_units_with(vec![caller, middle, callee], &[edge], |_, _| Ok(()))
            .unwrap();
    let analysis = closure.unit(0).unwrap().analysis();
    assert!(
        analysis
            .function(0)
            .unwrap()
            .dependency_namespace(&import)
            .unwrap()
            .is_none()
    );

    let caller = consumer(&import);
    let callee = producer();
    let edge = association(&caller, &callee, 0, 1);
    let rejected = DependencyClosure::from_units_with(vec![caller, callee], &[edge], |_, _| {
        Err("artifact mismatch".into())
    });
    assert_eq!(rejected.err().unwrap(), "artifact mismatch");
}
