//! Bounded original local macro substitution; this establishes no callable permission.
use super::{Census, Module, expansions::FileSpan, macro_scope};
use crate::conditional_source::Configuration;
use std::collections::BTreeMap;
use syn::{parse::Parser, spanned::Spanned};
mod compiler;
mod globs;
pub(crate) use compiler::verify_compiler_in;
pub use compiler::{CompilerInput, verify_compiler};

#[derive(Clone, Copy)]
pub struct Input<'a> {
    pub census: &'a Census,
    pub cfgs: &'a [String],
    pub namespace: &'a [String],
    /// Full original macro_rules item, including its matcher and body.
    pub definition: &'a FileSpan,
    /// Full original invocation item, including its semicolon when present.
    pub invocation: &'a FileSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentBinding {
    pub parameter: String,
    pub value: String,
}

/// Source mechanics only; fields are private and cannot be deserialized as proof.
pub struct CheckedExpansion {
    definition: FileSpan,
    invocation: FileSpan,
    namespace: Vec<String>,
    bindings: Vec<IdentBinding>,
    items: Vec<syn::Item>,
    macro_path: Vec<String>,
    template_bom: bool,
}

impl CheckedExpansion {
    pub fn macro_path(&self) -> &[String] {
        &self.macro_path
    }

    /// Coordinates in this immutable template buffer; no semantic permission.
    pub fn source_span(&self, span: proc_macro2::Span) -> FileSpan {
        let range = span.byte_range();
        let prefix = if self.template_bom { 3 } else { 0 };
        FileSpan {
            path: self.definition.path.clone(),
            digest: self.definition.digest.clone(),
            span: crate::ByteSpan {
                start: prefix + range.start as u32,
                end: prefix + range.end as u32,
            },
        }
    }

    pub fn definition(&self) -> &FileSpan {
        &self.definition
    }
    pub fn invocation(&self) -> &FileSpan {
        &self.invocation
    }
    pub fn namespace(&self) -> &[String] {
        &self.namespace
    }
    pub fn bindings(&self) -> &[IdentBinding] {
        &self.bindings
    }
    pub fn items(&self) -> &[syn::Item] {
        &self.items
    }
}

pub fn verify(input: Input<'_>) -> Result<CheckedExpansion, String> {
    let modules = super::verify_with_module_lints(input.census, input.cfgs, true)?;
    verify_modules(input, &modules, false)
}

fn verify_modules(
    input: Input<'_>,
    modules: &[Module],
    source_coordinates: bool,
) -> Result<CheckedExpansion, String> {
    let mut lookup = Lookup {
        input,
        source_coordinates,
        configuration: Configuration::new(input.cfgs),
        modules: modules
            .iter()
            .map(|module| (module.namespace.clone(), module))
            .collect(),
        definitions: Vec::new(),
        checked: None,
    };
    lookup.module(&[], macro_scope::Scope::default())?;

    lookup
        .checked
        .ok_or_else(|| "Missing exact original generated-source invocation".into())
}

struct Lookup<'a> {
    input: Input<'a>,
    source_coordinates: bool,
    configuration: Configuration<'a>,
    modules: BTreeMap<Vec<String>, &'a Module>,
    definitions: Vec<(FileSpan, &'a syn::ItemMacro)>,
    checked: Option<CheckedExpansion>,
}

impl<'a> Lookup<'a> {
    fn module(
        &mut self,
        namespace: &[String],
        mut scope: macro_scope::Scope,
    ) -> Result<macro_scope::Scope, String> {
        let module = *self
            .modules
            .get(namespace)
            .ok_or("Missing exact macro source module")?;
        for item in &module.items {
            if !self
                .configuration
                .active(crate::conditional_source::item_attributes(item))?
            {
                continue;
            }
            let site = self.span(module, item.span())?;
            if site == *self.input.invocation {
                self.invocation(module, item, &scope)?;
                return Ok(scope);
            }
            self.observe(module, item, &mut scope)?;
            if let syn::Item::Mod(child) = item {
                self.child(module, child, &mut scope)?;
                if self.checked.is_some() {
                    return Ok(scope);
                }
            }
        }
        Ok(scope)
    }

    fn span(&self, module: &Module, span: proc_macro2::Span) -> Result<FileSpan, String> {
        let file = self
            .input
            .census
            .files
            .iter()
            .find(|file| file.path == module.path)
            .ok_or("Missing immutable generated-source buffer")?;
        Ok(FileSpan {
            path: file.path.clone(),
            digest: file.digest.clone(),
            span: crate::source_evidence::original_span(&file.bytes, span),
        })
    }

    fn observe(
        &mut self,
        module: &Module,
        item: &'a syn::Item,
        scope: &mut macro_scope::Scope,
    ) -> Result<(), String> {
        if let syn::Item::Use(import) = item
            && globs::closed(import, &self.modules, &self.configuration)?
        {
            return scope.observe(&globs::observed(import), &self.configuration, None);
        }
        if let syn::Item::ExternCrate(import) = item
            && module
                .verified_external_imports
                .contains(&import.ident.to_string())
        {
            let site = self.span(module, item.span())?;
            let record = self
                .input
                .census
                .external_imports
                .iter()
                .find(|record| record.declaration == site)
                .ok_or("Missing exact external macro import source")?;
            scope.import_external(
                record
                    .exports
                    .macros
                    .iter()
                    .map(|item| item.name.clone())
                    .collect(),
            );
            return Ok(());
        }
        let binding = if let syn::Item::Macro(definition) = item
            && definition.mac.path.is_ident("macro_rules")
        {
            Some(self.definition(module, definition)?)
        } else {
            None
        };

        scope.observe(item, &self.configuration, binding)
    }

    fn definition(
        &mut self,
        module: &Module,
        definition: &'a syn::ItemMacro,
    ) -> Result<macro_scope::DefinitionBinding, String> {
        let name = definition
            .ident
            .as_ref()
            .ok_or("Missing original local macro name")?;
        let site = self.span(module, definition.span())?;
        self.definitions.push((site.clone(), definition));
        Ok(macro_scope::DefinitionBinding {
            identity: module
                .namespace
                .iter()
                .cloned()
                .chain([name.to_string()])
                .collect::<Vec<_>>()
                .join("::"),
            site,
        })
    }

    fn child(
        &mut self,
        module: &Module,
        child: &syn::ItemMod,
        scope: &mut macro_scope::Scope,
    ) -> Result<(), String> {
        let namespace: Vec<_> = module
            .namespace
            .iter()
            .cloned()
            .chain([child.ident.to_string()])
            .collect();
        let imported = self.configuration.effective_attributes(&child.attrs)?.iter()
            .any(|attribute| matches!(attribute, syn::Meta::Path(path) if path.is_ident("macro_use")));
        if imported || self.input.namespace.starts_with(&namespace) {
            let child = self.module(&namespace, scope.child())?;
            if imported {
                scope.import_child(&child)?;
            }
        }
        Ok(())
    }

    fn invocation(
        &mut self,
        module: &Module,
        item: &syn::Item,
        scope: &macro_scope::Scope,
    ) -> Result<(), String> {
        if module.namespace != self.input.namespace || self.checked.is_some() {
            return Err("Generated-source invocation has another or ambiguous namespace".into());
        }
        let syn::Item::Macro(invocation) = item else {
            return Err("Generated source requires an original item macro invocation".into());
        };
        let name = invocation
            .mac
            .path
            .get_ident()
            .ok_or("Generated source requires a plain local macro name")?;
        if !invocation.attrs.is_empty() || invocation.ident.is_some() {
            return Err(
                "Generated-source invocation has unsupported attributes or item naming".into(),
            );
        }

        let binding = scope
            .lookup(&name.to_string())?
            .ok_or("Missing exact lexical macro binding")?;
        if binding.site != *self.input.definition {
            return Err(
                "Generated-source macro definition differs from actual textual binding".into(),
            );
        }
        let definition = self
            .definitions
            .iter()
            .find(|(site, _)| site == &binding.site)
            .map(|(_, definition)| *definition)
            .ok_or("Missing original bound macro definition")?;

        let mut expansion = expand(&self.input, definition, invocation, self.source_coordinates)?;
        expansion.macro_path = binding.identity.split("::").map(str::to_owned).collect();
        self.checked = Some(expansion);
        Ok(())
    }
}

fn expand(
    input: &Input<'_>,
    definition: &syn::ItemMacro,
    invocation: &syn::ItemMacro,
    source_coordinates: bool,
) -> Result<CheckedExpansion, String> {
    if !definition.attrs.is_empty() {
        return Err("Generated-source definition has unsupported attributes".into());
    }
    let bindings = ident_bindings(definition, &invocation.mac)?;
    // Bound substitution before cloning potentially repeated identifier tokens.
    let template_bytes = definition.mac.tokens.to_string().len();
    let argument_bytes = invocation.mac.tokens.to_string().len();
    if template_bytes
        .checked_mul(argument_bytes)
        .is_none_or(|bound| bound > super::MAX_BYTES)
    {
        return Err("Generated-source substitution exceeds the source budget".into());
    }
    let interpreter = if source_coordinates {
        crate::nominal_boundary::resolution::expand_local_ident_source
    } else {
        crate::nominal_boundary::resolution::expand_local_ident_macro
    };
    let tokens = interpreter(definition, &invocation.mac)?;
    let file: syn::File = syn::parse2(tokens).map_err(|error| error.to_string())?;
    if file.items.is_empty() || file.items.len() > 256 || !file.attrs.is_empty() {
        return Err("Generated-source expansion lacks a bounded complete item roster".into());
    }

    Ok(CheckedExpansion {
        definition: input.definition.clone(),
        invocation: input.invocation.clone(),
        namespace: input.namespace.to_vec(),
        bindings,
        items: file.items,
        macro_path: Vec::new(),
        template_bom: input
            .census
            .files
            .iter()
            .find(|file| file.path == input.definition.path)
            .is_some_and(|file| file.bytes.starts_with('\u{feff}')),
    })
}

fn ident_bindings(
    definition: &syn::ItemMacro,
    invocation: &syn::Macro,
) -> Result<Vec<IdentBinding>, String> {
    let first = definition
        .mac
        .tokens
        .clone()
        .into_iter()
        .next()
        .ok_or("Missing generated-source matcher")?;
    let proc_macro2::TokenTree::Group(pattern) = first else {
        return Err("Generated-source matcher is not a group".into());
    };
    let matcher: Matcher = syn::parse2(pattern.stream()).map_err(|error| error.to_string())?;
    let inputs = syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated
        .parse2(invocation.tokens.clone())
        .map_err(|error| error.to_string())?;
    if matcher.names.is_empty()
        || matcher.names.len() > 16
        || matcher.names.len() != inputs.len()
        || matcher.trailing_comma != inputs.trailing_punct()
    {
        return Err("Generated-source ident substitutions have an unsupported arity".into());
    }

    Ok(matcher
        .names
        .into_iter()
        .zip(inputs)
        .map(|(parameter, value)| IdentBinding {
            parameter: parameter.to_string(),
            value: value.to_string(),
        })
        .collect())
}

struct Matcher {
    names: Vec<syn::Ident>,
    trailing_comma: bool,
}
impl syn::parse::Parse for Matcher {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let mut names = Vec::new();
        let mut trailing_comma = false;
        while !input.is_empty() {
            input.parse::<syn::Token![$]>()?;
            let name: syn::Ident = input.parse()?;
            input.parse::<syn::Token![:]>()?;
            let fragment: syn::Ident = input.parse()?;
            if fragment != "ident" || names.contains(&name) {
                return Err(syn::Error::new(
                    fragment.span(),
                    "Expected distinct ident captures",
                ));
            }
            names.push(name);
            if !input.is_empty() {
                input.parse::<syn::Token![,]>()?;
                trailing_comma = input.is_empty();
            }
        }
        Ok(Self {
            names,
            trailing_comma,
        })
    }
}

#[cfg(test)]
#[path = "generated_sources/tests.rs"]
mod tests;
