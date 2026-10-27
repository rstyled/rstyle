//! Independent raw source and selected-implementation expectations; no permission.
use super::*;
const GENERATED: &str = "rstyle.rust.generated-conversion-member";
const SOURCE: &str = r#"
macro_rules! conversions {
    ($name:ident, $error:ident, $input:ty) => {
        pub struct $name(u64);
        pub struct $error { attempted: u64 }
        impl TryFrom<$input> for $name {
            type Error = $error;
            fn try_from(value: $input) -> Result<$name, $error> {
                if value as u64 > 7 { return Err($error { attempted: value as u64 }); }
                Ok($name(value as u64))
            }
        }
    };
}
conversions!(N16, E16, u16);
conversions!(N32, E32, u32);
conversions!(N64, E64, u64);
conversions!(Native, NativeError, usize);
#[cfg(any())] conversions!(Inactive, InactiveError, u32);
macro_rules! generic {
    () => {
        pub struct Generic<T>(T);
        impl<T> TryFrom<Option<T>> for Generic<T> {
            type Error = ();
            fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
                match value { Some(value) => Ok(Self(value)), None => Err(()) }
            }
        }
    };
}
generic!();
mod lookalike {
    pub trait TryFrom<T>: Sized { type Error; fn try_from(value: T) -> Result<Self, Self::Error>; }
    conversions!(Custom, CustomError, u16);
}
"#;

#[test]
fn generated_unsigned_members_retain_template_anchors_and_actual_core_selection() {
    let functions = captured_source("hir-data-v3", SOURCE);
    for input in ["u16", "u32", "u64", "usize"] {
        let member = functions
            .iter()
            .find(|facts| {
                facts
                    .name
                    .replace("std::", "core::")
                    .ends_with(&format!("core::convert::TryFrom<{input}>>::try_from"))
            })
            .unwrap();
        let raw = &member.build.frontend_options[GENERATED];
        assert_eq!(raw["status"], "observed");
        assert_eq!(member.build.frontend_options[KEY]["status"], "unsupported");
        check_member(&raw["observation"], input);
        assert_ne!(classify(member).status, EligibilityVerdict::Eligible);
    }
    let records: Vec<_> = functions
        .iter()
        .filter_map(|f| f.build.frontend_options.get(GENERATED))
        .collect();
    assert_eq!(records.len(), 5);
    assert_eq!(
        records
            .iter()
            .filter(|r| r["status"] == "unsupported")
            .count(),
        1
    );
    assert!(
        functions
            .iter()
            .filter(|f| f.name.contains("lookalike"))
            .all(|f| !f.build.frontend_options.contains_key(GENERATED))
    );
}

fn check_member(observed: &Value, input: &str) {
    assert_eq!(
        observed["identity"],
        "rstyle.rust.generated-conversion-member-observation/1"
    );
    assert_eq!(observed["admission_authority"], false);
    assert_eq!(observed["method"], observed["selected"]);
    assert_eq!(
        observed["requested_signature"],
        observed["selected_signature"]
    );
    assert_eq!(observed["trait_definition"]["crate"], "core");
    assert_eq!(
        observed["trait_arguments"][1]["identity"],
        serde_json::json!({"kind":"builtin","name":input})
    );
    let error = &observed["associated_error"];
    assert_eq!(error["declared_type"], error["normalized_projection"]);
    assert_eq!(slice(&error["source"]), "type Error = $error;");
    assert!(
        slice(&observed["implementation_source"]).starts_with("impl TryFrom<$input> for $name")
    );
    assert!(slice(&observed["method_source"]).starts_with("fn try_from(value: $input)"));
    assert_eq!(observed["members"].as_array().unwrap().len(), 2);
    assert_eq!(
        observed["members"][0]["definition"],
        error["implementation_member"]
    );
    assert_eq!(observed["members"][1]["definition"], observed["method"]);
}

fn slice(source: &Value) -> &str {
    let start = source["span"]["start"].as_u64().unwrap() as usize;
    let end = source["span"]["end"].as_u64().unwrap() as usize;
    &SOURCE[start..end]
}

#[test]
fn generated_conversion_capture_does_not_backport_to_historical_profiles() {
    let functions = captured_source("hir-data-v2", SOURCE);
    assert!(
        functions
            .iter()
            .all(|f| !f.build.frontend_options.contains_key(GENERATED))
    );
}
