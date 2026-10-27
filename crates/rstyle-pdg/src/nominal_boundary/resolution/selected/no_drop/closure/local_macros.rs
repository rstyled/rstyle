//! Source-only macro expansion retains every selected output in the cleanup walk.
use super::*;

impl Closure<'_> {
    pub(super) fn local_macro(&mut self, mac: &syn::Macro) -> Result<(), String> {
        let first = mac
            .path
            .segments
            .first()
            .ok_or("Empty cleanup macro path")?;

        if self.ownership_engine
            && self.scopes.iter().skip(1).any(|scope| {
                scope.contains_key(&first.ident.to_string()) || scope.contains_key("*")
            })
        {
            return Err(
                "Cleanup macro has unresolved nested import or declaration shadowing".into(),
            );
        }
        let definition = mac
            .path
            .get_ident()
            .and_then(|name| self.definitions.get(&name.to_string()))
            .cloned();
        if let Some(definition) = definition {
            return self.expand_definition(&definition, mac);
        }
        if self.external_macro(mac)? {
            return Ok(());
        }

        self.imported_or_builtin_macro(mac)
    }

    fn imported_or_builtin_macro(&mut self, mac: &syn::Macro) -> Result<(), String> {
        let imported = self.imported_macro(mac)?;
        if let Some(definition) = imported {
            return self.expand_definition(&definition, mac);
        }
        self.builtin_macro(mac)
    }

    fn external_macro(&mut self, mac: &syn::Macro) -> Result<bool, String> {
        if !self.external_macros {
            return Ok(false);
        }
        let Some(environment) = self.dependency_sources else {
            return Ok(false);
        };
        let names = crate::source_evidence::plain_relative_path(&mac.path)
            .ok_or("External cleanup macro requires an original relative path")?;
        let Some(expressions) =
            environment.external_macro_expressions(&self.enums.namespace, &names, mac)?
        else {
            return Ok(false);
        };
        if self.generated_context != 0 || self.enums.unindexed || self.unknown_glob() {
            return Err("External cleanup macro lacks original unambiguous lexical scope".into());
        }

        self.begin_expansion()?;
        // These are original invocation arguments, not generated template code.
        self.generated_context -= 1;
        for expression in &expressions {
            self.visit_expr(expression);
        }
        Ok(true)
    }

    fn expand_definition(
        &mut self,
        definition: &syn::ItemMacro,
        mac: &syn::Macro,
    ) -> Result<(), String> {
        let tokens = if self.ownership_engine {
            macros::expand_ownership(
                definition,
                mac,
                self.edition
                    .ok_or("Missing local macro definition edition")?,
            )?
        } else {
            macros::expand(definition, mac)?
        };
        self.expansion(tokens)
    }

    fn imported_macro(&self, mac: &syn::Macro) -> Result<Option<syn::ItemMacro>, String> {
        let Some(resolver) = &self.ownership_macros else {
            return Ok(None);
        };
        if self.enums.unindexed {
            return Err("Cleanup imported macro has no original indexed namespace".into());
        }
        let first = mac
            .path
            .segments
            .first()
            .ok_or("Empty cleanup macro path")?;

        let first = first.ident.to_string();

        if self.unknown_glob()
            || self
                .scopes
                .iter()
                .skip(1)
                .any(|scope| scope.contains_key(&first))
        {
            return Err(format!(
                "Cleanup imported macro {first} has unresolved lexical shadowing (unknown glob: {})",
                self.unknown_glob()
            ));
        }
        if self.library_macro_imports
            && resolver.library_invocation(&self.enums.namespace, mac, self.library)?
        {
            return Ok(None);
        }
        resolver.resolve(&self.enums.namespace, mac)
    }
}
