use super::*;
use crate::clones::review::{CompilationTarget, Package, Versions};

fn locator(name: &str, kind: &str) -> Locator {
    Locator {
        package: Package {
            identity: "Cargo.toml".into(),
            name: "sample".into(),
            version: "1.0.0".into(),
        },
        source: "src/lib.rs".into(),
        qualified_name: format!("sample::{name}"),
        declaration_kind: kind.into(),
        discriminator: "unique-declaration".into(),
    }
}

fn context() -> BuildContext {
    BuildContext {
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
        extraction_policy: "source-local/1".into(),
    }
}

fn declaration(name: &str, kind: &str, source: &str) -> Declaration {
    Declaration {
        locator: locator(name, kind),
        source: source.into(),
    }
}

fn selection(owner: &str, method: &str) -> Selection {
    Selection {
        owner: declaration(owner, "struct", &format!("struct {owner};")),
        method: declaration(
            &format!("{owner}::{method}"),
            "method",
            &format!("fn {method}(&self)->u32"),
        ),
        origin: Origin::Inherent,
    }
}

fn member(name: &str) -> Member {
    Member {
        locator: locator(name, "free-function"),
        context: context(),
        source: format!("fn {name}(arg:&Owner)->(u32,u32) {{(arg.first(),arg.second())}}"),
    }
}

fn snapshot() -> Snapshot {
    Snapshot {
        versions: Versions {
            detector: "ordered-local-capability".into(),
            detector_version: 1,
            identity_version: 1,
            locator_version: 1,
            context_version: 1,
        },
        members: vec![member("alpha"), member("beta")],
        proposal: Proposal::OrderedCapability {
            roles: vec![
                CapabilityRole {
                    left: selection("Alpha", "used"),
                    right: selection("Beta", "size"),
                },
                CapabilityRole {
                    left: selection("Alpha", "limit"),
                    right: selection("Beta", "capacity"),
                },
            ],
        },
    }
}

fn dismissed(snapshot: &Snapshot) -> Vec<Entry> {
    let mut entries = vec![];
    record_dismissal(
        &mut entries,
        &LiveFinding::checked(snapshot).unwrap(),
        "reviewed".into(),
    )
    .unwrap();
    entries
}

fn stale(entries: &[Entry], snapshot: &Snapshot) -> StaleReason {
    let current = LiveFinding::checked(snapshot).unwrap();
    let Lookup::Stale(found) = checked(entries).unwrap().lookup(&current) else {
        panic!("expected stale")
    };
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].entry.reason, "reviewed");
    found[0].reason
}

#[test]
fn exact_review_roundtrip_borrows_entry_without_matching_policy() {
    let snapshot = snapshot();
    let entries = dismissed(&snapshot);
    let bytes = serde_json::to_string(&entries).unwrap();
    assert!(!bytes.contains("matching_policy"));
    assert!(!bytes.contains("source_owner"));
    let loaded: Vec<Entry> = serde_json::from_str(&bytes).unwrap();
    let store = checked(&loaded).unwrap();
    let live = LiveFinding::checked(&snapshot).unwrap();
    for _ in 0..2 {
        let Lookup::HiddenExact(entry) = store.lookup(&live) else {
            panic!("expected hidden")
        };
        assert!(std::ptr::eq(entry, &loaded[0]));
    }
}

#[test]
fn original_signature_role_and_origin_changes_remain_visible_stale() {
    let original = snapshot();
    let entries = dismissed(&original);
    for attack in 0..4 {
        let mut current = original.clone();
        let Proposal::OrderedCapability { roles } = &mut current.proposal else {
            unreachable!()
        };
        match attack {
            0 => current.members[0].source.push_str(" changed caller"),
            1 => roles[0].left.method.source = "fn used(&mut self)->u32".into(),
            2 => roles.swap(0, 1),
            _ => {
                roles[0].left.origin = Origin::LocalTrait {
                    declaration: Box::new(declaration(
                        "Metric",
                        "trait",
                        "trait Metric { fn used(&self)->u32; }",
                    )),
                    member: "used".into(),
                }
            }
        }
        assert_eq!(stale(&entries, &current), StaleReason::Body);
    }
}

#[test]
fn context_versions_and_membership_changes_are_distinct() {
    let original = snapshot();
    let entries = dismissed(&original);
    let mut current = original.clone();
    for member in &mut current.members {
        member.context.debug_assertions = false;
    }
    assert_eq!(stale(&entries, &current), StaleReason::Context);
    current = original.clone();
    current.versions.detector_version += 1;
    assert_eq!(stale(&entries, &current), StaleReason::Versions);
    current = original.clone();
    current.members[1].locator.qualified_name = "sample::gamma".into();
    assert_eq!(stale(&entries, &current), StaleReason::Membership);
}

#[test]
fn changed_family_or_unrelated_locator_never_reuses_dismissal() {
    let original = snapshot();
    let entries = dismissed(&original);
    let mut unrelated = original.clone();
    unrelated.members[0].locator.qualified_name = "sample::gamma".into();
    unrelated.members[1].locator.qualified_name = "sample::omega".into();
    assert!(matches!(
        checked(&entries)
            .unwrap()
            .lookup(&LiveFinding::checked(&unrelated).unwrap()),
        Lookup::VisibleNew
    ));
    let dispatch = Snapshot {
        versions: original.versions.clone(),
        members: vec![original.members[0].clone()],
        proposal: Proposal::EnumDispatch {
            enumeration: declaration("Item", "enum", "enum Item { A(Alpha), B(Beta) }"),
            arms: vec![
                DispatchArm {
                    variant: "A".into(),
                    selection: selection("Alpha", "used"),
                },
                DispatchArm {
                    variant: "B".into(),
                    selection: selection("Beta", "size"),
                },
            ],
        },
    };
    assert!(matches!(
        checked(&entries)
            .unwrap()
            .lookup(&LiveFinding::checked(&dispatch).unwrap()),
        Lookup::VisibleNew
    ));
}

#[test]
fn duplicate_or_malformed_entries_cannot_load_or_publish() {
    let original = snapshot();
    let mut entries = dismissed(&original);
    let live = LiveFinding::checked(&original).unwrap();
    assert_eq!(
        record_dismissal(&mut entries, &live, "duplicate".into()),
        Err(Error::DuplicateReview)
    );
    assert_eq!(
        record_dismissal(&mut entries, &live, " ".into()),
        Err(Error::InvalidReason)
    );
    entries.push(entries[0].clone());
    assert_eq!(validate_entries(&entries), Err(Error::DuplicateReview));
    let mut malformed = original.clone();
    malformed.members.swap(0, 1);
    assert!(LiveFinding::checked(&malformed).is_err());
    let mut malformed = original.clone();
    malformed.members[0].locator.source = "../lib.rs".into();
    assert!(LiveFinding::checked(&malformed).is_err());
    let mut malformed = original.clone();
    malformed.members[0].context.cfgs = vec!["unix".into(), "unix".into()];
    assert!(LiveFinding::checked(&malformed).is_err());
    let mut malformed = original;
    let Proposal::OrderedCapability { roles } = &mut malformed.proposal else {
        unreachable!()
    };
    roles[1].left.method = roles[0].left.method.clone();
    assert!(LiveFinding::checked(&malformed).is_err());
}

#[test]
fn wire_rejects_proof_fields_growth_and_unknown_policies() {
    let entries = dismissed(&snapshot());
    for field in ["trusted", "membership", "matching_policy"] {
        let mut value = serde_json::to_value(&entries).unwrap();
        value[0][field] = serde_json::json!(true);
        assert!(serde_json::from_value::<Vec<Entry>>(value).is_err());
    }
    let mut value = serde_json::to_value(&entries).unwrap();
    value[0]["finding"]["members"][0]["context"]["matching_policy"] = serde_json::json!({});
    assert!(serde_json::from_value::<Vec<Entry>>(value).is_err());
}
