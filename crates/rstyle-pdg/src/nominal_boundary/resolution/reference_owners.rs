//! Lexical nominal joins for original reference-instance observations only.
use super::*;

pub(super) fn verify(
    facts: &HirFnFacts,
    paths: &[(syn::TypePath, concrete_types::Nominal)],
) -> Result<(), String> {
    let environment = standalone_environment(facts)?;
    environment.check_input_owner(facts)?;
    let namespace = namespace(facts, &environment)?;
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let mut resolution = declaration_resolution(facts, &interface, environment)?;
    resolution.namespace = namespace;

    for (path, nominal) in paths {
        resolution.matches_nominal(path, nominal)?;
    }
    Ok(())
}

fn namespace(facts: &HirFnFacts, environment: &Environment) -> Result<Namespace, String> {
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Reference owner lacks original source file")?;
    let bytes = environment.source(std::path::Path::new(file))?;
    let mut found = None;

    for module in &environment.modules {
        if module.path != std::path::Path::new(file) {
            continue;
        }
        for item in &module.items {
            let syn::Item::Impl(item) = item else {
                continue;
            };
            for method in &item.items {
                let syn::ImplItem::Fn(method) = method else {
                    continue;
                };
                if exact_method(method, bytes.bytes(), facts, false)
                    && found.replace(module.namespace.clone()).is_some()
                {
                    return Err("Reference impl-method source owner is ambiguous".into());
                }
            }
        }
    }
    found.ok_or_else(|| "Reference owner is not an exact original impl method".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const METHOD: &str = "pub fn new() -> u8 { 0 }";

    fn fixture() -> (Environment, HirFnFacts) {
        let source = format!(
            "mod owner {{ pub struct Input; impl Input {{ {METHOD} }} }} mod other {{ pub struct Input; impl Input {{ pub fn new() -> u8 {{ 1 }} }} }}"
        );
        let mut census = (*environment::tests::census()).clone();
        census.files[0].bytes = source.clone().into();
        census.files[0].digest = concrete_types::digest(source.as_bytes());
        let environment =
            Environment::with_options(Arc::new(census), true, Some(&[]), false).unwrap();
        let mut facts: HirFnFacts =
            serde_json::from_str(include_str!("selected/function_cleanup_tests/facts.json"))
                .unwrap();
        facts.build.frontend_options.insert(
            "source_file".into(),
            serde_json::json!("/sample/src/lib.rs"),
        );
        let start = source.find(METHOD).unwrap() as u32;
        facts.fn_span = ByteSpan {
            start,
            end: start + METHOD.len() as u32,
        };
        facts.source.bytes = METHOD.into();
        (environment, facts)
    }

    #[test]
    fn namespace_requires_exact_method_range_and_unique_checked_module() {
        let (mut environment, mut facts) = fixture();
        assert_eq!(namespace(&facts, &environment).unwrap(), ["owner"]);
        facts.fn_span.end -= 1;
        assert!(namespace(&facts, &environment).is_err());
        facts.fn_span.end += 1;
        facts.source.bytes = "pub fn new() -> u8 { 1 }".into();
        assert!(namespace(&facts, &environment).is_err());
        facts.source.bytes = METHOD.into();
        let original = environment
            .modules
            .iter()
            .find(|module| module.namespace == ["owner"])
            .unwrap();
        let duplicate = census::Module {
            path: original.path.clone(),
            namespace: original.namespace.clone(),
            items: original.items.clone(),
            verified_macro_imports: original.verified_macro_imports.clone(),
            verified_external_imports: original.verified_external_imports.clone(),
        };
        environment.modules.push(duplicate);
        assert!(
            namespace(&facts, &environment)
                .unwrap_err()
                .contains("ambiguous")
        );
    }
}
