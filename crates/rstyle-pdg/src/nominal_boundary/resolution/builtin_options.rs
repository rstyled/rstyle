//! Core Option spellings in the exact original module/block scope. This route
//! proves no nominal ownership, Copy/drop behavior or neighboring function body.

use super::selected::no_drop::scalar_result_owners::CheckedScalarResultSource;
use super::*;
use syn::visit::Visit;

/// Original Result path and its optional selected constructor tag.
pub(super) struct ResultSource<'a> {
    pub path: &'a syn::Path,
    pub variant: Option<&'a str>,
}

/// Field spellings resolve in the declaration module, without function locals.
pub(super) struct DeclarationScope<'a> {
    pub resolution: &'a Resolution,
    pub namespace: &'a Namespace,
    pub facts: &'a HirFnFacts,
}

impl PathResolution for DeclarationScope<'_> {
    fn resolve(&self, path: &syn::TypePath) -> Result<TypeTree, String> {
        Scoped {
            resolution: self.resolution,
            namespace: self.namespace,
        }
        .resolve(path)
    }

    fn array_length(&self, source: &syn::Expr, expected: u64) -> Result<(), String> {
        array_lengths::verify(
            &Scoped {
                resolution: self.resolution,
                namespace: self.namespace,
            },
            source,
            expected,
        )
    }

    fn scalar_option(&self, path: &syn::TypePath, payload: &TypeTree) -> Result<(), String> {
        if !supports(self.facts, super::fresh_products::CAPABILITY)? {
            return Err("Declaration Option fields require their fresh-product capability".into());
        }
        let TypeTree::Scalar(scalar) = payload else {
            return Err("Fresh product Option field requires a scalar payload".into());
        };
        self.option(path, scalar)
    }
}

impl DeclarationScope<'_> {
    fn option(&self, path: &syn::TypePath, scalar: &str) -> Result<(), String> {
        let environment = &self.resolution.environment;
        environment.check_context(self.facts)?;
        let configuration = crate::conditional_source::Configuration::new(&self.facts.build.cfgs);
        environment.check_ancestors(self.facts, self.namespace, &configuration)?;

        let modules: Vec<_> = environment
            .modules
            .iter()
            .filter(|module| &module.namespace == self.namespace)
            .collect();
        let [module] = modules.as_slice() else {
            return Err("Option field lacks one exact declaration module".into());
        };

        crate::data_library::verify_sysroot_roots(self.facts, &["core", "std"], "Option field")?;
        Scope {
            environment,
            module,
            blocks: Vec::new(),
            configuration: &configuration,
            facts: self.facts,
        }
        .verify(path, scalar)
    }
}

/// Reviewed Option semantics make Option<scalar> Copy and drop-free; this grants
/// neither arbitrary nominal Copy nor owned Option/product value permission.
pub(super) fn scalar_copy(facts: &HirFnFacts, payload: &TypeTree) -> Result<(), String> {
    if !supports(facts, super::fresh_products::CAPABILITY)?
        || !matches!(payload, TypeTree::Scalar(token) if concrete_types::scalar(token) && token != "unit")
    {
        return Err("Cleanup Option field requires its registered scalar payload".into());
    }
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Option field cleanup requires a pinned compiler/target")?;

    // Bind conditional Copy and absence of Drop to the complete reviewed core
    // source, independently of the registered constructor definitions.
    let baseline = crate::data_library::registry();
    let reviewed = baseline.result_propagation_source_digests.get("option.rs");
    if reviewed.is_none()
        || registry.result_propagation_source_digests.get("option.rs") != reviewed
        || !registry.definitions.contains_key("std::prelude::v1::None")
        || !registry.definitions.contains_key("std::prelude::v1::Some")
    {
        return Err("Option field cleanup lacks reviewed complete core source/variants".into());
    }
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Option field cleanup")
}

impl Resolution {
    /// The retained generated member supplies the exact module owner; the
    /// complete namespace certificate and ordinary Result lookup remain required.
    pub(super) fn verify_transport_result_type(
        &self,
        facts: &HirFnFacts,
        path: &syn::Path,
    ) -> Result<(), String> {
        let source = self
            .transport_source()
            .ok_or("Missing checked transport Result source owner")?;
        source.verify(facts)?;
        let member = source.member();

        let namespace = member.namespace();
        let environment = &self.environment;
        environment.check_context(facts)?;
        selected::verify_transport_builtin_imports(facts, self, namespace)?;
        crate::data_library::verify_sysroot_roots(
            facts,
            &["core", "std"],
            "Transport Result type",
        )?;
        let modules: Vec<_> = environment
            .modules
            .iter()
            .filter(|module| {
                module.namespace == namespace && module.path == member.expansion().invocation().path
            })
            .collect();
        let [module] = modules.as_slice() else {
            return Err("Transport Result type lacks one exact checked invocation module".into());
        };
        let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
        let scope = Scope {
            environment,
            module,
            blocks: Vec::new(),
            configuration: &configuration,
            facts,
        };
        scope.generated_result_names(path, member.expansion().items())?;
        scope.verify_result(path, None)
    }

    /// Namespace authority only; callers separately verify the source payload
    /// spelling against this exact independently Copy-certified scalar product.
    pub(crate) fn verify_product_option_type(
        &self,
        facts: &HirFnFacts,
        path: &syn::TypePath,
        payload: &TypeTree,
    ) -> Result<(), String> {
        if !supports(facts, "rust.concrete-option-propagation")?
            || !matches!(payload, TypeTree::Struct { .. } | TypeTree::Tuple(_))
            || !self.exact_product_sources(payload)
            || !crate::checked_option_products::checked_copy(
                &crate::data_types::TypeToken::from_tree(payload),
                self,
            )
        {
            return Err(
                "Product Option requires an independently checked local Copy scalar product".into(),
            );
        }

        self.environment
            .verify_builtin_scope(facts, |scope| scope.product_option(path))
    }
    // Token conversion intentionally omits declaration source text. Keep exact
    // captured-tree equality separately; a proved struct includes nested fields.
    fn exact_product_sources(&self, payload: &TypeTree) -> bool {
        match payload {
            TypeTree::Tuple(values) => values.iter().all(|value| self.exact_product_sources(value)),
            TypeTree::Struct { .. } => self.checked_owned_copy(payload),
            _ => true,
        }
    }
}

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

    /// An exact original source receipt replaces only coarse ancestor admission.
    pub(super) fn verify_checked_result_source(
        &self,
        facts: &HirFnFacts,
        source: ResultSource<'_>,
        receipt: &CheckedScalarResultSource,
    ) -> Result<(), String> {
        self.check_context(facts)?;
        crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Builtin Result value")?;
        let namespace = receipt.checked_namespace(facts, self)?;
        receipt.verify_builtin_imports(facts, self, namespace)?;
        let file = facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            .ok_or("Missing checked Result source file")?;
        let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
        let mut found = 0;

        for module in self.modules.iter().filter(|module| {
            module.path == std::path::Path::new(file) && module.namespace == namespace
        }) {
            let mut owners = Owners::new(self, module, facts, &configuration)?;
            owners.receipt = Some(receipt);
            for blocks in owners.scan()? {
                Scope {
                    environment: self,
                    module,
                    blocks,
                    configuration: &configuration,
                    facts,
                }
                .verify_result(source.path, source.variant)?;
                found += 1;
            }
        }
        if found != 1 {
            return Err("Checked Result source lacks one exact original callable owner".into());
        }
        Ok(())
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
            for blocks in owners(self, module, facts, &configuration)? {
                self.check_ancestors(facts, &module.namespace, &configuration)?;
                verify(&Scope {
                    environment: self,
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
        facts: &HirFnFacts,
        namespace: &[String],
        configuration: &crate::conditional_source::Configuration<'_>,
    ) -> Result<(), String> {
        for module in self
            .modules
            .iter()
            .filter(|module| namespace.starts_with(&module.namespace))
        {
            for item in &module.items {
                selected::module_item_for(facts, item, module, configuration)?;
            }
        }
        Ok(())
    }
}

fn owners<'a>(
    environment: &'a Environment,
    module: &'a census::Module,
    facts: &HirFnFacts,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<Vec<Vec<&'a syn::Block>>, String> {
    Owners::new(environment, module, facts, configuration)?.scan()
}

struct Owners<'a, 'b> {
    environment: &'a Environment,
    module: &'a census::Module,
    bytes: &'a str,
    facts: &'b HirFnFacts,
    configuration: &'b crate::conditional_source::Configuration<'b>,
    blocks: Vec<&'a syn::Block>,
    matches: Vec<Vec<&'a syn::Block>>,
    failure: Option<String>,
    receipt: Option<&'b CheckedScalarResultSource>,
}

impl<'a, 'b> Owners<'a, 'b> {
    fn new(
        environment: &'a Environment,
        module: &'a census::Module,
        facts: &'b HirFnFacts,
        configuration: &'b crate::conditional_source::Configuration<'b>,
    ) -> Result<Self, String> {
        Ok(Self {
            environment,
            module,
            bytes: environment.source(&module.path)?.bytes(),
            facts,
            configuration,
            blocks: Vec::new(),
            matches: Vec::new(),
            failure: None,
            receipt: None,
        })
    }

    fn scan(mut self) -> Result<Vec<Vec<&'a syn::Block>>, String> {
        for item in &self.module.items {
            self.visit_item(item);
        }
        if let Some(error) = self.failure {
            return Err(error);
        }
        Ok(self.matches)
    }
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
        if !self.configuration.active(&item.attrs)?
            || !(selected::implementation(self.facts, item)
                || self.checked_tryfrom(item)?
                || selected::local_traits::implementation(
                    self.facts,
                    self.environment,
                    self.module,
                    item,
                )?)
        {
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

    fn checked_tryfrom(&self, item: &syn::ItemImpl) -> Result<bool, String> {
        let Some(receipt) = self.receipt else {
            return Ok(false);
        };
        if !receipt.trait_owner() {
            return Ok(false);
        }
        Ok(crate::tryfrom_calls::source_member(
            self.facts,
            item,
            self.bytes,
            &crate::tryfrom_calls::RegistryAuthority,
        )?
        .is_some())
    }
}

struct GlobImport<'a> {
    original_key: &'a str,
    module_scope: bool,
}

struct Scope<'a, 'b> {
    environment: &'a Environment,
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
        if self.lookup(scalar)?.is_some() {
            return Err("Option scalar spelling is shadowed".into());
        }
        self.option_owner(path)
    }
    fn product_option(&self, path: &syn::TypePath) -> Result<(), String> {
        if path.qself.is_some()
            || path
                .path
                .segments
                .iter()
                .take(path.path.segments.len().saturating_sub(1))
                .any(|part| !matches!(part.arguments, syn::PathArguments::None))
        {
            return Err("Product Option has a qualified or generic library path".into());
        }
        self.option_owner(path)
    }
    fn option_owner(&self, path: &syn::TypePath) -> Result<(), String> {
        let names: Namespace = path
            .path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect();
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

    fn generated_result_names(&self, path: &syn::Path, items: &[syn::Item]) -> Result<(), String> {
        let first = path.segments.first().ok_or("Empty transport Result path")?;
        for name in [first.ident.to_string(), "core".into(), "std".into()] {
            if lookup(items.iter(), &name, self, false)?.is_some() {
                return Err("Transport expansion shadows a required Result type binding".into());
            }
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

    fn glob_excludes(
        &self,
        target: &[String],
        original: GlobImport<'_>,
        name: &str,
    ) -> Result<(), String> {
        if original.module_scope && supports(self.facts, crate::borrowed_result_types::CAPABILITY)?
        {
            let prefix: Vec<_> = original
                .original_key
                .strip_prefix("*::")
                .ok_or("Missing original module glob path")?
                .split("::")
                .map(str::to_owned)
                .collect();
            if selected::lexical_glob_excludes(
                self.facts,
                self.environment,
                selected::LexicalGlob {
                    namespace: &self.module.namespace,
                    path: &prefix,
                },
                name,
            )? {
                return Ok(());
            }
        }
        if !supports(self.facts, "rust.concrete-option-propagation")? {
            return Err("Option source has unresolved relevant glob imports".into());
        }
        let target = if target.first().is_some_and(|part| part == "<local>") {
            anchored(target[1..].to_vec(), &self.module.namespace, true)?
        } else {
            target.to_vec()
        };
        selected::glob_excludes(self.environment, &target, name, self.configuration)
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
            if let Some(binding) = lookup(items.into_iter(), name, self, false)? {
                return Ok(Some(binding));
            }
        }
        lookup(self.module.items.iter(), name, self, true)
    }
}

fn lookup<'a>(
    items: impl Iterator<Item = &'a syn::Item>,
    name: &str,
    scope: &Scope<'_, '_>,
    module_scope: bool,
) -> Result<Option<Namespace>, String> {
    let namespace = &scope.module.namespace;
    let mut imports = BTreeMap::new();
    let mut direct = None;
    for item in items {
        if type_name(item).as_deref() == Some(name) {
            direct = Some(external(item)?);
        }
        if let syn::Item::Use(item) = item {
            lookup_import(item, &mut imports, namespace)?;
        }
    }

    for ((_, key), target) in &imports {
        if key.starts_with("*::") {
            scope.glob_excludes(
                target,
                GlobImport {
                    original_key: key,
                    module_scope,
                },
                name,
            )?;
        }
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

fn lookup_import(
    item: &syn::ItemUse,
    imports: &mut BTreeMap<(Namespace, String), Namespace>,
    namespace: &Namespace,
) -> Result<(), String> {
    let prefix = if item.leading_colon.is_some() {
        vec!["<external>".into()]
    } else {
        Vec::new()
    };
    import_tree(
        &mut Imports {
            values: imports,
            namespace,
            selected: true,
        },
        &item.tree,
        prefix,
    )
}

#[cfg(test)]
mod transport_scope_tests {
    use super::*;

    #[test]
    fn generated_result_scope_preserves_original_and_emitted_type_shadow_checks() {
        let fixture = crate::generated_sources::checked::tests::fixture::Fixture::new();
        let environment = Environment::with_source_options(
            Arc::new(fixture.census.clone()),
            SourceOptions {
                constants: true,
                cfgs: Some(&fixture.facts.build.cfgs),
                expansions: true,
                module_lints: true,
            },
        )
        .unwrap();
        let configuration =
            crate::conditional_source::Configuration::new(&fixture.facts.build.cfgs);
        for (source, path, expected) in [
            ("", "Result", true),
            ("", "core::result::Result", true),
            ("use core::result::Result as Outcome;", "Outcome", true),
            ("struct Result;", "Result", false),
            ("mod core {}", "Result", false),
            ("type std = ();", "std::result::Result", false),
            ("use other::Result;", "Result", false),
        ] {
            let module = census::Module {
                path: fixture.census.files[0].path.clone(),
                namespace: Vec::new(),
                items: syn::parse_file(source).unwrap().items,
                verified_macro_imports: Default::default(),
                verified_external_imports: Default::default(),
            };
            let scope = Scope {
                environment: &environment,
                module: &module,
                blocks: Vec::new(),
                configuration: &configuration,
                facts: &fixture.facts,
            };
            let path = syn::parse_str(path).unwrap();
            assert_eq!(
                scope.verify_result(&path, None).is_ok(),
                expected,
                "{source}"
            );
            assert_emitted_result_shadows(&scope);
        }
    }

    fn assert_emitted_result_shadows(scope: &Scope<'_, '_>) {
        for emitted in [
            "struct Result;",
            "mod core {}",
            "type std = ();",
            "use other::Result;",
        ] {
            let items = syn::parse_file(emitted).unwrap().items;
            assert!(
                scope
                    .generated_result_names(&syn::parse_quote!(Result), &items)
                    .is_err(),
                "{emitted}"
            );
        }
    }
}
