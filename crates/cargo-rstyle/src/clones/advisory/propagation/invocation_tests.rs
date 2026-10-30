use super::*;
use crate::clones::identity;
use serde_json::json;

fn unit(name: &str, test: bool, suffix: &str, dependency: Option<&str>) -> capture::Unit {
    let mut invocation: Vec<String> = [
        "--crate-name",
        name,
        "--out-dir",
        "/captured/out",
        "-C",
        &format!("extra-filename=-{suffix}"),
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    if test {
        invocation.push("--test".into());
    }
    if let Some(path) = dependency {
        invocation.extend(["--extern".into(), format!("renamed={path}")]);
    }
    serde_json::from_value(json!({
        "extraction_profile": "r0",
        "target": {
            "package": name, "manifest_dir": format!("/{name}"),
            "source": format!("/{name}/src/lib.rs"), "name": name,
            "kind": ["lib"], "test": true
        },
        "test": test, "invocation": invocation,
        "directory": format!("/run/{name}/{suffix}"), "artifacts": []
    }))
    .unwrap()
}

fn links(units: &[capture::Unit]) -> (Vec<UnitId>, Routes) {
    let assignments = identity::assign(units).unwrap();
    let ids: Vec<_> = assignments.iter().map(|unit| unit.context.id).collect();
    let rows: Vec<_> = ids.iter().copied().zip(units).collect();
    (ids, invocation_links(&rows))
}

#[test]
fn check_and_test_callers_link_the_emitted_check_library() {
    let units = [
        unit("leaf", false, "check", None),
        unit("leaf", true, "test", None),
        unit(
            "caller",
            false,
            "caller-check",
            Some("/captured/out/libleaf-check.rmeta"),
        ),
        unit(
            "caller",
            true,
            "caller-test",
            Some("/captured/out/libleaf-check.rmeta"),
        ),
    ];
    let (ids, routes) = links(&units);
    assert_eq!(routes[&ids[2]], BTreeSet::from([ids[0]]));
    assert_eq!(routes[&ids[3]], BTreeSet::from([ids[0]]));
    assert!(routes[&ids[1]].is_empty());
}

#[test]
fn duplicate_aliases_and_wrong_output_paths_remain_unresolved() {
    let mut caller = unit(
        "caller",
        false,
        "caller",
        Some("/captured/out/libleaf-check.rmeta"),
    );
    caller
        .invocation
        .push("--extern=renamed=/other/libleaf-check.rmeta".into());
    let units = [unit("leaf", false, "check", None), caller];
    let (ids, routes) = links(&units);
    assert!(routes[&ids[1]].is_empty());
    let units = [
        unit("leaf", false, "check", None),
        unit(
            "caller",
            false,
            "caller",
            Some("/other/libleaf-check.rmeta"),
        ),
    ];
    let (ids, routes) = links(&units);
    assert!(routes[&ids[1]].is_empty());
}

#[test]
fn test_harness_output_cannot_supply_a_dependency_library() {
    let units = [
        unit("leaf", true, "test", None),
        unit(
            "caller",
            true,
            "caller",
            Some("/captured/out/libleaf-test.rmeta"),
        ),
    ];
    let (ids, routes) = links(&units);
    assert!(routes[&ids[1]].is_empty());
}
