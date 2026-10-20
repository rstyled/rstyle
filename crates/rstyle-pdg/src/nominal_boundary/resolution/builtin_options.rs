//! Core Option spellings in the exact original module/block scope. This route
//! proves no nominal ownership, Copy/drop behavior or neighboring function body.

use super::*;
use syn::visit::Visit;

impl Environment {
    pub(crate) fn verify_option_type(
        &self,
        facts: &HirFnFacts,
        path: &syn::TypePath,
        scalar: &str,
    ) -> Result<(), String> {
        self.check_context(facts)?;
        library_roots(facts)?;
        let file = facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            .ok_or("Missing Option source file")?;
        let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);

        let mut found = 0;
        for module in self
            .modules
            .iter()
            .filter(|module| module.path.to_string_lossy() == file)
        {
            let bytes = self.source(&module.path)?.bytes();
            for blocks in owners(module, bytes, facts, &configuration)? {
                self.check_ancestors(&module.namespace, &configuration)?;
                Scope {
                    module,
                    blocks,
                    configuration: &configuration,
                }
                .verify(path, scalar)?;
                found += 1;
            }
        }

        if found != 1 {
            return Err("Option type lacks one exact original function owner".into());
        }
        Ok(())
    }

    fn check_ancestors(
        &self,
        namespace: &[String],
        configuration: &crate::conditional_source::Configuration<'_>,
    ) -> Result<(), String> {
        for module in self
            .modules
            .iter()
            .filter(|module| namespace.starts_with(&module.namespace))
        {
            for item in &module.items {
                selected::module_item_in(item, configuration, true)?;
            }
        }
        Ok(())
    }
}

fn library_roots(facts: &HirFnFacts) -> Result<(), String> {
    let invocation = facts
        .build
        .frontend_options
        .get("invocation")
        .and_then(serde_json::Value::as_array)
        .ok_or("Missing Option compiler invocation")?;
    let mut extern_value = false;

    for argument in invocation {
        let argument = argument
            .as_str()
            .ok_or("Invalid Option compiler argument")?;
        if argument.starts_with('@') {
            return Err("Option roots require expanded compiler arguments".into());
        }
        let value = if extern_value {
            Some(argument)
        } else {
            argument.strip_prefix("--extern=")
        };
        if let Some(value) = value {
            let name = value
                .split('=')
                .next()
                .unwrap_or(value)
                .rsplit(':')
                .next()
                .unwrap_or(value);
            if matches!(name, "core" | "std") {
                return Err("Option library root is overridden by an extern dependency".into());
            }
        }
        extern_value = argument == "--extern";
    }

    if extern_value {
        return Err("Incomplete Option extern argument".into());
    }
    Ok(())
}

fn owners<'a>(
    module: &'a census::Module,
    bytes: &'a str,
    facts: &HirFnFacts,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<Vec<Vec<&'a syn::Block>>, String> {
    let mut owners = Owners {
        bytes,
        facts,
        configuration,
        blocks: Vec::new(),
        matches: Vec::new(),
        failure: None,
    };
    for item in &module.items {
        owners.visit_item(item);
    }

    if let Some(error) = owners.failure {
        return Err(error);
    }
    Ok(owners.matches)
}

struct Owners<'a, 'b> {
    bytes: &'a str,
    facts: &'b HirFnFacts,
    configuration: &'b crate::conditional_source::Configuration<'b>,
    blocks: Vec<&'a syn::Block>,
    matches: Vec<Vec<&'a syn::Block>>,
    failure: Option<String>,
}

impl<'a> Visit<'a> for Owners<'a, '_> {
    fn visit_expr(&mut self, expression: &'a syn::Expr) {
        let span = source_span(self.bytes, expression.span());
        if span.start > self.facts.fn_span.start || span.end < self.facts.fn_span.end {
            return;
        }
        if let Err(error) = selected::function_attributes(
            self.facts,
            crate::conditional_source::expression_attributes(expression),
        ) {
            self.failure = Some(error);
            return;
        }
        syn::visit::visit_expr(self, expression);
    }

    fn visit_item(&mut self, item: &'a syn::Item) {
        if self.failure.is_some() {
            return;
        }
        if let Err(error) = self.inspect_item(item) {
            self.failure = Some(error);
        }
    }

    fn visit_block(&mut self, block: &'a syn::Block) {
        let span = source_span(self.bytes, block.span());
        if span.start > self.facts.fn_span.start || span.end < self.facts.fn_span.end {
            return;
        }

        self.blocks.push(block);
        syn::visit::visit_block(self, block);
        self.blocks.pop();
    }
}

impl<'a> Owners<'a, '_> {
    fn inspect_item(&mut self, item: &'a syn::Item) -> Result<(), String> {
        let syn::Item::Fn(function) = item else {
            return Ok(());
        };
        let span = source_span(self.bytes, function.span());
        if span.start > self.facts.fn_span.start || span.end < self.facts.fn_span.end {
            return Ok(());
        }
        if !self.configuration.active(&function.attrs)? {
            return Err("Option source function owner is inactive".into());
        }
        selected::module_item(item, self.configuration)?;

        if (FunctionSource {
            vis: &function.vis,
            sig: &function.sig,
            body: &function.block,
        })
        .exact(self.bytes, self.facts)
        {
            self.matches.push(self.blocks.clone());
            return Ok(());
        }
        if !function
            .sig
            .generics
            .params
            .iter()
            .all(|param| matches!(param, syn::GenericParam::Lifetime(_)))
        {
            return Err("Nested Option owner has unsupported enclosing generics".into());
        }
        self.visit_block(&function.block);
        Ok(())
    }
}

struct Scope<'a, 'b> {
    module: &'a census::Module,
    blocks: Vec<&'a syn::Block>,
    configuration: &'b crate::conditional_source::Configuration<'b>,
}

impl Scope<'_, '_> {
    fn verify(&self, path: &syn::TypePath, scalar: &str) -> Result<(), String> {
        if !concrete_types::scalar(scalar) || scalar == "unit" || path.qself.is_some() {
            return Err("Option path requires an exact bool/integer payload".into());
        }
        let names: Namespace = path
            .path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect();
        if self.lookup(scalar)?.is_some() {
            return Err("Option scalar spelling is shadowed".into());
        }
        let Some(first) = names.first() else {
            return Err("Empty Option type path".into());
        };

        let binding = self.lookup(first)?;
        let resolved = match binding {
            Some(mut binding) => {
                binding.extend_from_slice(&names[1..]);
                binding
            }
            None if names == ["Option"] => vec!["core".into(), "option".into(), "Option".into()],
            None => names,
        };
        if resolved != ["core", "option", "Option"] && resolved != ["std", "option", "Option"] {
            return Err("Option path lacks a direct core/std type binding".into());
        }
        if self
            .lookup(&resolved[0])?
            .is_some_and(|binding| binding != [resolved[0].clone()])
        {
            return Err("Option library root is shadowed".into());
        }
        Ok(())
    }

    fn lookup(&self, name: &str) -> Result<Option<Namespace>, String> {
        for block in self.blocks.iter().rev() {
            let mut items = Vec::new();
            for statement in &block.stmts {
                match statement {
                    syn::Stmt::Item(item)
                        if self
                            .configuration
                            .active(crate::conditional_source::item_attributes(item))? =>
                    {
                        selected::module_item(item, self.configuration)?;
                        items.push(item);
                    }
                    syn::Stmt::Macro(statement)
                        if self.configuration.active(&statement.attrs)? =>
                    {
                        return Err("Enclosing block has unresolved macro effects".into());
                    }
                    _ => {}
                }
            }
            if let Some(binding) = lookup(items.into_iter(), name, &self.module.namespace)? {
                return Ok(Some(binding));
            }
        }
        lookup(self.module.items.iter(), name, &self.module.namespace)
    }
}

fn lookup<'a>(
    items: impl Iterator<Item = &'a syn::Item>,
    name: &str,
    namespace: &Namespace,
) -> Result<Option<Namespace>, String> {
    let mut imports = BTreeMap::new();
    let mut direct = None;
    for item in items {
        if type_name(item).as_deref() == Some(name) {
            direct = Some(external(item)?);
        }
        if let syn::Item::Use(item) = item {
            let prefix = if item.leading_colon.is_some() {
                vec!["<external>".into()]
            } else {
                Vec::new()
            };
            import_tree(
                &mut Imports {
                    values: &mut imports,
                    namespace,
                    selected: true,
                },
                &item.tree,
                prefix,
            )?;
        }
    }

    if imports.keys().any(|(_, key)| key.starts_with("*::")) {
        return Err("Option source has unresolved relevant glob imports".into());
    }
    let imported = imports.get(&(namespace.clone(), name.into())).cloned();
    if direct.is_some() && imported.is_some() {
        return Err("Ambiguous Option type binding".into());
    }
    Ok(direct.or(imported).map(|mut path| {
        if path.first().is_some_and(|part| part == "<external>") {
            path.remove(0);
        }
        path
    }))
}

fn import_tree(
    imports: &mut Imports<'_>,
    tree: &syn::UseTree,
    prefix: Namespace,
) -> Result<(), String> {
    if let syn::UseTree::Group(group) = tree {
        for tree in &group.items {
            import_tree(imports, tree, prefix.clone())?;
        }
        return Ok(());
    }
    let mut prefix = prefix;
    if let syn::UseTree::Path(path) = tree
        && matches!(path.ident.to_string().as_str(), "crate" | "self" | "super")
    {
        prefix.push("<local>".into());
    }
    imports.tree(tree, prefix)
}

fn type_name(item: &syn::Item) -> Option<String> {
    match item {
        syn::Item::Struct(item) => Some(item.ident.to_string()),
        syn::Item::Enum(item) => Some(item.ident.to_string()),
        syn::Item::Union(item) => Some(item.ident.to_string()),
        syn::Item::Type(item) => Some(item.ident.to_string()),
        syn::Item::Trait(item) => Some(item.ident.to_string()),
        syn::Item::TraitAlias(item) => Some(item.ident.to_string()),
        syn::Item::Mod(item) => Some(item.ident.to_string()),
        syn::Item::ExternCrate(item) => Some(
            item.rename
                .as_ref()
                .map_or(&item.ident, |(_, name)| name)
                .to_string(),
        ),
        _ => None,
    }
}

fn external(item: &syn::Item) -> Result<Namespace, String> {
    if let syn::Item::ExternCrate(item) = item
        && (item.ident == "core" || item.ident == "std")
    {
        return Ok(vec![item.ident.to_string()]);
    }
    Err("Option path is shadowed by a source type/module declaration".into())
}
