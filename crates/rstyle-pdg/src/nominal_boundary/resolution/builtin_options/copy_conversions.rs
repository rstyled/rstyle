//! Exact conversion ownership replaces coarse admission, never lexical lookup.
use super::*;

impl Environment {
    pub(in crate::nominal_boundary::resolution) fn verify_conversion_result_source(
        &self,
        facts: &HirFnFacts,
        source: ResultSource<'_>,
        receipt: &CheckedCopyConversion,
    ) -> Result<(), String> {
        self.check_context(facts)?;
        crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Conversion Result")?;
        let namespace = receipt.checked_namespace(facts, self)?;
        let file = facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            .ok_or("Missing conversion Result source file")?;
        let mut found = 0;

        for module in self.modules.iter().filter(|module| {
            module.path == std::path::Path::new(file) && module.namespace == namespace
        }) {
            found += self.verify_conversion_result_module(facts, &source, module)?;
        }
        if found != 1 {
            return Err("Conversion Result lacks one exact checked original member".into());
        }
        Ok(())
    }

    fn verify_conversion_result_module(
        &self,
        facts: &HirFnFacts,
        source: &ResultSource<'_>,
        module: &census::Module,
    ) -> Result<usize, String> {
        let bytes = self.source(&module.path)?.bytes();
        let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
        let mut found = 0;

        for item in &module.items {
            let syn::Item::Impl(item) = item else {
                continue;
            };
            for member in &item.items {
                let syn::ImplItem::Fn(method) = member else {
                    continue;
                };
                if !(FunctionSource {
                    vis: &method.vis,
                    sig: &method.sig,
                    body: &method.block,
                })
                .exact(bytes, facts)
                {
                    continue;
                }
                // The receipt binds this complete original member, implementation,
                // source census, build and selected core trait.
                Scope {
                    environment: self,
                    module,
                    blocks: vec![&method.block],
                    configuration: &configuration,
                    facts,
                }
                .verify_result(source.path, source.variant)?;
                found += 1;
            }
        }
        Ok(found)
    }
}
