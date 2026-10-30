use super::*;

#[test]
fn density_classifies_external_helpers_at_two_edges_without_mutation_authority() {
    let policy = format!("{POLICY}\n[side_effects.paths]\ntime=['std::thread::yield_now']\n");
    let source = "fn leaf(value:u32){std::thread::yield_now();} pub fn caller(value:u32){leaf(value);leaf(2);} pub fn outer(){caller(1);}";
    let directory = workspace(source, &policy);
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    measured(&report, "leaf", 1000, "finding");
    measured(&report, "caller", 1000, "finding");
    assert_eq!(
        density(&report, "caller")["event_inventory"]["counts"]["time"],
        2
    );
    let caller = super::super::function(&report, "caller");
    assert_eq!(caller["evidence"]["coverage"], "incomplete");
    assert_eq!(density(&report, "outer")["complete"], false);
    assert!(density(&report, "outer")["measurement"]["actual"].is_null());
}

#[test]
fn density_helper_classification_rejects_hidden_work_borrows_unknowns_and_cycles() {
    let policy = format!("{POLICY}\n[side_effects.paths]\ntime=['std::thread::yield_now']\n");
    for source in [
        "fn leaf(value:&mut u32){std::thread::yield_now();} pub fn caller(value:&mut u32){leaf(value);}",
        "fn leaf(){std::thread::yield_now();std::hint::spin_loop();} pub fn caller(){leaf();}",
        "macro_rules! opaque {()=>{()}} fn leaf(){std::thread::yield_now();opaque!();} pub fn caller(){leaf();}",
        "fn leaf(){std::thread::yield_now();leaf();} pub fn caller(){leaf();}",
        "unsafe fn leaf(){std::thread::yield_now();} pub fn caller(){unsafe{leaf();}}",
    ] {
        let directory = workspace(source, &policy);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{source}: {}",
            stdout(&output)
        );
        assert_eq!(density(&report, "caller")["complete"], false, "{source}");
        assert!(
            density(&report, "caller")["measurement"]["actual"].is_null(),
            "{source}"
        );
    }
}

#[test]
fn density_external_helper_replay_reclassifies_without_source_and_obeys_budget() {
    let policy = format!("{POLICY}\n[side_effects.paths]\ntime=['std::thread::yield_now']\n");
    let directory = workspace(
        "fn leaf(){std::thread::yield_now();} pub fn caller(){leaf();}",
        &policy,
    );
    let (output, original) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    let (output, limited) = requested(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(density(&limited, "caller")["complete"], false);
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let output = replay(directory.path(), &original, "compact");
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let replayed: Value = serde_json::from_slice(&output.stdout).unwrap();
    measured(&replayed, "caller", 1000, "finding");
    std::fs::write(directory.path().join("effects.toml"), POLICY).unwrap();
    let output = replay(directory.path(), &original, "compact");
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let replayed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(density(&replayed, "caller")["complete"], false);
    assert!(density(&replayed, "caller")["measurement"]["actual"].is_null());
}

#[test]
fn density_helper_source_alias_does_not_gain_authority_from_normalized_u32() {
    let policy = format!("{POLICY}\n[side_effects.paths]\ntime=['std::thread::yield_now']\n");
    let source = "type Word=u32; fn leaf(value:Word){std::thread::yield_now();} pub fn caller(value:u32){leaf(value);}";
    let directory = workspace(source, &policy);
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let leaf = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            function["facts"]["name"]
                .as_str()
                .unwrap()
                .ends_with("leaf")
        })
        .unwrap();
    assert_eq!(
        leaf["facts"]["signature"]["params"][0]["ty"]["normalized"],
        "u32"
    );
    assert_eq!(density(&report, "caller")["complete"], false);
    assert!(density(&report, "caller")["measurement"]["actual"].is_null());
}

#[test]
fn density_retained_helper_cannot_complete_an_owner_after_observation_budget_exhaustion() {
    let policy = format!("{POLICY}\n[side_effects.paths]\ntime=['std::thread::yield_now']\n");
    let source =
        "fn leaf(){std::thread::yield_now();} pub fn first(){leaf();} pub fn second(){leaf();}";
    let directory = workspace(source, &policy);
    let (output, report) = requested(directory.path(), &["--max-analysis-functions", "2"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    measured(&report, "leaf", 1000, "finding");
    let wrappers = [density(&report, "first"), density(&report, "second")];
    assert_eq!(
        wrappers
            .iter()
            .filter(|entry| entry["complete"] == true)
            .count(),
        1
    );
    let exhausted = wrappers
        .iter()
        .find(|entry| entry["complete"] == false)
        .unwrap();
    assert!(exhausted["measurement"]["actual"].is_null());
    assert_eq!(exhausted["measurement"]["outcome"], "not_evaluated");
    assert_eq!(
        report["mutation_effects"]["units"][0]["statistics"]["observations"],
        2
    );
}
