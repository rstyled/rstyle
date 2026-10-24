//! Exact, source-owned cfg_if module expansion proof.
use super::{Closure, Site, cfg_if, expansions, macro_scope};
use syn::spanned::Spanned;

impl Closure<'_> {
    pub(super) fn observe(
        &self,
        site: &Site<'_>,
        item: &syn::Item,
        scope: &mut macro_scope::Scope,
    ) -> Result<(), String> {
        let configuration = self
            .selected
            .as_ref()
            .ok_or("Missing macro configuration")?;
        let binding = match item {
            syn::Item::Macro(item) if item.mac.path.is_ident("macro_rules") => {
                let name = item.ident.as_ref().ok_or("Missing original macro name")?;
                Some(macro_scope::DefinitionBinding {
                    identity: self.identity(site, &name.to_string()),
                    site: self.span(site.path, item.span())?,
                })
            }
            _ => None,
        };
        scope.observe(item, configuration, binding)
    }

    pub(super) fn macro_use(&self, module: &syn::ItemMod) -> Result<bool, String> {
        let configuration = self
            .selected
            .as_ref()
            .ok_or("Missing macro configuration")?;
        Ok(configuration.effective_attributes(&module.attrs)?.iter().any(|attribute| {
            matches!(attribute, syn::Meta::Path(path) if path.is_ident("macro_use"))
        }))
    }

    pub(super) fn expand(
        &mut self,
        site: &Site<'_>,
        item: &syn::Item,
        scope: &macro_scope::Scope,
    ) -> Result<Option<Vec<syn::Item>>, String> {
        if self.expansions.is_none() {
            return Ok(None);
        }

        let syn::Item::Macro(invocation) = item else {
            return Ok(None);
        };
        if !invocation.mac.path.is_ident("cfg_if") {
            return Ok(None);
        }

        let Ok(Some(binding)) = scope.lookup("cfg_if") else {
            return Ok(None);
        };
        let Some(definition) = self.definition(binding)? else {
            return Ok(None);
        };

        let configuration = self
            .selected
            .as_ref()
            .ok_or("Missing macro configuration")?;
        let Ok(selection) = cfg_if::select_with_index(invocation, configuration) else {
            return Ok(None);
        };

        let expansion = Expansion {
            site,
            invocation,
            binding,
            definition: &definition,
            selection: &selection,
        };

        let proof = expansion.proof(self)?;
        self.selected_modules(site, &selection, &proof)?;
        Ok(Some(selection.items))
    }

    fn definition(
        &self,
        binding: &macro_scope::DefinitionBinding,
    ) -> Result<Option<syn::ItemMacro>, String> {
        let source = self
            .files
            .get(&binding.site.path)
            .ok_or("Missing macro definition source")?;
        let file = syn::parse_file(&source.bytes).map_err(|error| error.to_string())?;
        Ok(file.items.into_iter().find_map(|item| match item {
            syn::Item::Macro(item)
                if self.span(&binding.site.path, item.span()).ok().as_ref()
                    == Some(&binding.site)
                    && cfg_if::verify_definition(&item).is_ok() =>
            {
                Some(item)
            }
            _ => None,
        }))
    }

    fn selected_modules(
        &mut self,
        site: &Site<'_>,
        selection: &cfg_if::Selection,
        proof: &Proof<'_>,
    ) -> Result<(), String> {
        let configuration = self
            .selected
            .as_ref()
            .ok_or("Missing macro configuration")?;
        let active = selection
            .items
            .iter()
            .map(|item| configuration.active(crate::conditional_source::item_attributes(item)))
            .collect::<Result<Vec<_>, _>>()?;
        for (item, active) in selection.items.iter().zip(active) {
            if active
                && let syn::Item::Mod(module) = item
                && module.content.is_none()
            {
                self.module_record(site, module, proof)?;
            }
        }
        Ok(())
    }

    fn identity(&self, site: &Site<'_>, name: &str) -> String {
        site.namespace
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(name))
            .collect::<Vec<_>>()
            .join("::")
    }

    fn span(
        &self,
        path: &std::path::Path,
        span: proc_macro2::Span,
    ) -> Result<expansions::FileSpan, String> {
        let record = self.files.get(path).ok_or("Missing original span source")?;
        let source = &record.bytes;
        let text = source.strip_prefix('\u{feff}').unwrap_or(source);

        let prefix = source.len() - text.len();
        Ok(expansions::FileSpan {
            path: path.into(),
            digest: record.digest.clone(),
            span: crate::ByteSpan {
                start: (crate::source_evidence::byte_offset(text, span.start()) + prefix)
                    .try_into()
                    .map_err(|_| "Macro source span overflow")?,
                end: (crate::source_evidence::byte_offset(text, span.end()) + prefix)
                    .try_into()
                    .map_err(|_| "Macro source span overflow")?,
            },
        })
    }

    fn module_record(
        &mut self,
        site: &Site<'_>,
        module: &syn::ItemMod,
        proof: &Proof<'_>,
    ) -> Result<(), String> {
        let parent = site.namespace.join("::");
        let name = module.ident.to_string();
        let identity = self.identity(site, &name);
        let records = self
            .expansions
            .ok_or("Missing module expansion inventory")?;

        let matching: Vec<_> = records
            .iter()
            .enumerate()
            .filter(|(_, record)| record.module == identity)
            .collect();
        let [(index, record)] = matching.as_slice() else {
            return Err("Missing or duplicate expanded module identity".into());
        };

        let paths = [
            site.directory.join(format!("{name}.rs")),
            site.directory.join(&name).join("mod.rs"),
        ];
        let loaded: Vec<_> = paths
            .iter()
            .filter(|path| self.files.contains_key(*path))
            .collect();

        if record.parent != parent
            || record.declaration != self.span(site.path, module.span())?
            || loaded.as_slice() != [&record.loaded_file]
            || !proof.matches(&record.chain)
            || !self.used_expansions.insert(*index)
        {
            return Err("Expanded module lacks exact original source/chain proof".into());
        }
        Ok(())
    }
}

struct Proof<'a> {
    binding: &'a macro_scope::DefinitionBinding,
    definition: expansions::FileSpan,
    invocations: Vec<expansions::FileSpan>,
}

impl Proof<'_> {
    fn matches(&self, chain: &[expansions::ExpansionStep]) -> bool {
        chain.len() == self.invocations.len()
            && chain
                .iter()
                .zip(&self.invocations)
                .all(|(step, invocation)| {
                    step.macro_definition == self.binding.identity
                        && step.definition == self.definition
                        && step.invocation == *invocation
                })
    }
}

struct Expansion<'a, 'b> {
    site: &'a Site<'b>,
    invocation: &'a syn::ItemMacro,
    binding: &'a macro_scope::DefinitionBinding,
    definition: &'a syn::ItemMacro,
    selection: &'a cfg_if::Selection,
}

impl Expansion<'_, '_> {
    fn proof<'a>(&'a self, closure: &Closure<'_>) -> Result<Proof<'a>, String> {
        let calls = internal_calls(self.definition.mac.tokens.clone());
        if calls.len() != 4 {
            return Err("Unproved cfg_if internal invocation sites".into());
        }

        let mut invocations = vec![closure.span(&self.binding.site.path, calls[2])?];
        for _ in 0..self.selection.index.unwrap_or(0) {
            invocations.push(closure.span(&self.binding.site.path, calls[3])?);
        }

        invocations.push(closure.span(
            &self.binding.site.path,
            calls[usize::from(!self.selection.has_else)],
        )?);

        invocations.push(closure.span(self.site.path, self.invocation.mac.span())?);

        let prefix = self
            .definition
            .mac
            .path
            .span()
            .join(
                self.definition
                    .ident
                    .as_ref()
                    .ok_or("Missing cfg_if name")?
                    .span(),
            )
            .ok_or("Unjoinable macro definition prefix")?;
        Ok(Proof {
            binding: self.binding,
            definition: closure.span(&self.binding.site.path, prefix)?,
            invocations,
        })
    }
}

fn internal_calls(tokens: proc_macro2::TokenStream) -> Vec<proc_macro2::Span> {
    let tokens: Vec<_> = tokens.into_iter().collect();
    let mut calls = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if let proc_macro2::TokenTree::Group(group) = token {
            calls.extend(internal_calls(group.stream()));
        }
        if matches!(token, proc_macro2::TokenTree::Ident(name) if name == "cfg_if")
            && matches!(tokens.get(index + 1), Some(proc_macro2::TokenTree::Punct(punct)) if punct.as_char() == '!')
            && let Some(proc_macro2::TokenTree::Group(group)) = tokens.get(index + 2)
            && let Some(span) = token.span().join(group.span())
        {
            calls.push(span);
        }
    }
    calls
}
