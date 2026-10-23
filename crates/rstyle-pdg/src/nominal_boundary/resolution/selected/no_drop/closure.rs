//! Exhaustive active original/nested/generated-item visitor for cleanup absence.
use super::*;
use std::collections::BTreeMap;
use syn::visit::Visit;

pub(super) fn verify(
    environment: &Environment,
    facts: &HirFnFacts,
    target: &Namespace,
) -> Result<(), String> {
    verify_mode(environment, facts, target, SourceMode::Cleanup)
}

pub(super) fn verify_reference_product(
    environment: &Environment,
    facts: &HirFnFacts,
    target: &Namespace,
) -> Result<(), String> {
    verify_mode(environment, facts, target, SourceMode::Product)
}

/// Exact Copy-product callers separately prove every field and builtin Copy.
/// This shares the exhaustive product-source grammar, never proof caches.
pub(super) fn verify_copy_product(
    environment: &Environment,
    facts: &HirFnFacts,
    target: &Namespace,
) -> Result<(), String> {
    verify_mode(environment, facts, target, SourceMode::Product)
}

pub(super) fn verify_copy_owner(
    environment: &Environment,
    facts: &HirFnFacts,
    target: &Namespace,
) -> Result<(), String> {
    verify_mode(environment, facts, target, SourceMode::CopyOwner)
}

pub(super) fn verify_scalar_error(
    environment: &Environment,
    facts: &HirFnFacts,
    target: &Namespace,
) -> Result<(), String> {
    verify_mode(environment, facts, target, SourceMode::ScalarError)
}

#[derive(Clone, Copy)]
enum SourceMode {
    Cleanup,
    Product,
    CopyOwner,
    ScalarError,
}

fn verify_mode(
    environment: &Environment,
    facts: &HirFnFacts,
    target: &Namespace,
    mode: SourceMode,
) -> Result<(), String> {
    let reference_product = !matches!(mode, SourceMode::Cleanup);
    let cfgs = &facts.build.cfgs;

    let mut visitor = Closure {
        configuration: crate::conditional_source::Configuration::new(cfgs),
        edition: facts.build.edition.known_value(),
        library: crate::data_library::registry_for(facts),
        native_tls: cfgs.iter().any(|cfg| cfg == "target_thread_local"),
        test_marker: supports(facts, enum_errors::CAPABILITY)?,
        enums: enum_globs::Scope::new(
            &environment.modules,
            cfgs,
            reference_product || supports(facts, enum_errors::CAPABILITY)?,
        )?,
        names: headers::Names::new(target),
        definitions: BTreeMap::new(),
        scopes: Vec::new(),
        error: None,
        collecting: true,
        expansions: 0,
        nested_context: 0,
        generated_context: 0,
        reference_product,
        scalar_bindings: matches!(mode, SourceMode::CopyOwner | SourceMode::ScalarError),
        checked_external_imports: supports(facts, super::super::fresh_products::CAPABILITY)?,
    };
    for collecting in [true, false] {
        visitor.pass(environment, collecting)?;
    }
    Ok(())
}

fn inherited(
    modules: &[census::Module],
    namespace: &[String],
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<BTreeMap<String, syn::ItemMacro>, String> {
    let mut definitions = BTreeMap::new();
    for depth in 0..namespace.len() {
        let parent = modules
            .iter()
            .find(|module| module.namespace == namespace[..depth])
            .ok_or("Cleanup macro scope lacks its original ancestor")?;
        for item in &parent.items {
            if !configuration.active(crate::conditional_source::item_attributes(item))? {
                continue;
            }
            if matches!(item, syn::Item::Mod(module) if module.ident == namespace[depth]) {
                break;
            }
            if let syn::Item::Macro(item) = item
                && let Some(name) = &item.ident
            {
                definitions.insert(name.to_string(), item.clone());
            }
            if let syn::Item::Mod(module) = item
                && imports_macros(module, configuration)?
            {
                definitions.extend(imported_definitions(
                    modules,
                    parent,
                    module,
                    configuration,
                )?);
            }
        }
    }
    Ok(definitions)
}

fn imports_macros(
    module: &syn::ItemMod,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<bool, String> {
    let mut imported = false;
    for attribute in configuration.effective_attributes(&module.attrs)? {
        if attribute.path().is_ident("macro_use") {
            if !matches!(attribute, syn::Meta::Path(_)) {
                return Err(
                    "Cleanup module macro import requires exact whole-module grammar".into(),
                );
            }
            imported = true;
        }
    }
    Ok(imported)
}

fn imported_definitions(
    modules: &[census::Module],
    parent: &census::Module,
    module: &syn::ItemMod,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<BTreeMap<String, syn::ItemMacro>, String> {
    let namespace: Vec<_> = parent
        .namespace
        .iter()
        .cloned()
        .chain([module.ident.to_string()])
        .collect();
    let child = modules
        .iter()
        .find(|child| child.namespace == namespace)
        .ok_or("Missing verified macro import source")?;
    let mut definitions = BTreeMap::new();

    for item in &child.items {
        if !configuration.active(crate::conditional_source::item_attributes(item))? {
            continue;
        }
        let syn::Item::Macro(item) = item else {
            return Err("Cleanup macro import contains nondefinitions".into());
        };
        if !item.mac.path.is_ident("macro_rules") {
            return Err("Cleanup macro import is not an original macro_rules definition".into());
        }
        let name = item
            .ident
            .as_ref()
            .ok_or("Cleanup macro import lacks definition")?;
        definitions.insert(name.to_string(), item.clone());
    }
    Ok(definitions)
}

#[cfg(test)]
pub(super) fn inspect(
    items: &[&syn::Item],
    cfgs: &[String],
    target: &Namespace,
) -> Result<(), String> {
    inspect_with_library(items, cfgs, target, None)
}

#[cfg(test)]
pub(super) fn inspect_with_library(
    items: &[&syn::Item],
    cfgs: &[String],
    target: &Namespace,
    library: Option<&'static crate::data_library::Registry>,
) -> Result<(), String> {
    inspect_policy(
        items,
        target,
        Inspection {
            cfgs,
            library,
            closed_imports: false,
            test_marker: false,
            reference_product: false,
            scalar_bindings: false,
            edition: None,
        },
    )
}

#[cfg(test)]
pub(super) fn inspect_closed_imports(
    items: &[&syn::Item],
    target: &Namespace,
) -> Result<(), String> {
    inspect_policy(
        items,
        target,
        Inspection {
            cfgs: &[],
            library: crate::data_library::registries().first(),
            closed_imports: true,
            test_marker: true,
            reference_product: false,
            scalar_bindings: false,
            edition: None,
        },
    )
}

#[cfg(test)]
struct Inspection<'a> {
    cfgs: &'a [String],
    library: Option<&'static crate::data_library::Registry>,
    closed_imports: bool,
    test_marker: bool,
    reference_product: bool,
    scalar_bindings: bool,
    edition: Option<&'a str>,
}

#[cfg(test)]
pub(super) fn inspect_reference_product(
    items: &[&syn::Item],
    target: &Namespace,
    library: Option<&'static crate::data_library::Registry>,
) -> Result<(), String> {
    inspect_reference_product_edition(items, target, library, Some("2021"))
}

#[cfg(test)]
pub(super) fn inspect_reference_product_edition(
    items: &[&syn::Item],
    target: &Namespace,
    library: Option<&'static crate::data_library::Registry>,
    edition: Option<&str>,
) -> Result<(), String> {
    inspect_policy(
        items,
        target,
        Inspection {
            cfgs: &[],
            library,
            closed_imports: true,
            test_marker: true,
            reference_product: true,
            scalar_bindings: false,
            edition,
        },
    )
}

#[cfg(test)]
pub(super) fn inspect_scalar_error(items: &[&syn::Item], target: &Namespace) -> Result<(), String> {
    inspect_policy(
        items,
        target,
        Inspection {
            cfgs: &[],
            library: crate::data_library::registries().first(),
            closed_imports: true,
            test_marker: true,
            reference_product: true,
            scalar_bindings: true,
            edition: Some("2021"),
        },
    )
}

#[cfg(test)]
fn inspect_policy(
    items: &[&syn::Item],
    target: &Namespace,
    policy: Inspection<'_>,
) -> Result<(), String> {
    let Inspection {
        cfgs,
        library,
        closed_imports,
        test_marker,
        reference_product,
        scalar_bindings,
        edition,
    } = policy;
    let mut visitor = Closure {
        configuration: crate::conditional_source::Configuration::new(cfgs),
        edition,
        library,
        native_tls: cfgs.iter().any(|cfg| cfg == "target_thread_local"),
        test_marker,
        enums: enum_globs::Scope::inline(items, cfgs, closed_imports)?,
        names: headers::Names::new(target),
        definitions: BTreeMap::new(),
        scopes: Vec::new(),
        error: None,
        collecting: true,
        expansions: 0,
        nested_context: 0,
        generated_context: 0,
        reference_product,
        scalar_bindings,
        checked_external_imports: false,
    };
    visitor.scopes = vec![visitor.scope(items.iter().copied())?];
    for item in items {
        visitor.visit_item(item);
    }
    if let Some(error) = visitor.error.take() {
        return Err(error);
    }
    visitor.names.close();
    visitor.collecting = false;
    for item in items {
        visitor.visit_item(item);
    }
    visitor.error.map_or(Ok(()), Err)
}

fn shadows_primitive(item: &syn::Item) -> bool {
    super::super::super::builtin_options::type_name(item)
        .is_some_and(|name| concrete_types::scalar(&name) || name == "str")
}

struct Closure<'a> {
    configuration: crate::conditional_source::Configuration<'a>,
    edition: Option<&'a str>,
    library: Option<&'static crate::data_library::Registry>,
    native_tls: bool,
    test_marker: bool,
    enums: enum_globs::Scope,
    names: headers::Names,
    definitions: BTreeMap<String, syn::ItemMacro>,
    scopes: Vec<BTreeMap<String, bool>>,
    error: Option<String>,
    collecting: bool,
    expansions: usize,
    nested_context: usize,
    generated_context: usize,
    reference_product: bool,
    scalar_bindings: bool,
    checked_external_imports: bool,
}

impl Closure<'_> {
    fn pass(&mut self, environment: &Environment, collecting: bool) -> Result<(), String> {
        self.collecting = collecting;
        for module in &environment.modules {
            self.module(environment, module)?;
        }

        self.finish_pass()
    }
    fn finish_pass(&mut self) -> Result<(), String> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        self.names.close();
        Ok(())
    }
    fn module(&mut self, environment: &Environment, module: &census::Module) -> Result<(), String> {
        self.begin_module(environment, module)?;

        for item in &module.items {
            self.module_item(environment, module, item)?;
            if let Some(error) = self.error.take() {
                return Err(format!(
                    "{error} [module {}, source {}]",
                    module.namespace.join("::"),
                    module.path.display()
                ));
            }
        }
        Ok(())
    }
    fn begin_module(
        &mut self,
        environment: &Environment,
        module: &census::Module,
    ) -> Result<(), String> {
        self.enums.namespace = module.namespace.clone();
        self.definitions = inherited(&environment.modules, &module.namespace, &self.configuration)?;
        self.scopes = vec![self.scope(module.items.iter())?];
        Ok(())
    }
    fn module_item(
        &mut self,
        environment: &Environment,
        parent: &census::Module,
        item: &syn::Item,
    ) -> Result<(), String> {
        if self.reference_product
            && self.checked_external_imports
            && checked_external_import(environment, parent, item)?
        {
            // The immutable census checked this exact root declaration, its
            // source span and complete pinned macro exports. Calls and local
            // shadowing still pass through the exhaustive visitor.
            return Ok(());
        }
        if let syn::Item::Mod(child) = item {
            if ["alloc", "core", "std"].contains(&child.ident.to_string().as_str()) {
                return Err("Cleanup standard root is shadowed".into());
            }
            for attribute in &child.attrs {
                self.visit_attribute(attribute);
            }
            if self.configuration.active(&child.attrs)?
                && imports_macros(child, &self.configuration)?
            {
                self.definitions.extend(imported_definitions(
                    &environment.modules,
                    parent,
                    child,
                    &self.configuration,
                )?);
            }
        } else {
            self.visit_item(item);
        }
        Ok(())
    }
    fn scope<'a>(
        &self,
        items: impl IntoIterator<Item = &'a syn::Item>,
    ) -> Result<BTreeMap<String, bool>, String> {
        let mut scope = BTreeMap::new();
        for item in items {
            if !self
                .configuration
                .active(crate::conditional_source::item_attributes(item))?
            {
                continue;
            }
            self.scope_item(item, &mut scope);
        }
        Ok(scope)
    }
    fn scope_item(&self, item: &syn::Item, scope: &mut BTreeMap<String, bool>) {
        let binding = match item {
            syn::Item::Struct(item) => Some((&item.ident, true)),
            syn::Item::Enum(item) => Some((&item.ident, true)),
            syn::Item::Union(item) => Some((&item.ident, true)),
            syn::Item::Trait(item) => {
                scope.insert(format!("trait:{}", item.ident), true);
                Some((&item.ident, false))
            }
            syn::Item::Type(item) => Some((&item.ident, false)),
            syn::Item::Mod(item) => Some((&item.ident, false)),
            syn::Item::Use(item) => {
                self.scope_import(item, scope);
                None
            }
            _ => None,
        };

        if let Some((name, nominal)) = binding {
            source_imports::bind(scope, name.to_string(), nominal);
        }
    }
    fn scope_import(&self, item: &syn::ItemUse, scope: &mut BTreeMap<String, bool>) {
        self.enums.bindings(&item.tree, &[], scope);
        library_macros::bindings(&item.tree, &[], scope, self.library);
        if self.enums.closed_imports && item.leading_colon.is_none() {
            source_imports::record(&item.tree, scope);
        }
    }
    fn unknown_glob(&self) -> bool {
        self.scopes.iter().any(|scope| scope.contains_key("*"))
    }
    fn local_trait(&self, item: &syn::ItemImpl) -> bool {
        let Some((_, path, _)) = &item.trait_ else {
            return false;
        };
        if self.unknown_glob() {
            return false;
        }
        if self.enums.known_trait(path) {
            return true;
        }

        let Some(name) = path.get_ident().map(ToString::to_string) else {
            return false;
        };
        self.scopes
            .iter()
            .rev()
            .find(|scope| scope.contains_key(&name))
            .and_then(|scope| scope.get(&format!("trait:{name}")))
            .copied()
            .unwrap_or(false)
    }
    fn local_nominal(&self, item: &syn::ItemImpl) -> bool {
        let mut ty = item.self_ty.as_ref();
        loop {
            ty = match ty {
                syn::Type::Paren(inner) => &inner.elem,
                syn::Type::Group(inner) => &inner.elem,
                _ => break,
            };
        }

        let syn::Type::Path(ty) = ty else {
            return false;
        };
        let Some(name) = ty.path.get_ident() else {
            // Generic arguments do not change the nominal binding itself.
            if ty.qself.is_some() || ty.path.segments.len() != 1 {
                return false;
            }
            let name = ty.path.segments[0].ident.to_string();
            return self
                .scopes
                .iter()
                .rev()
                .find_map(|scope| scope.get(&name).or_else(|| scope.get("*")))
                .copied()
                .unwrap_or(false);
        };
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&name.to_string()).or_else(|| scope.get("*")))
            .copied()
            .unwrap_or(false)
    }
    fn thread_local(&mut self, mac: &syn::Macro) -> Result<(), String> {
        let imported = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get("macro:thread_local"))
            .copied()
            .unwrap_or(false);
        if !imported || !self.native_tls || !self.library.is_some_and(library_macros::pinned_native)
        {
            return Err("Cleanup thread_local lacks pinned native source/import authority".into());
        }
        for item in library_macros::scalar_statics(mac)? {
            let syn::Item::Static(value) = &item else {
                unreachable!();
            };
            let name =
                library_macros::scalar_name(&value.ty).ok_or("Missing scalar static type")?;
            if self.names.aliases.contains(&name)
                || self.scopes.iter().any(|scope| scope.contains_key(&name))
            {
                return Err("Cleanup thread_local scalar type is shadowed".into());
            }
            self.visit_item(&item);
        }
        Ok(())
    }
    fn macro_definition(&mut self, item: &syn::ItemMacro, name: &syn::Ident) {
        for attribute in &item.attrs {
            self.visit_attribute(attribute);
        }
        if self.error.is_some() {
            return;
        }

        let name = name.to_string();
        if super::super::builtin_derive(&name) || standard_macro(&name) || name == "test" {
            self.error = Some("Cleanup closure shadows builtin expansion authority".into());
            return;
        }
        self.definitions.insert(name, item.clone());
    }
    fn collect_item(&mut self, item: &syn::Item) -> Result<(), String> {
        if self.scalar_bindings && shadows_primitive(item) {
            return Err("Copy source declaration shadows an unresolved primitive binding".into());
        }
        self.collect_declaration_name(item)?;

        match item {
            syn::Item::Use(item) => self.collect_import(item)?,
            syn::Item::Type(item) => {
                self.names.aliases.insert(item.ident.to_string());
                if ["alloc", "core", "std"].contains(&item.ident.to_string().as_str()) {
                    return Err("Cleanup standard root alias is shadowed".into());
                }
            }
            syn::Item::ExternCrate(item) => {
                let attributes = self.configuration.effective_attributes(&item.attrs)?;
                if attributes
                    .iter()
                    .any(|attribute| attribute.path().is_ident("macro_use"))
                    || item.rename.is_some()
                {
                    return Err(
                        "Cleanup external macro/renamed root authority is unavailable".into(),
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn collect_declaration_name(&mut self, item: &syn::Item) -> Result<(), String> {
        let name = match item {
            syn::Item::Struct(item) => Some(&item.ident),
            syn::Item::Enum(item) => Some(&item.ident),
            syn::Item::Union(item) => Some(&item.ident),
            syn::Item::Trait(item) => Some(&item.ident),
            syn::Item::Mod(item) => Some(&item.ident),
            _ => None,
        };

        if name.is_some_and(|name| name == "From") {
            self.names.from_shadowed = true;
        }
        if self.reference_product
            && let Some(name) = name
            && prelude_traits::REFERENCE_NAMES.contains(&name.to_string().as_str())
        {
            self.names
                .reference_prelude_shadowed
                .insert(name.to_string());
        }
        if name.is_some_and(|name| ["alloc", "core", "std"].contains(&name.to_string().as_str())) {
            return Err("Cleanup standard root declaration is shadowed".into());
        }

        Ok(())
    }
    fn collect_import(&mut self, item: &syn::ItemUse) -> Result<(), String> {
        if self.scalar_bindings
            && self.generated_context > 0
            && source_imports::has_glob(&item.tree)
            && !self.enums.only_original_enum_globs(&item.tree)
        {
            return Err("Copy source expansion has unresolved glob bindings".into());
        }
        if self.scalar_bindings
            && source_imports::bindings(&item.tree)
                .iter()
                .any(|(name, _)| concrete_types::scalar(name) || name == "str")
        {
            return Err("Copy source import shadows an unresolved primitive binding".into());
        }
        if self.reference_product {
            self.collect_reference_import(item);
        }

        if source_imports::bindings(&item.tree)
            .iter()
            .any(|(name, _)| name == "From")
            || source_imports::has_glob(&item.tree)
                && if self.reference_product {
                    self.generated_context > 0 && !self.enums.only_original_enum_globs(&item.tree)
                } else {
                    self.nested_context > 0
                }
        {
            self.names.from_shadowed = true;
        }
        import_authority(&item.tree, Vec::new(), self.library, &self.enums)?;
        self.names.import(&item.tree)?;
        Ok(())
    }
    fn collect_reference_import(&mut self, item: &syn::ItemUse) {
        for (name, _) in source_imports::bindings(&item.tree) {
            if prelude_traits::REFERENCE_NAMES.contains(&name.as_str()) {
                self.names.reference_prelude_shadowed.insert(name);
            }
        }
        if self.generated_context > 0
            && source_imports::has_glob(&item.tree)
            && !self.enums.only_original_enum_globs(&item.tree)
        {
            self.names.reference_prelude_shadowed.extend(
                prelude_traits::REFERENCE_NAMES
                    .iter()
                    .map(|name| (*name).to_owned()),
            );
        }
    }
    fn check(&mut self, result: Result<(), String>) -> bool {
        if self.error.is_none() {
            self.error = result.err();
        }
        self.error.is_none()
    }
    fn expansion(&mut self, tokens: proc_macro2::TokenStream) -> Result<(), String> {
        self.begin_expansion()?;

        let result = self.expanded_tokens(tokens);
        self.generated_context -= 1;
        result
    }
    fn begin_expansion(&mut self) -> Result<(), String> {
        self.expansions += 1;
        if self.expansions > 16384 {
            return Err("Cleanup expansion budget exceeded".into());
        }
        self.generated_context += 1;
        Ok(())
    }
    fn expanded_tokens(&mut self, tokens: proc_macro2::TokenStream) -> Result<(), String> {
        if let Ok(file) = syn::parse2::<syn::File>(tokens.clone()) {
            self.expanded_items(&file)?;
        } else {
            let block: syn::Block =
                syn::parse_str(&format!("{{{tokens}}}")).map_err(|error| error.to_string())?;
            self.visit_block(&block);
        }
        Ok(())
    }
    fn enter_nested(&mut self, scope: BTreeMap<String, bool>) {
        self.scopes.push(scope);
        self.nested_context += 1;
    }
    fn leave_nested(&mut self) {
        self.nested_context -= 1;
        self.scopes.pop();
    }
    fn expanded_items(&mut self, file: &syn::File) -> Result<(), String> {
        self.enter_nested(self.scope(file.items.iter())?);
        for item in &file.items {
            self.visit_item(item);
        }
        self.leave_nested();
        Ok(())
    }
    fn active(&mut self, attributes: &[syn::Attribute]) -> bool {
        match self.configuration.active(attributes) {
            Ok(active) => active,
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }
    fn item_header(&mut self, item: &syn::Item) -> bool {
        if let syn::Item::Macro(item) = item
            && item.mac.path.is_ident("macro_rules")
            && let Some(name) = &item.ident
        {
            self.macro_definition(item, name);
            return false;
        }

        if matches!(item, syn::Item::TraitAlias(_) | syn::Item::Verbatim(_)) {
            self.error = Some("Cleanup source has opaque trait/item syntax".into());
            return false;
        }
        true
    }
    fn prelude_trait(&self, path: &syn::Path) -> Option<Namespace> {
        // Macro-generated imports/definitions can outlive their temporary visit
        // scope. Their collected names must still block absence-based lookup.
        if self.names.from_shadowed
            || self.names.drops.contains("From")
            || self.names.aliases.contains("From")
        {
            return None;
        }
        prelude_traits::resolve(path, &self.scopes, self.library)
    }
    fn active_item(&mut self, item: &syn::Item) {
        if self.item_header(item) && self.item_evidence(item) {
            syn::visit::visit_item(self, item);
        }
    }
    fn item_evidence(&mut self, item: &syn::Item) -> bool {
        let result = if self.collecting {
            self.collect_item(item)
        } else if let syn::Item::Impl(item) = item {
            self.names.verify(
                item,
                headers::Resolution {
                    local_nominal: self.local_nominal(item),
                    standard_roots: !self.unknown_glob(),
                    local_trait: self.local_trait(item),
                    reference_product_debug: self.reference_product_debug(item),
                    reference_product_prelude: self.reference_product_prelude(item),
                    resolved_trait: if self.enums.closed_imports {
                        item.trait_.as_ref().and_then(|(_, path, _)| {
                            source_imports::resolve(path, &self.scopes)
                                .or_else(|| self.prelude_trait(path))
                        })
                    } else {
                        None
                    },
                },
            )
        } else {
            Ok(())
        };

        self.check(result)
    }
    fn reference_product_debug(&self, item: &syn::ItemImpl) -> bool {
        if !self.reference_product || self.unknown_glob() {
            return false;
        }
        let Some((_, path, _)) = &item.trait_ else {
            return false;
        };
        let Some(path) = source_imports::resolve(path, &self.scopes) else {
            return false;
        };
        match path
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["core", "fmt", "Debug"] => {
                library_macros::import(&["core".into(), "fmt".into()], "Debug", self.library)
            }
            ["std", "fmt", "Debug"] => library_macros::std_debug(self.library),
            _ => false,
        }
    }
    fn reference_product_prelude(
        &self,
        item: &syn::ItemImpl,
    ) -> Option<prelude_traits::ReferenceTrait> {
        if !self.reference_product {
            return None;
        }
        let (_, path, _) = item.trait_.as_ref()?;
        let name = path.segments.first()?.ident.to_string();

        if self.names.reference_prelude_shadowed.contains(&name)
            || self.names.drops.contains(&name)
            || self.names.aliases.contains(&name)
        {
            return None;
        }
        prelude_traits::reference_product(path, &self.scopes, self.library, self.edition)
    }
    fn block_contents(&mut self, block: &syn::Block, scope: BTreeMap<String, bool>) {
        let saved = self.definitions.clone();
        self.enter_nested(scope);
        syn::visit::visit_block(self, block);
        self.finish_block(saved);
    }
    fn finish_block(&mut self, definitions: BTreeMap<String, syn::ItemMacro>) {
        self.leave_nested();
        self.definitions = definitions;
    }
    fn enter_module_namespace(&mut self, module: &syn::ItemMod) -> bool {
        let previous = self.enums.unindexed;
        self.enums.unindexed |= self.nested_context > 0;
        self.enums.namespace.push(module.ident.to_string());
        previous
    }
    fn leave_module_namespace(&mut self, previous: bool) {
        self.enums.namespace.pop();
        self.enums.unindexed = previous;
    }
    fn nested_module(&mut self, module: &syn::ItemMod) -> Result<(), String> {
        let Some((_, items)) = &module.content else {
            return Err("Cleanup nested external module lacks independent source ownership".into());
        };

        let previous = self.enter_module_namespace(module);
        let result = self.nested_module_contents(module, items);
        self.leave_module_namespace(previous);
        result
    }
    fn nested_module_contents(
        &mut self,
        module: &syn::ItemMod,
        items: &[syn::Item],
    ) -> Result<(), String> {
        self.scopes.push(self.scope(items.iter())?);
        let definitions = self.definitions.clone();

        syn::visit::visit_item_mod(self, module);
        self.finish_module_contents(definitions);
        Ok(())
    }
    fn finish_module_contents(&mut self, definitions: BTreeMap<String, syn::ItemMacro>) {
        self.definitions = definitions;
        self.scopes.pop();
    }
    fn builtin_macro(&mut self, mac: &syn::Macro) -> Result<(), String> {
        if self.unknown_glob() {
            return Err("Cleanup builtin macro has an unresolved glob import in scope".into());
        }
        if mac.path.is_ident("thread_local") {
            return self.thread_local(mac);
        }

        let normalized = library_macros::qualified_alloc(mac, self.library);
        for expression in macros::builtin(normalized.as_ref().unwrap_or(mac))? {
            self.visit_expr(&expression);
        }
        Ok(())
    }
    fn variant_attributes(&mut self, variant: &syn::Variant) {
        for attribute in &variant.attrs {
            if attribute.path().is_ident("default") {
                if !matches!(attribute.meta, syn::Meta::Path(_))
                    || !matches!(variant.fields, syn::Fields::Unit)
                {
                    self.error = Some("Cleanup default marker is not a unit enum variant".into());
                }
            } else {
                self.visit_attribute(attribute);
            }
        }
    }
    fn effective_attribute(&mut self, attribute: &syn::Meta) -> bool {
        let result = self.attribute_authority(attribute);
        self.check(result)
    }
    fn attribute_authority(&self, attribute: &syn::Meta) -> Result<(), String> {
        if attribute.path().is_ident("derive") {
            if self.unknown_glob() {
                return Err("Cleanup builtin derive has an unresolved glob import in scope".into());
            }
            return super::super::derive(attribute);
        }
        if attribute.path().is_ident("test") {
            // Inspect the whole original body even if the harness removes it.
            if !self.test_marker
                || !matches!(attribute, syn::Meta::Path(_))
                || self.unknown_glob()
                || self.definitions.contains_key("test")
            {
                return Err("Cleanup test marker lacks exact builtin authority".into());
            }
            return Ok(());
        }
        if !inert_attribute(attribute) {
            return Err("Cleanup closure has unknown attribute expansion".into());
        }
        Ok(())
    }
    fn variant_contents(&mut self, variant: &syn::Variant) {
        self.variant_attributes(variant);
        self.visit_fields(&variant.fields);
        if let Some((_, expression)) = &variant.discriminant {
            self.visit_expr(expression);
        }
    }
}

impl<'ast> Visit<'ast> for Closure<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if self.error.is_some() || !self.active(crate::conditional_source::item_attributes(item)) {
            return;
        }

        self.active_item(item);
    }
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let scope = self.scope(block.stmts.iter().filter_map(|stmt| match stmt {
            syn::Stmt::Item(item) => Some(item),
            _ => None,
        }));
        let Ok(scope) = scope else {
            self.error = scope.err();
            return;
        };
        self.block_contents(block, scope);
    }
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let result = self.nested_module(module);
        self.check(result);
    }
    fn visit_generic_param(&mut self, parameter: &'ast syn::GenericParam) {
        if self.collecting
            && let syn::GenericParam::Type(parameter) = parameter
        {
            self.names.aliases.insert(parameter.ident.to_string());
        }
        syn::visit::visit_generic_param(self, parameter);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.error.is_some() {
            return;
        }
        let definition = mac
            .path
            .get_ident()
            .and_then(|name| self.definitions.get(&name.to_string()))
            .cloned();

        let result = if let Some(definition) = definition {
            macros::expand(&definition, mac).and_then(|tokens| self.expansion(tokens))
        } else {
            self.builtin_macro(mac)
        };
        self.check(result);
    }
    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if matches!(expression, syn::Expr::Verbatim(_)) {
            self.error = Some("Cleanup source has opaque expression syntax".into());
            return;
        }
        if let syn::Expr::Block(block) = expression {
            match self.configuration.active(&block.attrs) {
                Ok(false) => return,
                Err(error) => {
                    self.error = Some(error);
                    return;
                }
                Ok(true) => {}
            }
        }
        syn::visit::visit_expr(self, expression);
    }
    fn visit_variant(&mut self, variant: &'ast syn::Variant) {
        if !self.active(&variant.attrs) {
            return;
        }
        self.variant_contents(variant);
    }
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        let effective = self
            .configuration
            .effective_attributes(std::slice::from_ref(attribute));
        let Ok(effective) = effective else {
            self.error = effective.err();
            return;
        };

        for attribute in effective {
            if !self.effective_attribute(&attribute) {
                return;
            }
        }
    }
}

fn checked_external_import(
    environment: &Environment,
    owner: &census::Module,
    item: &syn::Item,
) -> Result<bool, String> {
    let syn::Item::ExternCrate(import) = item else {
        return Ok(false);
    };
    if !owner
        .verified_external_imports
        .contains(&import.ident.to_string())
    {
        return Ok(false);
    }

    let source = environment.source(&owner.path)?;
    let span = ByteSpan {
        start: crate::source_evidence::original_span(source.bytes(), import.extern_token.span)
            .start,
        end: crate::source_evidence::original_span(source.bytes(), import.semi_token.span).end,
    };
    Ok(environment
        .census()
        .external_imports
        .iter()
        .any(|record| record.declaration.path == owner.path && record.declaration.span == span))
}

fn inert_attribute(attribute: &syn::Meta) -> bool {
    [
        "doc",
        "allow",
        "warn",
        "deny",
        "forbid",
        "cfg",
        "inline",
        "cold",
        "must_use",
        "repr",
        "non_exhaustive",
        "track_caller",
        "deprecated",
        "macro_use",
    ]
    .iter()
    .any(|name| attribute.path().is_ident(name))
}

fn standard_macro(name: &str) -> bool {
    [
        "cfg",
        "stringify",
        "vec",
        "matches",
        "assert",
        "assert_eq",
        "assert_ne",
        "debug_assert",
        "debug_assert_eq",
        "debug_assert_ne",
        "panic",
        "unreachable",
        "unimplemented",
        "todo",
        "write",
        "writeln",
        "format",
        "format_args",
        "concat",
        "thread_local",
    ]
    .contains(&name)
}

pub(super) fn protected(name: &str) -> bool {
    standard_macro(name)
        || super::super::builtin_derive(name)
        || ["alloc", "core", "std", "test"].contains(&name)
}

fn import_authority(
    tree: &syn::UseTree,
    mut prefix: Vec<String>,
    library: Option<&crate::data_library::Registry>,
    source: &enum_globs::Scope,
) -> Result<(), String> {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            import_authority(&path.tree, prefix, library, source)
        }
        syn::UseTree::Group(group) => {
            for child in &group.items {
                import_authority(child, prefix.clone(), library, source)?;
            }
            Ok(())
        }
        _ => import_leaf(tree, &prefix, library, source),
    }
}

fn import_leaf(
    tree: &syn::UseTree,
    prefix: &[String],
    library: Option<&crate::data_library::Registry>,
    source: &enum_globs::Scope,
) -> Result<(), String> {
    match tree {
        // A glob creates ambiguity only when a macro/derive/standard trait is
        // actually resolved in its lexical scope. Importing names alone creates
        // no destructor implementation. Scope maps retain a conservative barrier.
        syn::UseTree::Rename(rename)
            if source.macro_free_function(prefix, &rename.ident.to_string()) =>
        {
            Ok(())
        }
        syn::UseTree::Name(name) if source.macro_free_function(prefix, &name.ident.to_string()) => {
            Ok(())
        }
        syn::UseTree::Rename(rename) if protected(&rename.rename.to_string()) => {
            Err("Cleanup import shadows standard expansion authority".into())
        }
        syn::UseTree::Name(name) => named_import(name, prefix, library, source),
        _ => Ok(()),
    }
}

fn named_import(
    name: &syn::UseName,
    prefix: &[String],
    library: Option<&crate::data_library::Registry>,
    source: &enum_globs::Scope,
) -> Result<(), String> {
    if source.closed_imports
        && prefix == ["std", "fmt"]
        && name.ident == "Debug"
        && library_macros::std_debug(library)
        || library_macros::import(prefix, &name.ident.to_string(), library)
    {
        return Ok(());
    }
    if name.ident == "self" && prefix.last().is_some_and(|name| protected(name)) {
        return Err("Cleanup self import shadows standard authority".into());
    }
    if protected(&name.ident.to_string()) {
        return Err(format!(
            "Cleanup import shadows builtin expansion authority: {}::{}",
            prefix.join("::"),
            name.ident
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn inherited_expansion(text: &str, name: &str) -> Result<String, String> {
    let file = syn::parse_file(text).map_err(|error| error.to_string())?;
    let modules = vec![census::Module {
        path: "/sample/src/lib.rs".into(),
        namespace: vec![],
        items: file.items,
        verified_macro_imports: Default::default(),
        verified_external_imports: Default::default(),
    }];
    let configuration = crate::conditional_source::Configuration::new(&[]);
    let definitions = inherited(&modules, &["child".into()], &configuration)?;
    let definition = definitions
        .get(name)
        .ok_or("Missing inherited test macro")?;
    let invocation = syn::parse_str::<syn::ExprMacro>(&format!("{name}!()"))
        .map_err(|error| error.to_string())?;
    macros::expand(definition, &invocation.mac).map(|tokens| tokens.to_string())
}

#[cfg(test)]
pub(super) fn imported_expansion(text: &str, name: &str) -> Result<String, String> {
    let file = syn::parse_file(text).map_err(|error| error.to_string())?;
    let mut modules = Vec::new();
    for item in &file.items {
        if let syn::Item::Mod(module) = item
            && let Some((_, items)) = &module.content
        {
            modules.push(census::Module {
                path: "/sample/src/lib.rs".into(),
                namespace: vec![module.ident.to_string()],
                items: items.clone(),
                verified_macro_imports: Default::default(),
                verified_external_imports: Default::default(),
            });
        }
    }
    modules.push(census::Module {
        path: "/sample/src/lib.rs".into(),
        namespace: vec![],
        items: file.items,
        verified_macro_imports: Default::default(),
        verified_external_imports: Default::default(),
    });
    let configuration = crate::conditional_source::Configuration::new(&[]);
    let definitions = inherited(&modules, &["child".into()], &configuration)?;
    let definition = definitions.get(name).ok_or("Missing imported test macro")?;
    let invocation = syn::parse_str::<syn::ExprMacro>(&format!("{name}!()"))
        .map_err(|error| error.to_string())?;
    macros::expand(definition, &invocation.mac).map(|tokens| tokens.to_string())
}

#[cfg(test)]
mod checked_external_tests {
    use super::*;
    use crate::nominal_boundary::census::{expansions::FileSpan, external_imports};

    #[test]
    fn checked_import_requires_exact_original_declaration_not_only_a_crate_name() {
        let source = "#[macro_use] extern crate alloc;\n";
        let mut census =
            (*crate::nominal_boundary::resolution::environment::tests::census()).clone();
        census.files[0].bytes = source.into();
        census.files[0].digest = census.files[0].bytes.identity().into();
        census.external_imports.push(external_imports::Import {
            declaration: FileSpan {
                path: census.files[0].path.clone(),
                digest: census.files[0].digest.clone(),
                span: ByteSpan { start: 13, end: 32 },
            },
            exports: crate::data_library::registry()
                .alloc_macro_exports
                .clone()
                .unwrap(),
        });
        let environment =
            Environment::with_options(std::sync::Arc::new(census), true, Some(&[]), false).unwrap();
        let module = &environment.modules[0];
        assert!(checked_external_import(&environment, module, &module.items[0]).unwrap());
        for source in [
            "extern crate alloc;",
            "extern crate alloc as renamed;",
            "#[custom] extern crate alloc;",
        ] {
            let unproved: syn::Item = syn::parse_str(source).unwrap();
            assert!(
                !checked_external_import(&environment, module, &unproved).unwrap(),
                "{source}"
            );
        }
    }
}
