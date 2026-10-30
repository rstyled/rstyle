use super::*;
use crate::clones::review::{
    CompilationTarget, Package,
    exact_rules::{self, ReviewSnapshot},
    local_adapter,
};
use rstyle_pdg::ResolvedTy;

fn context() -> local::BuildContext {
    local::BuildContext {
        compiler_commit: "compiler".into(),
        edition: "2024".into(),
        data_layout: "layout".into(),
        compilation_target: CompilationTarget {
            name: "sample".into(),
            kind: vec!["lib".into()],
            test: false,
        },
        target_triple: "aarch64-unknown-linux-gnu".into(),
        pointer_width: 64,
        features: vec![],
        cfgs: vec!["unix".into()],
        opt_level: "0".into(),
        debug_assertions: true,
        overflow_checks: true,
        panic_strategy: "unwind".into(),
        extraction_policy: "hir-data/1".into(),
    }
}

fn declaration(name: &str) -> Rc<Declaration> {
    let ty = ResolvedTy {
        normalized: Some("u128".into()),
        observed: "u128".into(),
        spelling: "u128".into(),
    };
    Rc::new(Declaration {
        context: context(),
        locator: Locator {
            package: Package {
                identity: "Cargo.toml".into(),
                name: "sample".into(),
                version: "1.0.0".into(),
            },
            source: "src/lib.rs".into(),
            qualified_name: name.into(),
            declaration_kind: "compiler-function".into(),
            discriminator: "unique-declaration".into(),
        },
        signature: Signature {
            parameters: vec![ty.clone()],
            result: ty,
            method: None,
            abi: serde_json::from_value(serde_json::json!({"status":"known","value":"Rust"}))
                .unwrap(),
            generics: vec![],
            has_self: false,
            is_variadic: false,
            is_async: false,
            is_unsafe: false,
            is_const: false,
        },
        implements: None,
    })
}

fn snapshot() -> Snapshot {
    let node = serde_json::json!({"kind":"lit","ty":null,"callee_id":null,"callee_sig":null,"macro_name":null,"lit_kind":"int","detail":"7","line":0});
    Snapshot {
        versions: local_adapter::versions("compiler-patterns"),
        policy: Params::default(),
        hide_implements: Vec::new(),
        kind: CandidateKind::Parameterize,
        sites: vec![Member {
            declaration: declaration("sample::first"),
            body: Rc::new(Body {
                graph: serde_json::from_value(serde_json::json!({"nodes":[node],"edges":[]}))
                    .unwrap(),
                opaque_source: None,
            }),
        }],
        definitions: vec![],
    }
}

fn entries(current: &Snapshot) -> Vec<Entry> {
    let mut entries = Vec::new();
    exact_rules::record_dismissal(
        &mut entries,
        &exact_rules::LiveFinding::checked(current).unwrap(),
        "reviewed".into(),
    )
    .unwrap();
    entries
}

#[test]
fn shared_graph_owners_and_full_values_control_lookup_without_fingerprints() {
    let original = snapshot();
    let entries = entries(&original);
    assert!(Rc::ptr_eq(
        &original.sites[0].body,
        &entries[0].finding.sites[0].body
    ));
    assert!(Rc::ptr_eq(
        &original.sites[0].declaration,
        &entries[0].finding.sites[0].declaration
    ));
    let wire = serde_json::to_string(&entries).unwrap();
    assert!(!wire.contains("candidate_id") && !wire.contains("shape") && !wire.contains("sha256"));
    let decoded: Vec<Entry> = serde_json::from_str(&wire).unwrap();
    let current = exact_rules::LiveFinding::checked(&original).unwrap();
    assert!(matches!(
        exact_rules::ValidatedStore::new(&decoded)
            .unwrap()
            .lookup(&current),
        exact_rules::Lookup::HiddenExact(_)
    ));
    let mut changed = snapshot();
    Rc::get_mut(&mut changed.sites[0].body).unwrap().graph.nodes[0].detail = "8".into();
    let live = exact_rules::LiveFinding::checked(&changed).unwrap();
    assert!(matches!(
        exact_rules::ValidatedStore::new(&decoded)
            .unwrap()
            .lookup(&live),
        exact_rules::Lookup::Stale(_)
    ));
}

#[test]
fn policy_context_signature_kind_membership_and_opaque_changes_cannot_hide() {
    let original = snapshot();
    let entries = entries(&original);
    let mut variants = vec![snapshot(); 7];
    variants[0].policy.max_holes += 1;
    Rc::make_mut(&mut variants[1].sites[0].declaration)
        .context
        .overflow_checks = false;
    variants[2].kind = CandidateKind::GenericFn;
    variants[3].definitions.push(declaration("sample::method"));
    variants[4].sites[0].declaration = declaration("sample::first");
    Rc::get_mut(&mut variants[4].sites[0].declaration)
        .unwrap()
        .signature
        .is_unsafe = true;
    variants[5].sites[0].body = Rc::new(Body {
        graph: Pdg::default(),
        opaque_source: Some("changed closure".into()),
    });
    variants[6].hide_implements.push("not-matched".into());
    let store = exact_rules::ValidatedStore::new(&entries).unwrap();
    for changed in &variants {
        let live = exact_rules::LiveFinding::checked(changed).unwrap();
        assert!(matches!(store.lookup(&live), exact_rules::Lookup::Stale(_)));
    }
    let mut ambiguous = snapshot();
    ambiguous.sites.push(ambiguous.sites[0].clone());
    assert!(ambiguous.validate().is_err());
}

#[test]
fn review_wire_requires_complete_graph_metadata_and_policy() {
    let value = serde_json::to_value(entries(&snapshot())).unwrap();
    for field in [
        "ty",
        "callee_id",
        "callee_sig",
        "macro_name",
        "lit_kind",
        "detail",
        "line",
    ] {
        let mut changed = value.clone();
        changed[0]["finding"]["sites"][0]["body"]["graph"]["nodes"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            serde_json::from_value::<Vec<Entry>>(changed).is_err(),
            "{field}"
        );
    }
    let mut changed = value;
    changed[0]["finding"]["policy"]
        .as_object_mut()
        .unwrap()
        .remove("pair_coverage_milli");
    assert!(serde_json::from_value::<Vec<Entry>>(changed).is_err());
}

#[test]
fn store_byte_limits_prevent_partial_publication() {
    use crate::clones::review::{ReviewStore, baseline_io};
    let mut store = ReviewStore {
        schema_version: 8,
        compiler_pattern_entries: entries(&snapshot()),
        ..ReviewStore::default()
    };
    store.compiler_pattern_entries[0].reason = "x".repeat(baseline_io::MAX_BYTES);
    assert!(store.to_json().is_err());
    assert!(ReviewStore::from_json(&" ".repeat(baseline_io::MAX_BYTES + 1)).is_err());
}

fn cross_unit_snapshot() -> Snapshot {
    let mut snapshot = snapshot();
    let mut second = snapshot.sites[0].clone();
    let second_declaration = Rc::make_mut(&mut second.declaration);
    second_declaration.locator.qualified_name = "other::second".into();
    second_declaration.locator.package.identity = "other/Cargo.toml".into();
    second_declaration.locator.package.name = "other".into();
    second_declaration.context.compilation_target.name = "other".into();
    snapshot.sites.push(second);
    snapshot.sites.sort_by(|a, b| {
        (&a.declaration.locator, &a.declaration.context)
            .cmp(&(&b.declaration.locator, &b.declaration.context))
    });
    let mut definition = declaration("sample::method");
    Rc::make_mut(&mut definition)
        .context
        .compilation_target
        .name = "definitions".into();
    snapshot.definitions.push(definition);
    snapshot
}

#[test]
fn cross_unit_members_and_definitions_retain_context_in_reviews_and_status() {
    use crate::clones::review::{positive_workflow, status};
    struct Row(Result<Snapshot, String>);
    impl positive_workflow::Row for Row {
        type Snapshot = Snapshot;
        fn snapshot(&self) -> &Result<Snapshot, String> {
            &self.0
        }
    }
    let original = cross_unit_snapshot();
    let entries = entries(&original);
    let wire = serde_json::to_value(&entries).unwrap();
    assert!(wire[0]["finding"].get("context").is_none());
    assert_eq!(original.declarations().count(), 3);
    let decoded: Vec<Entry> = serde_json::from_value(wire).unwrap();
    assert_eq!(decoded, entries);
    let store = exact_rules::ValidatedStore::new(&decoded).unwrap();
    assert!(matches!(
        store.lookup(&exact_rules::LiveFinding::checked(&original).unwrap()),
        exact_rules::Lookup::HiddenExact(_)
    ));
    for definition in [false, true] {
        let mut changed = original.clone();
        let member = if definition {
            &mut changed.definitions[0]
        } else {
            &mut changed.sites[1].declaration
        };
        Rc::make_mut(member).context.overflow_checks = false;
        let live = exact_rules::LiveFinding::checked(&changed).unwrap();
        let exact_rules::Lookup::Stale(stale) = store.lookup(&live) else {
            panic!("Changed secondary member context must keep the finding visible")
        };
        assert_eq!(stale[0].reason, exact_rules::StaleReason::Context);
        let evaluated = positive_workflow::evaluate(
            &entries,
            &[Row(Ok(changed))],
            positive_workflow::Contract {
                requested: true,
                retain_records: true,
                family: status::Family::CompilerPattern,
                label: "Compiler proposal",
            },
        )
        .unwrap();
        assert_eq!(evaluated.records[0].state, status::State::ContextChanged);
    }
}

#[test]
fn cross_unit_baselines_require_every_member_context_and_key_each_target() {
    use crate::clones::review::{
        baseline_common::CompilerInventory, baseline_io::Artifact, compiler_pattern_baseline,
    };
    let original = cross_unit_snapshot();
    let contexts = original
        .declarations()
        .map(|member| member.context.clone())
        .collect();
    let mut baseline =
        compiler_pattern_baseline::Baseline::new(Rc::new(serde_json::json!({})), contexts).unwrap();
    baseline.entries.push(original.clone());
    baseline.validate().unwrap();
    baseline
        .contexts
        .retain(|context| context.compilation_target.name != "definitions");
    assert!(baseline.validate().is_err());
    let mut changed = original.clone();
    Rc::make_mut(&mut changed.definitions[0])
        .context
        .compilation_target
        .test = true;
    assert_ne!(original.baseline_key(), changed.baseline_key());
    Rc::make_mut(&mut changed.sites[1].declaration)
        .context
        .compiler_commit
        .clear();
    assert!(changed.validate().is_err());
    let mut missing = serde_json::to_value(&original).unwrap();
    missing["sites"][1]["declaration"]
        .as_object_mut()
        .unwrap()
        .remove("context");
    assert!(serde_json::from_value::<Snapshot>(missing).is_err());
}
