//! Build original cfg-active bindings once; never infer an absent imported body.
use super::*;
use syn::spanned::Spanned;

impl SourceMacros {
    pub(super) fn check_globs(
        &mut self,
        environment: &Environment,
        configuration: &crate::conditional_source::Configuration<'_>,
    ) -> Result<(), String> {
        for module in &environment.modules {
            let mut unknown = false;
            for item in &module.items {
                if let syn::Item::Use(item) = item
                    && configuration.active(&item.attrs)?
                    && source_imports::has_glob(&item.tree)
                    && !environment.anonymous_use(&module.namespace, item)
                    && environment.generated_use(&module.namespace, item).is_none()
                {
                    unknown = true;
                }
            }
            self.modules
                .get_mut(&module.namespace)
                .ok_or("Missing original macro source module")?
                .glob = unknown;
        }

        Ok(())
    }

    pub(super) fn from_modules(
        modules: &[census::Module],
        configuration: &crate::conditional_source::Configuration<'_>,
    ) -> Result<Self, String> {
        let mut result = Self {
            modules: BTreeMap::new(),
            imports: BTreeMap::new(),
        };
        for module in modules {
            if result.modules.contains_key(&module.namespace) {
                return Err("Ambiguous original macro source module".into());
            }
            let mut bindings = Module::default();
            for item in &module.items {
                if configuration.active(crate::conditional_source::item_attributes(item))? {
                    observe(&mut bindings, &module.namespace, item, configuration)?;
                }
            }
            result.modules.insert(module.namespace.clone(), bindings);
        }
        for module in result.modules.values() {
            if module
                .children
                .values()
                .any(|(child, _)| !result.modules.contains_key(child))
            {
                return Err("Macro namespace child lacks its checked source module".into());
            }
        }

        Ok(result)
    }

    pub(super) fn check_originals(&self, environment: &Environment) -> Result<(), String> {
        for module in &environment.modules {
            let source = environment.source(&module.path)?;
            let bindings = &self.modules[&module.namespace];
            for definition in bindings.definitions.values().flatten() {
                let span = crate::source_evidence::original_span(source.bytes(), definition.span());
                let original: syn::ItemMacro =
                    syn::parse_str(source.range(span)?).map_err(|error| {
                        format!("Macro definition lacks complete original source: {error}")
                    })?;
                if original.ident != definition.ident
                    || !original.mac.path.is_ident("macro_rules")
                    || original.mac.tokens.to_string() != definition.mac.tokens.to_string()
                {
                    return Err("Macro definition differs from its exact source owner".into());
                }
            }
        }

        Ok(())
    }
}

fn observe(
    module: &mut Module,
    owner: &[String],
    item: &syn::Item,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    if let syn::Item::Macro(definition) = item
        && definition.mac.path.is_ident("macro_rules")
    {
        observe_definition(module, definition, configuration)
    } else {
        observe_binding(module, owner, item, configuration)
    }
}

fn observe_binding(
    module: &mut Module,
    owner: &[String],
    item: &syn::Item,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    match item {
        syn::Item::Use(item) => imports(module, owner, item, configuration)?,
        syn::Item::Mod(item) => {
            let name = item.ident.to_string();
            if module
                .children
                .insert(
                    name.clone(),
                    (joined(owner, &name), visibility(owner, &item.vis)?),
                )
                .is_some()
            {
                return Err("Duplicate active original macro module binding".into());
            }
        }
        _ => barrier(module, item),
    }

    Ok(())
}

fn observe_definition(
    module: &mut Module,
    item: &syn::ItemMacro,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    if let Err(error) = attributes(&item.attrs, configuration) {
        module
            .invalid_definitions
            .insert(item.span().byte_range().start, error);
    }
    let name = item
        .ident
        .as_ref()
        .ok_or("Missing original macro definition name")?;

    module
        .definitions
        .entry(name.to_string())
        .or_default()
        .push(item.clone());
    Ok(())
}

fn imports(
    module: &mut Module,
    owner: &[String],
    item: &syn::ItemUse,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    module.glob |= source_imports::has_glob(&item.tree);
    let visible = visibility(owner, &item.vis)?;
    for (name, target) in source_imports::bindings(&item.tree) {
        if name != "_" {
            module.imports.entry(name).or_default().push(Import {
                target,
                visible: visible.clone(),
                position: item.span().byte_range().start,
                absolute: item.leading_colon.is_some(),
                error: attributes(&item.attrs, configuration).err(),
            });
        }
    }

    Ok(())
}

fn barrier(module: &mut Module, item: &syn::Item) {
    let name = match item {
        syn::Item::Struct(item) => Some(&item.ident),
        syn::Item::Enum(item) => Some(&item.ident),
        syn::Item::Union(item) => Some(&item.ident),
        syn::Item::Trait(item) => Some(&item.ident),
        syn::Item::TraitAlias(item) => Some(&item.ident),
        syn::Item::Type(item) => Some(&item.ident),
        syn::Item::ExternCrate(item) => {
            Some(item.rename.as_ref().map_or(&item.ident, |(_, name)| name))
        }
        _ => None,
    };
    if let Some(name) = name {
        module.barriers.insert(name.to_string());
    }
}

fn visibility(owner: &[String], vis: &syn::Visibility) -> Result<Key, String> {
    match vis {
        syn::Visibility::Public(_) => Ok(Vec::new()),
        syn::Visibility::Inherited => Ok(owner.to_vec()),
        syn::Visibility::Restricted(restricted) => {
            let names = crate::source_evidence::plain_relative_path(&restricted.path)
                .ok_or("Unknown original macro visibility path")?;
            let (prefix, _) = super::paths::visibility_origin(owner, &names)?;
            if !owner.starts_with(&prefix) {
                return Err("Macro visibility is not an enclosing original module".into());
            }
            Ok(prefix)
        }
    }
}

pub(super) fn preceding(
    module: &Module,
    name: &str,
    position: usize,
) -> Result<Option<syn::ItemMacro>, String> {
    let Some(definitions) = module.definitions.get(name) else {
        return Ok(None);
    };
    let mut selected = None;
    for definition in definitions {
        if definition.span().byte_range().start < position {
            if let Some(error) = module
                .invalid_definitions
                .get(&definition.span().byte_range().start)
            {
                return Err(error.clone());
            }
            if definition
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("macro_export"))
            {
                return Err(
                    "Exported macro requires separately checked crate-root ownership".into(),
                );
            }
            selected = Some(definition.clone());
        }
    }

    Ok(selected)
}

fn attributes(
    attrs: &[syn::Attribute],
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    for attribute in configuration.effective_attributes(attrs)? {
        let path = attribute.path();
        if !["cfg", "doc", "allow", "warn", "deny", "forbid"]
            .iter()
            .any(|name| path.is_ident(name))
            && crate::source_evidence::plain_relative_path(path)
                != Some(vec!["rustfmt".into(), "skip".into()])
        {
            return Err("Imported macro has an unsupported active attribute expansion".into());
        }
    }

    Ok(())
}
