use super::propagation::checked;
use super::*;

const GENERATED: &str = r#"
macro_rules! make {
    () => {
        pub fn generated_empty() {}
        pub fn generated_write(target: &mut u32) {*target=1; *target=2;}
        pub fn generated_io() {let _ = std::fs::read("/unused");}
        pub fn generated_dynamic(callback: fn()) {callback();}
        pub fn generated_unsafe(value: *const u32) -> u32 {unsafe {*value}}
    };
}
make!();
pub fn original() {}
"#;

fn unit(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]
}

fn generated<'a>(report: &'a Value, name: &str) -> &'a Value {
    unit(report)["generated_effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap()
}

fn kinds(row: &Value, field: &str) -> Vec<String> {
    let mut kinds = row[field]
        .as_array()
        .unwrap()
        .iter()
        .map(|effect| effect["kind"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    kinds.sort();
    kinds
}

fn unavailable(report: &Value, expected: usize) {
    let unit = unit(report);
    assert_eq!(report["compiler_advisory"]["complete"], false);
    assert_eq!(unit["unavailable"].as_array().unwrap().len(), expected);
    assert_eq!(unit["observations"].as_array().unwrap().len(), 1);
    assert_eq!(unit["evaluation"]["summary"]["functions"], 1);
    for row in unit["generated_effects"].as_array().unwrap() {
        assert!(
            unit["unavailable"]
                .as_array()
                .unwrap()
                .iter()
                .any(|missing| { missing["key"] == row["key"] })
        );
        for field in [
            "source",
            "statements",
            "complexity",
            "density",
            "evaluation",
        ] {
            assert!(row.get(field).is_none(), "{field}: {row}");
        }
    }
}

fn generated_owner_locations(report: &Value) {
    let rows = unit(report)["generated_effects"].as_array().unwrap();
    assert_eq!(rows.len(), 5);
    let keys = rows
        .iter()
        .map(|row| row["key"].to_string())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(keys.len(), 5);
    for row in rows {
        assert_eq!(row["metrics"], "rstyle.compiler-generated-effects/2");
        assert_eq!(row["path"], "src/lib.rs");
        assert!(row["line"].as_u64().unwrap() > 0);
        assert!(row["column"].as_u64().unwrap() > 0);
    }
}

#[test]
fn generated_direct_effects_reuse_collectors_without_inventing_source_measurements() {
    let (_directory, output, report) = capture(GENERATED, &[]);
    checked(&output, 2);
    unavailable(&report, 5);
    generated_owner_locations(&report);
    let empty = generated(&report, "generated_empty");
    for field in ["mutations", "calls", "intrinsic_effects", "direct_effects"] {
        assert_eq!(empty[field], serde_json::json!([]), "{field}");
    }
    let write = generated(&report, "generated_write");
    assert_eq!(write["mutations"].as_array().unwrap().len(), 2);
    assert!(
        write["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|event| event["escapes"] == true)
    );
    assert_eq!(kinds(write, "direct_effects"), ["mutation"]);
    let io = generated(&report, "generated_io");
    assert!(
        io["calls"]
            .as_array()
            .unwrap()
            .iter()
            .any(|call| call["callee"] == "std::fs::read")
    );
    assert_eq!(kinds(io, "direct_effects"), ["io"]);
    let dynamic = generated(&report, "generated_dynamic");
    assert!(
        dynamic["calls"]
            .as_array()
            .unwrap()
            .iter()
            .any(|call| call["dynamic"] == true)
    );
    assert!(
        kinds(dynamic, "direct_effects")
            .iter()
            .any(|kind| kind == "unknown")
    );
    let unsafe_body = generated(&report, "generated_unsafe");
    assert_eq!(kinds(unsafe_body, "intrinsic_effects"), ["unsafe"]);
    assert_eq!(kinds(unsafe_body, "direct_effects"), ["unsafe"]);
}

#[test]
fn authored_functions_and_generated_local_calls_keep_separate_direct_effects() {
    let source = r#"
        pub fn original() {let _ = std::fs::read("/unused");}
        macro_rules! make {()=>{pub fn generated_caller() {original();}}}
        make!();
    "#;
    let (_directory, output, report) = capture(source, &[]);
    checked(&output, 2);
    unavailable(&report, 1);
    let rows = unit(&report)["generated_effects"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "generated_caller");
    assert_eq!(rows[0]["direct_effects"], serde_json::json!([]));
    assert_eq!(
        unit(&report)["evaluation"]["summary"]["effect_counts"]["io"],
        1
    );
}
