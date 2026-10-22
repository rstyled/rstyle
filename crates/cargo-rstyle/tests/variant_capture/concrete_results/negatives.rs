use super::*;

const SOURCE: &str = r#"
#[derive(Clone, Debug)] pub struct E(());
impl E { pub fn new() -> E { E(()) } }
pub fn honest() -> Result<(), E> { Ok(()) }
pub fn local_item() -> Result<(), E> {
    fn Ok(_: ()) -> Result<(), E> { core::result::Result::Ok(()) }
    Ok(())
}
pub fn owned(error: E) -> E { error }
pub fn result_parameter(value: Result<(), E>) -> Result<(), E> { value }
pub fn stored() -> Result<(), E> { let value = Err(E::new()); value }
pub fn discarded() -> Result<(), E> { let _ = E::new(); Ok(()) }
pub fn payload() -> Result<u8, E> { Ok(1) }
pub fn consumed() -> bool { match honest() { Ok(()) => true, Err(_) => false } }
mod drop_owner {
    #[derive(Clone, Debug)] pub struct D(());
    impl Drop for D { fn drop(&mut self) {} }
    pub fn with_drop() -> Result<(), D> { Err(D(())) }
}
mod shadow {
    use super::E;
    #[allow(non_snake_case)]
    fn Ok(_: ()) -> Result<(), E> { core::result::Result::Ok(()) }
    pub fn selected() -> Result<(), E> { Ok(()) }
}
mod fake {
    enum Result<T, E> { Ok(T), Err(E) }
    pub fn wrong_type() -> Result<(), super::E> { Result::Ok(()) }
}
"#;

#[test]
fn no_drop_permission_does_not_enable_owned_storage_extraction_or_dropped_errors() {
    let (_directory, report) = capture(SOURCE, &[]);
    assert_eq!(function(&report, "honest")["outcome"], "admitted");
    for name in [
        "owned",
        "result_parameter",
        "stored",
        "discarded",
        "payload",
        "consumed",
        "drop_owner::with_drop",
        "fake::wrong_type",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
    reject_shadow_authority(&report);
}

fn reject_shadow_authority(report: &Value) {
    let honest = function(report, "honest");
    let mut genuine_body = honest["facts"]["body"].clone();
    let genuine: rstyle_pdg::HirNode =
        serde_json::from_value(variant_mut(&mut genuine_body).unwrap().clone()).unwrap();
    let genuine_facts: rstyle_pdg::HirFnFacts =
        serde_json::from_value(honest["facts"].clone()).unwrap();
    rstyle_pdg::concrete_results::verify_constructor(&genuine, &genuine_facts).unwrap();

    for name in ["shadow::selected", "local_item"] {
        let expected = if name == "local_item" {
            "block-local item/macro"
        } else {
            "shadow"
        };
        reject_forged_constructor(function(report, name), &genuine, expected);
    }
}

fn reject_forged_constructor(entry: &Value, genuine: &rstyle_pdg::HirNode, expected: &str) {
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(entry["facts"].clone()).unwrap();
    let mut forged = facts.body.children.last().unwrap().clone();
    assert_eq!(forged.kind, "expr_call");
    assert!(
        forged
            .res
            .as_ref()
            .is_none_or(|resolution| resolution.kind != RESOLUTION)
    );
    assert_eq!(
        forged.ty.as_ref().unwrap().normalized,
        genuine.ty.as_ref().unwrap().normalized
    );
    forged.res = genuine.res.clone();
    forged.dispatch = None;
    let error = match rstyle_pdg::concrete_results::verify_constructor(&forged, &facts) {
        Ok(_) => panic!("local Ok function acquired builtin Result authority"),
        Err(error) => error,
    };
    assert!(error.contains(expected), "{error}");
}
