//! Lexical input ownership for a separately checked, closed scalar projection.
//! Neighbouring items supply no values or calls to that grammar. This proof
//! grants no trait-bound, macro-resolution, artifact, or caller authority.
use super::CheckedProjection;
use crate::{
    HirFnFacts,
    nominal_boundary::{UnitContext, census},
};
use std::{
    collections::BTreeMap,
    path::{Component, Path},
};
use syn::spanned::Spanned;

#[cfg(test)]
mod tests;

type Result<T> = std::result::Result<T, String>;

pub(super) fn verify(original: &HirFnFacts, checked: &CheckedProjection) -> Result<()> {
    if original.source != checked.facts().source || original.build != checked.facts().build {
        return Err("Scalar owner witness differs from original source/build".into());
    }

    lexical(original)
}

/// Original lexical ownership only. Callers separately prove every operation,
/// type, obligation and dependency; neighboring items grant no such authority.
pub(super) fn lexical(original: &HirFnFacts) -> Result<()> {
    let options = &original.build.frontend_options;
    let census = options.data::<std::sync::Arc<census::Census>>()?;
    let context: UnitContext = serde_json::from_value(
        options
            .get(crate::nominal_boundary::CONTEXT_KEY)
            .ok_or("Missing scalar owner selected-unit context")?
            .clone(),
    )
    .map_err(|error| error.to_string())?;
    context.validate()?;
    context_binding(original, &context)?;
    census::check_budget(&census)?;
    if census.identity != census::IDENTITY || census.context != context {
        return Err("Scalar owner census context disagrees".into());
    }

    let files = files(&census)?;
    let source = options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing scalar owner source file")?;
    let configuration = crate::conditional_source::Configuration::new(&original.build.cfgs);
    let names: Vec<_> = original.name.split("::").collect();
    let root = &context.root_source;
    let directory = root.parent().ok_or("Missing scalar owner root directory")?;
    let owner = Owner {
        files: &files,
        configuration,
        original,
        source: Path::new(source),
    };
    if names.len() > 64 {
        return Err("Scalar owner module nesting exceeds budget".into());
    }

    owner.file(&Site {
        path: root,
        directory,
        names: &names,
    })
}

fn context_binding(original: &HirFnFacts, context: &UnitContext) -> Result<()> {
    let compiled = original
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Missing scalar compiler owner identity")?;
    let root = context
        .package
        .manifest_path
        .parent()
        .ok_or("Missing scalar package root")?;
    if compiled.get("identity").and_then(serde_json::Value::as_str)
        != Some("rstyle.rust.compiled-crate/1")
        || compiled
            .get("crate_name")
            .and_then(serde_json::Value::as_str)
            != Some(context.crate_name.as_str())
        || !context.root_source.starts_with(root)
        || root
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err("Scalar lexical context differs from compiler owner or package root".into());
    }
    Ok(())
}

fn files(census: &census::Census) -> Result<BTreeMap<&Path, &str>> {
    let root = census
        .context
        .package
        .manifest_path
        .parent()
        .ok_or("Missing scalar package root")?;
    let mut files = BTreeMap::new();
    for file in &census.files {
        if !file.path.is_absolute()
            || !file.path.starts_with(root)
            || file
                .path
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
            || !file.bytes.matches_identity(&file.digest)
            || files
                .insert(file.path.as_path(), file.bytes.as_ref())
                .is_some()
        {
            return Err("Invalid or duplicate scalar owner source input".into());
        }
    }
    Ok(files)
}

struct Owner<'a> {
    files: &'a BTreeMap<&'a Path, &'a str>,
    configuration: crate::conditional_source::Configuration<'a>,
    original: &'a HirFnFacts,
    source: &'a Path,
}

struct Site<'a> {
    path: &'a Path,
    directory: &'a Path,
    names: &'a [&'a str],
}

impl Owner<'_> {
    fn file(&self, site: &Site<'_>) -> Result<()> {
        let bytes = self
            .files
            .get(site.path)
            .ok_or("Missing scalar owner module input")?;
        let syntax = syn::parse_file(bytes.strip_prefix('\u{feff}').unwrap_or(bytes))
            .map_err(|error| error.to_string())?;
        self.attributes(&syntax.attrs)?;
        self.items(site, bytes, &syntax.items)
    }

    fn items(&self, site: &Site<'_>, bytes: &str, items: &[syn::Item]) -> Result<()> {
        let (name, rest) = site
            .names
            .split_first()
            .ok_or("Missing scalar lexical owner name")?;
        let item = owner_item(items, name)?;
        match item {
            syn::Item::Fn(function) if rest.is_empty() => self.function(site.path, bytes, function),
            syn::Item::Mod(module) if !rest.is_empty() && module.unsafety.is_none() => {
                self.module(site, bytes, module)
            }
            _ => Err("Scalar owner is generated or has an unsupported lexical chain".into()),
        }
    }

    fn module(&self, site: &Site<'_>, bytes: &str, module: &syn::ItemMod) -> Result<()> {
        self.attributes(&module.attrs)?;
        let child = site.directory.join(module.ident.to_string());
        let child_site = Site {
            path: site.path,
            directory: &child,
            names: &site.names[1..],
        };

        if let Some((_, items)) = &module.content {
            return self.items(&child_site, bytes, items);
        }

        let ordinary = site.directory.join(format!("{}.rs", module.ident));
        let nested = child.join("mod.rs");
        let target = match (
            self.files.contains_key(ordinary.as_path()),
            self.files.contains_key(nested.as_path()),
        ) {
            (true, false) => ordinary,
            (false, true) => nested,
            _ => return Err("Missing or ambiguous scalar file module".into()),
        };

        self.file(&Site {
            path: &target,
            ..child_site
        })
    }

    fn function(&self, path: &Path, bytes: &str, function: &syn::ItemFn) -> Result<()> {
        self.attributes(&function.attrs)?;
        let start = function_start(bytes, function);
        let end = crate::source_evidence::original_span(bytes, function.block.span()).end;

        if path != self.source
            || start != self.original.fn_span.start
            || end != self.original.fn_span.end
            || bytes.get(start as usize..end as usize) != Some(self.original.source.bytes.as_ref())
        {
            return Err("Scalar function range differs from its original lexical owner".into());
        }
        Ok(())
    }

    fn attributes(&self, attributes: &[syn::Attribute]) -> Result<()> {
        if !self.configuration.active(attributes)? {
            return Err("Inactive scalar owner module".into());
        }
        for attribute in self.configuration.effective_attributes(attributes)? {
            match &attribute {
                syn::Meta::List(value) if value.path.is_ident("cfg") => {}
                syn::Meta::Path(path)
                    if path.is_ident("no_std") || path.is_ident("no_implicit_prelude") => {}
                syn::Meta::Path(path) if path.is_ident("inline") => {}
                syn::Meta::List(value)
                    if ["allow", "warn", "deny", "forbid"]
                        .iter()
                        .any(|name| value.path.is_ident(name)) =>
                {
                    value.parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated).map_err(|error| error.to_string())?;
                }
                syn::Meta::NameValue(value)
                    if value.path.is_ident("doc")
                        && matches!(
                            &value.value,
                            syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(_),
                                ..
                            })
                        ) => {}
                syn::Meta::List(value) if value.path.is_ident("doc") => {
                    let value: syn::MetaNameValue =
                        value.parse_args().map_err(|error| error.to_string())?;
                    if !value.path.is_ident("html_root_url")
                        || !matches!(
                            value.value,
                            syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(_),
                                ..
                            })
                        )
                    {
                        return Err("Scalar module documentation metadata is unsupported".into());
                    }
                }
                _ => {
                    return Err(
                        "Scalar module ownership requires independently checked attributes".into(),
                    );
                }
            }
        }
        Ok(())
    }
}

fn function_start(bytes: &str, function: &syn::ItemFn) -> u32 {
    let span = if matches!(function.vis, syn::Visibility::Inherited) {
        function.sig.span()
    } else {
        function.vis.span()
    };
    crate::source_evidence::original_span(bytes, span).start
}

fn owner_item<'a>(items: &'a [syn::Item], name: &str) -> Result<&'a syn::Item> {
    let mut candidates = items.iter().filter(|item| match item {
        syn::Item::Fn(item) => item.sig.ident == name,
        syn::Item::Mod(item) => item.ident == name,
        _ => false,
    });
    let item = candidates
        .next()
        .ok_or("Missing original scalar lexical owner")?;
    if candidates.next().is_some() {
        return Err("Ambiguous scalar lexical owner".into());
    }
    Ok(item)
}
