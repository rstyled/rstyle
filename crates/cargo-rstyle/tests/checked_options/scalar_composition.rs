use super::*;

#[test]
fn scalar_option_values_compose_with_effects_rebinding_and_partial_flow() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), composition_source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 11);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        assert_eq!(
            function["outcome"],
            if matches!(
                name,
                "guarded" | "wildcard" | "mutable" | "generic" | "shared"
            ) {
                "excluded"
            } else {
                "admitted"
            },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
}

fn composition_source() -> &'static str {
    r#"
        pub fn identity(value: Option<i128>) -> Option<i128> { value }
        pub fn choose(pick: bool, value: i128) -> Option<i128> {
            if pick { Some(value) } else { None }
        }
        pub fn saved(mut value: Option<i128>, other: Option<i128>) -> i128 {
            let old = identity(value); value = other;
            match old { None => 0, Some(payload) => payload }
        }
        pub fn effects(mut byte: u8) -> u8 {
            let option = Some({ byte = 7; byte }); byte = 9;
            match option { Some(payload) => payload, None => byte }
        }
        pub fn partial(stop: bool, byte: u8) -> Option<u8> {
            Some({ if stop { return None; } byte })
        }
        pub fn boolean(value: Option<bool>) -> bool {
            match value { None => false, Some(payload) => payload }
        }
        pub fn guarded(value: Option<u8>) -> u8 {
            match value { Some(payload) if payload > 0 => payload, _ => 0 }
        }
        pub fn wildcard(value: Option<u8>) -> u8 {
            match value { Some(payload) => payload, _ => 0 }
        }
        pub fn mutable(value: Option<u8>) -> u8 {
            match value { Some(mut payload) => { payload = 1; payload }, None => 0 }
        }
        pub fn generic(byte: u8) -> Option<u8> { Some::<u8>(byte) }
        pub fn shared(byte: &u8) -> Option<&u8> { Some(byte) }
    "#
}
