use super::*;

fn partial(report: &Value, generated: usize, io: u64) {
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["complete"], false);
    let unit = &advisory["units"][0];
    assert_eq!(unit["observations"].as_array().unwrap().len(), 1);
    assert_eq!(unit["unavailable"].as_array().unwrap().len(), generated);
    assert_eq!(
        unit["generated_effects"].as_array().unwrap().len(),
        generated
    );
    assert_eq!(unit["unresolved_callees"], serde_json::json!([]));
    assert_eq!(
        unit["evaluation"]["summary"]["effect_counts"]["io"]
            .as_u64()
            .unwrap_or(0),
        io,
    );
    assert_eq!(advisory["planning"]["known_functions"], 1);
    assert_eq!(advisory["planning"]["input_complete"], false);
}

#[test]
fn generated_trait_targets_export_only_effects_shared_by_every_implementation() {
    for (second, expected) in [("let _=std::fs::read(\"/unused\");", 1), ("", 0)] {
        let source = format!(
            r#"
pub struct First;
pub struct Second;
pub trait Action {{ fn act(&self); }}
macro_rules! implement {{()=>{{
    impl Action for First {{ fn act(&self) {{ let _=std::fs::read("/unused"); }} }}
    impl Action for Second {{ fn act(&self) {{ {second} }} }}
}}}}
implement!();
pub fn caller(value:&dyn Action) {{ value.act(); }}
"#,
        );
        let (_directory, output, report) =
            capture(&source, &["--compiler-advisory-only", "--compiler-plans"]);
        propagation::checked(&output, 2);
        partial(&report, 2, expected);
        let generated = report["compiler_advisory"]["units"][0]["generated_effects"]
            .as_array()
            .unwrap();
        for owner in generated {
            assert!(
                owner["connections"]["implements"]
                    .as_str()
                    .unwrap()
                    .ends_with("::Action::act")
            );
        }
    }
}

#[test]
fn a_generated_default_trait_body_supplies_effects_without_authored_metrics() {
    let source = r#"
pub struct Reader;
macro_rules! define {()=>{
    pub trait Action {
        fn fallback(&self) { let _=std::fs::read("/unused"); }
    }
}}
define!();
impl Action for Reader {}
pub fn caller() { Reader.fallback(); }
"#;
    let (_directory, output, report) =
        capture(source, &["--compiler-advisory-only", "--compiler-plans"]);
    propagation::checked(&output, 2);
    partial(&report, 1, 1);
    let owner = &report["compiler_advisory"]["units"][0]["generated_effects"][0];
    assert!(
        owner["connections"]["id"]
            .as_str()
            .unwrap()
            .ends_with("::Action::fallback")
    );
    assert!(owner["connections"]["implements"].is_null());
}
