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
        self.verify_builtin_scope(facts, |scope| scope.verify(path, scalar))
    }

    pub(super) fn verify_result_source(
        &self,
        facts: &HirFnFacts,
        path: &syn::Path,
        variant: Option<&str>,
    ) -> Result<(), String> {
        self.verify_builtin_scope(facts, |scope| scope.verify_result(path, variant))
    }

    fn verify_builtin_scope(
        &self,
        facts: &HirFnFacts,
        mut verify: impl FnMut(&Scope<'_, '_>) -> Result<(), String>,
    ) -> Result<(), String> {
        self.check_context(facts)?;
        crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Builtin value")?;
        let file = facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            .ok_or("Missing builtin value source file")?;
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
                verify(&Scope {
                    module,
                    blocks,
                    configuration: &configuration,
                    facts,
                })?;
                found += 1;
            }
        }
        if found != 1 {
            return Err("Builtin value lacks one exact original function owner".into());
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
                selected::module_item_with_imports(item, module, configuration)?;
            }
        }
        Ok(())
    }
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
        if let syn::Item::Impl(implementation) = item {
            return self.inspect_implementation(implementation);
        }
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

    fn inspect_implementation(&mut self, item: &'a syn::ItemImpl) -> Result<(), String> {
        let span = source_span(self.bytes, item.span());
        if span.start > self.facts.fn_span.start || span.end < self.facts.fn_span.end {
            return Ok(());
        }
        if !self.configuration.active(&item.attrs)? || !selected::implementation(self.facts, item) {
            return Err("Option method requires an active supported inherent owner".into());
        }

        for member in &item.items {
            if let syn::ImplItem::Fn(method) = member
                && (FunctionSource {
                    vis: &method.vis,
                    sig: &method.sig,
                    body: &method.block,
                })
                .exact(self.bytes, self.facts)
            {
                if !self.configuration.active(&method.attrs)? {
                    return Err("Option source method owner is inactive".into());
                }
                selected::function_attributes(self.facts, &method.attrs)?;
                self.matches.push(self.blocks.clone());
            }
        }
        Ok(())
    }
}

struct Scope<'a, 'b> {
    module: &'a census::Module,
    blocks: Vec<&'a syn::Block>,
    configuration: &'b crate::conditional_source::Configuration<'b>,
    facts: &'b HirFnFacts,
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

    fn verify_result(&self, path: &syn::Path, variant: Option<&str>) -> Result<(), String> {
        let names: Namespace = path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect();
        let first = names.first().ok_or("Empty Result source path")?;

        let resolved = match self.lookup(first)? {
            Some(mut binding) => {
                binding.extend_from_slice(&names[1..]);
                binding
            }
            None if names.len() == 1 => match variant {
                None if first == "Result" => vec!["core".into(), "result".into(), "Result".into()],
                Some(tag) if first == tag => {
                    vec!["core".into(), "result".into(), "Result".into(), tag.into()]
                }
                _ => return Err("Result source has no builtin prelude binding".into()),
            },
            None if variant.is_some() && first == "Result" => {
                let mut resolved = vec!["core".into(), "result".into()];
                resolved.extend(names.clone());
                resolved
            }
            None => names.clone(),
        };
        let mut expected = vec![resolved[0].clone(), "result".into(), "Result".into()];
        if let Some(tag) = variant {
            expected.push(tag.into());
        }
        if !matches!(resolved[0].as_str(), "core" | "std")
            || resolved != expected
            || self
                .lookup(&resolved[0])?
                .is_some_and(|binding| binding != [resolved[0].clone()])
        {
            return Err("Result source lacks exact unshadowed core/std ownership".into());
        }
        if variant.is_some() {
            self.value_name(first)?;
        }
        Ok(())
    }

    fn value_name(&self, name: &str) -> Result<(), String> {
        for item in self
            .module
            .items
            .iter()
            .chain(self.blocks.iter().flat_map(|block| {
                block.stmts.iter().filter_map(|statement| match statement {
                    syn::Stmt::Item(item) => Some(item),
                    _ => None,
                })
            }))
        {
            let value = match item {
                syn::Item::Fn(item) => Some(&item.sig.ident),
                syn::Item::Const(item) => Some(&item.ident),
                syn::Item::Static(item) => Some(&item.ident),
                _ => None,
            };
            if value.is_some_and(|ident| ident == name) {
                return Err("Result constructor is shadowed by a source value".into());
            }
        }

        let mut bindings = ValueBindings {
            name,
            shadowed: false,
        };
        let function: syn::ItemFn = syn::parse_str(&self.facts.source.bytes)
            .map_err(|_| "Result constructor lacks its exact original signature")?;
        crate::source_evidence::closed_body(&function.block)?;
        bindings.visit_signature(&function.sig);
        bindings.visit_block(&function.block);
        for block in &self.blocks {
            bindings.visit_block(block);
        }
        if bindings.shadowed {
            return Err("Result constructor is shadowed by a local binding".into());
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

pub(super) fn type_name(item: &syn::Item) -> Option<String> {
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

struct ValueBindings<'a> {
    name: &'a str,
    shadowed: bool,
}
impl<'ast> Visit<'ast> for ValueBindings<'_> {
    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.shadowed |= pattern.ident == self.name;
        syn::visit::visit_pat_ident(self, pattern);
    }
}
