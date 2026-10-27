//! Prove only that a complete predecessor preserves enclosing textual lookup.
//! Method bodies still need their ordinary source/effect/cleanup proofs. Their
//! nested macros cannot introduce enclosing textual bindings; exported root
//! path bindings are deliberately outside this witness's authority.
use super::*;

impl Lookup<'_> {
    pub(super) fn preceding(
        &mut self,
        module: &Module,
        item: &syn::Item,
        scope: &macro_scope::Scope,
    ) -> Result<bool, String> {
        let syn::Item::Macro(invocation) = item else {
            return Ok(false);
        };
        let Some(name) = invocation.mac.path.get_ident() else {
            return Ok(false);
        };
        if invocation.ident.is_some() || !invocation.attrs.is_empty() {
            return Ok(false);
        }

        let Ok(Some(binding)) = scope.lookup(&name.to_string()) else {
            // An earlier unknown expansion must never be healed by this proof.
            return Ok(false);
        };

        let definition = self
            .definitions
            .iter()
            .find(|(site, _)| site == &binding.site)
            .map(|(_, definition)| *definition)
            .ok_or("Missing exact preceding macro definition")?;
        let site = self.span(module, invocation.span())?;
        let input = Input {
            census: self.input.census,
            cfgs: self.input.cfgs,
            namespace: &module.namespace,
            definition: &binding.site,
            invocation: &site,
            preceding_expansions: false,
        };
        let expansion = expand(&input, definition, invocation, self.source_coordinates)?;
        self.charge_preceding(expansion.substitution_bound)?;
        inert(&expansion.items, &self.configuration)
    }

    fn charge_preceding(&mut self, bytes: usize) -> Result<(), String> {
        self.preceding_count += 1;
        self.preceding_bytes = self
            .preceding_bytes
            .checked_add(bytes)
            .filter(|total| *total <= super::super::MAX_BYTES)
            .ok_or("Preceding macro substitutions exceed the aggregate source budget")?;
        if self.preceding_count > 64 {
            return Err("Preceding macro substitutions exceed the bounded invocation count".into());
        }
        Ok(())
    }
}

fn inert(items: &[syn::Item], configuration: &Configuration<'_>) -> Result<bool, String> {
    let mut observed = macro_scope::Scope::default();
    for item in items {
        if !configuration.active(crate::conditional_source::item_attributes(item))? {
            continue;
        }
        // Exhaustive for this contract. No recursive expansion, generated macro
        // definition/import, module export, or unknown item is silently skipped.
        if !matches!(item, syn::Item::Impl(_) | syn::Item::Struct(_)) {
            return Ok(false);
        }
        observed.observe(item, configuration, None)?;
    }
    Ok(observed.lookup("__preceding_textual_scope").is_ok())
}
