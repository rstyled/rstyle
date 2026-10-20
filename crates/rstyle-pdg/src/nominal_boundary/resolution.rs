//! Lexical nominal resolution from independently closed original source.

use super::{CONTEXT_KEY, UnitContext, census};
use crate::{
    ByteSpan, HirFnFacts,
    concrete_types::{
        self, InterfaceObservations, Observation, TypeTree, source_proof::PathResolution,
    },
    data_frontend::evidence,
    declaration_origins,
};
use std::{collections::BTreeMap, path::PathBuf, rc::Rc, sync::Arc};
pub(crate) mod environment;
mod selected;
pub(crate) use environment::Environment;
use syn::spanned::Spanned;

type Namespace = Vec<String>;

pub struct Resolution {
    pub namespace: Namespace,
    pub receiver: Option<TypeTree>,
    owner_path: Option<Namespace>,
    trees: BTreeMap<Namespace, TypeTree>,
    environment: Rc<Environment>,
}

impl Resolution {
    pub(crate) fn checked_owned_copy(&self, tree: &TypeTree) -> bool {
        let TypeTree::Struct { nominal, .. } = tree else {
            return false;
        };
        let key: Vec<_> = nominal
            .definition_path
            .split("::")
            .map(str::to_string)
            .collect();
        self.environment.copy_boundary && self.trees.get(&key) == Some(tree)
    }
}

/// Borrow a checked immutable lexical environment, or own the standalone result.
/// This view carries no helper proof or serialized verification capability.
pub struct ResolutionView<'a>(ResolutionStorage<'a>);

enum ResolutionStorage<'a> {
    Borrowed(&'a Resolution),
    Owned(Box<Resolution>),
}

impl<'a> ResolutionView<'a> {
    pub(crate) fn borrowed(value: &'a Resolution) -> Self {
        Self(ResolutionStorage::Borrowed(value))
    }
    pub(crate) fn owned(value: Resolution) -> Self {
        Self(ResolutionStorage::Owned(Box::new(value)))
    }
}

impl std::ops::Deref for ResolutionView<'_> {
    type Target = Resolution;
    fn deref(&self) -> &Resolution {
        match &self.0 {
            ResolutionStorage::Borrowed(value) => value,
            ResolutionStorage::Owned(value) => value,
        }
    }
}

pub(crate) fn from_facts(facts: &HirFnFacts) -> Result<Resolution, String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    verify(facts, &interface)
}

struct Declaration {
    path: PathBuf,
    span: ByteSpan,
    namespace: Namespace,
    module_index: usize,
    item_index: usize,
}

pub fn verify(facts: &HirFnFacts, interface: &InterfaceObservations) -> Result<Resolution, String> {
    let census = facts.build.frontend_options.data::<Arc<census::Census>>()?;
    let constants = constant_context(facts)?;
    let environment = Rc::new(Environment::with_options(
        Arc::clone(&census),
        constants,
        selected_cfgs(facts)?,
    )?);
    verify_in(facts, interface, environment)
}

pub(crate) fn constant_context(facts: &HirFnFacts) -> Result<bool, String> {
    supports(facts, "rust.nominal-constant-context")
}

pub(crate) fn verify_in(
    facts: &HirFnFacts,
    interface: &InterfaceObservations,
    environment: Rc<Environment>,
) -> Result<Resolution, String> {
    environment.check_context(facts)?;
    let origins = facts
        .build
        .frontend_options
        .data::<declaration_origins::Origins>()?;
    let context = declaration_origins::context(facts, interface, &origins)?;
    declaration_origins::verify_in(&context, &origins, &environment)?;
    let mut resolution = Resolution {
        namespace: Vec::new(),
        receiver: None,
        owner_path: None,
        trees: BTreeMap::new(),
        environment: Rc::clone(&environment),
    };

    for observation in context.parameters.iter().chain([&context.result]) {
        if let Observation::Captured { ty } = observation {
            collect(ty, &mut resolution.trees)?;
        }
    }
    verify_declarations(
        &resolution,
        &environment.index.declarations,
        &origins,
        environment.census(),
    )?;
    if environment.copy_boundary {
        selected::verify(facts, &origins, &resolution.trees, &environment)?;
    }
    function_scope(
        facts,
        &environment.modules,
        environment.census(),
        &mut resolution,
    )?;
    Ok(resolution)
}

fn file_bytes<'a>(census: &'a census::Census, path: &std::path::Path) -> Result<&'a str, String> {
    census
        .files
        .iter()
        .find(|file| file.path == path)
        .map(|file| file.bytes.as_ref())
        .ok_or_else(|| "Missing closed source file".into())
}

fn source_span(bytes: &str, span: proc_macro2::Span) -> ByteSpan {
    let source = bytes.strip_prefix('\u{feff}').unwrap_or(bytes);
    let prefix = bytes.len() - source.len();
    ByteSpan {
        start: (crate::source_evidence::byte_offset(source, span.start()) + prefix) as u32,
        end: (crate::source_evidence::byte_offset(source, span.end()) + prefix) as u32,
    }
}

struct Index {
    declarations: BTreeMap<Namespace, Declaration>,
    imports: BTreeMap<(Namespace, String), Namespace>,
    unresolved_expansions: std::collections::BTreeSet<Namespace>,
}
fn build_index(
    modules: &[census::Module],
    census: &census::Census,
    cfgs: Option<&[String]>,
) -> Result<Index, String> {
    let mut index = Index {
        declarations: BTreeMap::new(),
        imports: BTreeMap::new(),
        unresolved_expansions: Default::default(),
    };
    for (module_index, module) in modules.iter().enumerate() {
        index.items(&IndexSite {
            module,
            bytes: file_bytes(census, &module.path)?,
            module_index,
            selected: cfgs.is_some(),
        })?;
        if let Some(cfgs) = cfgs {
            let configuration = crate::conditional_source::Configuration::new(cfgs);
            if module
                .items
                .iter()
                .any(|item| selected::module_item(item, &configuration).is_err())
            {
                index.unresolved_expansions.insert(module.namespace.clone());
            }
        }
    }
    for (namespace, name) in index.imports.keys() {
        if cfgs.is_some()
            && (selected::builtin_derive(name)
                || concrete_types::scalar(name)
                || name.starts_with("*::"))
        {
            index.unresolved_expansions.insert(namespace.clone());
        }
        let key: Vec<_> = namespace.iter().cloned().chain([name.clone()]).collect();
        if index.declarations.contains_key(&key) {
            return Err("Import shadows a nominal declaration".into());
        }
    }
    Ok(index)
}
struct IndexSite<'a> {
    module: &'a census::Module,
    bytes: &'a str,
    module_index: usize,
    selected: bool,
}

impl Index {
    fn items(&mut self, site: &IndexSite<'_>) -> Result<(), String> {
        for (item_index, item) in site.module.items.iter().enumerate() {
            match item {
                syn::Item::Struct(item) => self.structure(site, item_index, item)?,
                syn::Item::Use(item) => {
                    let path = match (item.leading_colon.is_some(), site.selected) {
                        (false, _) => Vec::new(),
                        (true, true) => vec!["<external>".into()],
                        (true, false) => {
                            return Err("External import lacks captured nominal evidence".into());
                        }
                    };
                    Imports {
                        values: &mut self.imports,
                        namespace: &site.module.namespace,
                        selected: site.selected,
                    }
                    .tree(&item.tree, path)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn structure(
        &mut self,
        site: &IndexSite<'_>,
        item_index: usize,
        item: &syn::ItemStruct,
    ) -> Result<(), String> {
        let mut key = site.module.namespace.clone();
        key.push(item.ident.to_string());
        if !site.selected && concrete_types::lifetimes::parameter_count(&item.generics).is_err() {
            return Err("Generic nominal declaration".into());
        }

        let declaration = Declaration {
            path: site.module.path.clone(),
            span: if site.selected {
                struct_span(site.bytes, item)
            } else {
                source_span(site.bytes, item.span())
            },
            namespace: site.module.namespace.clone(),
            module_index: site.module_index,
            item_index,
        };
        if self.declarations.insert(key, declaration).is_some() {
            return Err("Ambiguous nominal declaration".into());
        }
        Ok(())
    }
}

struct Imports<'a> {
    values: &'a mut BTreeMap<(Namespace, String), Namespace>,
    namespace: &'a Namespace,
    selected: bool,
}

impl Imports<'_> {
    fn tree(&mut self, tree: &syn::UseTree, mut path: Namespace) -> Result<(), String> {
        match tree {
            syn::UseTree::Path(item) => {
                path.push(item.ident.to_string());
                self.tree(&item.tree, path)
            }
            syn::UseTree::Group(item) => self.group(item, path),
            leaf => self.endpoint(leaf, path),
        }
    }

    fn endpoint(&mut self, tree: &syn::UseTree, path: Namespace) -> Result<(), String> {
        match tree {
            syn::UseTree::Glob(_) if self.selected => self.glob(path),
            leaf => self.leaf(leaf, path),
        }
    }

    fn group(&mut self, item: &syn::UseGroup, path: Namespace) -> Result<(), String> {
        for child in &item.items {
            self.tree(child, path.clone())?;
        }
        Ok(())
    }

    fn glob(&mut self, path: Namespace) -> Result<(), String> {
        // Retain the unresolved glob; it never supplies a selected type witness.
        let key = (self.namespace.clone(), format!("*::{}", path.join("::")));
        self.values
            .entry(key)
            .or_insert(anchored(path, self.namespace, true)?);
        Ok(())
    }

    fn leaf(&mut self, leaf: &syn::UseTree, mut path: Namespace) -> Result<(), String> {
        let (name, alias) = match leaf {
            syn::UseTree::Name(item) => (item.ident.to_string(), item.ident.to_string()),
            syn::UseTree::Rename(item) => (item.ident.to_string(), item.rename.to_string()),
            _ => return Err("Glob imports require unavailable lexical resolution evidence".into()),
        };
        let alias = if name == "self" {
            if alias == "self" {
                path.last()
                    .cloned()
                    .ok_or("Self import lacks a parent path")?
            } else {
                alias
            }
        } else {
            path.push(name);
            alias
        };
        insert_import(alias, path, self.namespace, self.values)
    }
}

fn insert_import(
    name: String,
    path: Namespace,
    namespace: &Namespace,
    imports: &mut BTreeMap<(Namespace, String), Namespace>,
) -> Result<(), String> {
    let path = anchored(path, namespace, true)?;
    if imports.insert((namespace.clone(), name), path).is_some() {
        return Err(format!(
            "Ambiguous named import in {}",
            namespace.join("::")
        ));
    }
    Ok(())
}

fn anchored(path: Namespace, namespace: &Namespace, import: bool) -> Result<Namespace, String> {
    let (prefix, rest) = match path.first().map(String::as_str) {
        Some("crate") => (&[][..], &path[1..]),
        Some("self") => (namespace.as_slice(), &path[1..]),
        Some("super") => {
            let count = path
                .iter()
                .take_while(|part| part.as_str() == "super")
                .count();
            let length = namespace
                .len()
                .checked_sub(count)
                .ok_or("Nominal path escapes lexical crate root")?;
            (&namespace[..length], &path[count..])
        }
        _ => (
            if import {
                &[][..]
            } else {
                namespace.as_slice()
            },
            path.as_slice(),
        ),
    };
    Ok(prefix.iter().chain(rest).cloned().collect())
}

fn collect(tree: &TypeTree, trees: &mut BTreeMap<Namespace, TypeTree>) -> Result<(), String> {
    match tree {
        TypeTree::Struct { .. } => collect_nominal(tree, trees)?,
        TypeTree::Tuple(elements) => {
            for element in elements {
                collect(element, trees)?;
            }
        }
        TypeTree::Shared(element) | TypeTree::Slice(element) | TypeTree::Array { element, .. } => {
            collect(element, trees)?
        }
        _ => {}
    }
    Ok(())
}

fn collect_nominal(
    tree: &TypeTree,
    trees: &mut BTreeMap<Namespace, TypeTree>,
) -> Result<(), String> {
    let TypeTree::Struct {
        nominal, fields, ..
    } = tree
    else {
        return Err("Non-nominal type registry entry".into());
    };
    let key = nominal
        .definition_path
        .split("::")
        .map(str::to_string)
        .collect();
    if trees
        .insert(key, tree.clone())
        .is_some_and(|previous| previous != *tree)
    {
        return Err("Contradictory nominal trees in one interface".into());
    }

    for field in fields {
        collect(&field.ty, trees)?;
    }
    Ok(())
}

fn verify_declarations(
    resolution: &Resolution,
    declarations: &BTreeMap<Namespace, Declaration>,
    origins: &declaration_origins::Origins,
    census: &census::Census,
) -> Result<(), String> {
    for (key, tree) in &resolution.trees {
        let TypeTree::Struct {
            nominal, fields, ..
        } = tree
        else {
            return Err("Non-nominal registry entry".into());
        };
        let declaration = declarations
            .get(key)
            .ok_or("Nominal tree lacks lexical source declaration")?;
        let origin = origins
            .declarations
            .iter()
            .find(|origin| &origin.nominal == nominal)
            .ok_or("Missing nominal original-source origin")?;
        if origin.source_file != declaration.path.to_string_lossy()
            || origin.declaration_span != declaration.span
            || origin.file_bytes.as_ref() != file_bytes(census, &declaration.path)?
            || nominal.source_identity.as_deref() != Some(&census.context.package.package_id)
            || nominal.package_name != census.context.package.name
            || nominal.package_version != census.context.package.version
            || nominal.manifest_digest != census.context.package.manifest_digest
        {
            return Err(
                "Nominal declaration source, namespace or selected package disagrees".into(),
            );
        }
        let scoped = Scoped {
            resolution,
            namespace: &declaration.namespace,
        };
        for field in fields {
            concrete_types::source_proof::verify_top_spelling(
                &field.declaration,
                &field.ty,
                &scoped,
            )?;
        }
    }
    Ok(())
}

fn function_scope(
    facts: &HirFnFacts,
    modules: &[census::Module],
    census: &census::Census,
    resolution: &mut Resolution,
) -> Result<(), String> {
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing function source identity")?;
    let mut candidates = Vec::new();
    for module in modules
        .iter()
        .filter(|module| module.path.to_string_lossy() == file)
    {
        let bytes = file_bytes(census, &module.path)?;
        candidates.extend(module_functions(
            module,
            bytes,
            facts,
            resolution.environment.copy_boundary,
        ));
    }

    let [(namespace, receiver)] = candidates.as_slice() else {
        return Err("Missing or ambiguous exact lexical function ownership".into());
    };
    resolution.namespace = namespace.clone();
    if resolution.environment.copy_boundary {
        selected::check_namespace(&resolution.environment, namespace)?;
    }
    if let Some(receiver) = receiver {
        resolve_receiver(resolution, receiver, facts)?;
    }
    Ok(())
}

fn resolve_receiver(
    resolution: &mut Resolution,
    receiver: &syn::Type,
    facts: &HirFnFacts,
) -> Result<(), String> {
    let syn::Type::Path(path) = receiver else {
        return Err("Unsupported inherent receiver type".into());
    };
    resolution.owner_path = Some(
        Scoped {
            resolution,
            namespace: &resolution.namespace,
        }
        .name(path)?,
    );
    match resolution.resolve(path) {
        Ok(owner) => resolution.receiver = Some(owner),
        Err(error)
            if facts.signature.has_self
                || !supports(facts, "rust.static-inherent-helper-calls")? =>
        {
            return Err(error);
        }
        Err(_) => {}
    }
    Ok(())
}

/// A checked inherent owner borrows its tree through the lexical environment.
/// The standalone environment remains owned by this view until it is dropped.
pub struct OwnerView<'a>(ResolutionView<'a>);

impl std::ops::Deref for OwnerView<'_> {
    type Target = TypeTree;
    fn deref(&self) -> &TypeTree {
        self.0.receiver.as_ref().expect("checked inherent owner")
    }
}

/// Independently locate the exact nongeneric inherent implementation containing
/// this owned function, then resolve its concrete owner in the closed unit.
pub fn inherent_owner<'a>(
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    implementation: &ByteSpan,
) -> Result<OwnerView<'a>, String> {
    let resolution = facts.nominal_resolution()?;
    check_implementation(&resolution, facts, implementation)?;
    if resolution.receiver.is_none() {
        return Err("Missing independently resolved inherent owner".into());
    }
    Ok(OwnerView(resolution))
}

fn check_implementation(
    resolution: &Resolution,
    facts: &HirFnFacts,
    implementation: &ByteSpan,
) -> Result<(), String> {
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing inherent source file")?;

    let mut owners = 0;
    for module in resolution
        .environment
        .modules
        .iter()
        .filter(|module| module.path.to_string_lossy() == file)
    {
        let bytes = resolution.environment.source(&module.path)?.bytes();
        for item in &module.items {
            if let syn::Item::Impl(item) = item
                && source_span(bytes, item.span()) == *implementation
                && supported_inherent_member(item, bytes, facts)
            {
                owners += 1;
            }
        }
    }

    if owners != 1 {
        return Err("Missing or ambiguous exact nongeneric inherent implementation".into());
    }

    Ok(())
}

fn supports(facts: &HirFnFacts, capability: &str) -> Result<bool, String> {
    Ok(crate::extraction_profile::ExtractionProfile::from_facts(facts)?.supports(capability))
}

pub fn static_owner(
    facts: &dyn crate::unit_store::EvidenceFacts,
    implementation: &ByteSpan,
    nominal: &crate::concrete_types::Nominal,
) -> Result<(), String> {
    if facts.signature.has_self || !supports(facts, "rust.static-inherent-helper-calls")? {
        return Err(
            "Static inherent ownership requires its registered receiver-free boundary".into(),
        );
    }
    let resolution = facts.nominal_resolution()?;
    check_implementation(&resolution, facts, implementation)?;

    let owner = resolution
        .owner_path
        .as_ref()
        .ok_or("Missing exact static implementation owner")?;
    resolution.check_nominal(owner, nominal)
}

fn supported_inherent_member(item: &syn::ItemImpl, bytes: &str, facts: &HirFnFacts) -> bool {
    if item.trait_.is_some()
        || item.unsafety.is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
    {
        return false;
    }
    item.items.iter().any(|member| {
        let syn::ImplItem::Fn(method) = member else {
            return false;
        };
        (FunctionSource {
            vis: &method.vis,
            sig: &method.sig,
            body: &method.block,
        })
        .exact(bytes, facts)
            && crate::concrete_types::lifetimes::parameter_count(&method.sig.generics).is_ok()
            && supported_receiver(&method.sig, facts)
    })
}

fn supported_receiver(signature: &syn::Signature, facts: &HirFnFacts) -> bool {
    match signature.inputs.first() {
        Some(syn::FnArg::Receiver(receiver)) => {
            receiver.reference.is_some()
                && receiver.mutability.is_none()
                && receiver.colon_token.is_none()
        }
        _ => {
            !facts.signature.has_self
                && supports(facts, "rust.static-inherent-helper-calls").unwrap_or(false)
        }
    }
}

fn module_functions(
    module: &census::Module,
    bytes: &str,
    facts: &HirFnFacts,
    selected: bool,
) -> Vec<(Namespace, Option<Box<syn::Type>>)> {
    let mut candidates = Vec::new();
    for item in &module.items {
        match item {
            syn::Item::Fn(item)
                if (FunctionSource {
                    vis: &item.vis,
                    sig: &item.sig,
                    body: &item.block,
                })
                .exact(bytes, facts) =>
            {
                if !selected || selected::function_attributes(facts, &item.attrs).is_ok() {
                    candidates.push((module.namespace.clone(), None))
                }
            }
            syn::Item::Impl(item) => {
                if selected && !selected::implementation(facts, item) {
                    continue;
                }
                for member in &item.items {
                    if let syn::ImplItem::Fn(method) = member
                        && (FunctionSource {
                            vis: &method.vis,
                            sig: &method.sig,
                            body: &method.block,
                        })
                        .exact(bytes, facts)
                        && (!selected
                            || selected::function_attributes(facts, &method.attrs).is_ok())
                    {
                        candidates.push((module.namespace.clone(), Some(item.self_ty.clone())));
                    }
                }
            }
            _ => {}
        }
    }
    candidates
}

struct FunctionSource<'a> {
    vis: &'a syn::Visibility,
    sig: &'a syn::Signature,
    body: &'a syn::Block,
}
impl FunctionSource<'_> {
    fn exact(&self, bytes: &str, facts: &HirFnFacts) -> bool {
        let start = if matches!(self.vis, syn::Visibility::Inherited) {
            self.sig.span()
        } else {
            self.vis.span()
        };
        let range = ByteSpan {
            start: source_span(bytes, start).start,
            end: source_span(bytes, self.body.span()).end,
        };
        range == facts.fn_span
            && bytes.get(range.start as usize..range.end as usize)
                == Some(facts.source.bytes.as_str())
    }
}

struct Scoped<'a> {
    resolution: &'a Resolution,
    namespace: &'a Namespace,
}

impl PathResolution for Resolution {
    fn resolve(&self, path: &syn::TypePath) -> Result<TypeTree, String> {
        Scoped {
            resolution: self,
            namespace: &self.namespace,
        }
        .resolve(path)
    }
}

impl PathResolution for Scoped<'_> {
    fn resolve(&self, path: &syn::TypePath) -> Result<TypeTree, String> {
        if path.qself.is_some() || path.path.leading_colon.is_some() {
            return Err("Unsupported nominal type path".into());
        }
        concrete_types::lifetimes::arguments(path)?;

        if path.path.is_ident("Self") {
            return self
                .resolution
                .receiver
                .clone()
                .ok_or_else(|| "Self lacks exact owning impl proof".into());
        }
        let names = self.name(path)?;
        let tree = self
            .resolution
            .trees
            .get(&names)
            .cloned()
            .ok_or_else(|| "Named type lacks exact captured lexical declaration".to_string())?;
        concrete_types::lifetimes::verify_path(path, &tree)?;
        Ok(tree)
    }
}

impl Scoped<'_> {
    fn name(&self, path: &syn::TypePath) -> Result<Namespace, String> {
        if path.qself.is_some() || path.path.leading_colon.is_some() {
            return Err("Unsupported nominal type path".into());
        }
        concrete_types::lifetimes::arguments(path)?;

        let mut names: Namespace = path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        if names == ["Self"] {
            if concrete_types::lifetimes::arguments(path)?.is_some() {
                return Err("Self cannot carry explicit nominal lifetime arguments".into());
            }
            return self
                .resolution
                .owner_path
                .clone()
                .ok_or_else(|| "Self lacks exact owning impl proof".into());
        }
        if let Some(first) = names.first()
            && let Some(import) = self
                .resolution
                .environment
                .index
                .imports
                .get(&(self.namespace.clone(), first.clone()))
        {
            names.remove(0);
            let mut imported = import.clone();
            imported.extend(names);
            names = imported;
        } else {
            names = anchored(names, self.namespace, false)?;
        }
        if !self
            .resolution
            .environment
            .index
            .declarations
            .contains_key(&names)
        {
            return Err("Named type lacks exact captured lexical declaration".into());
        }
        Ok(names)
    }
}

impl Resolution {
    pub fn matches_nominal(
        &self,
        path: &syn::TypePath,
        nominal: &crate::concrete_types::Nominal,
    ) -> Result<(), String> {
        let names = Scoped {
            resolution: self,
            namespace: &self.namespace,
        }
        .name(path)?;
        self.check_nominal(&names, nominal)
    }

    fn check_nominal(
        &self,
        names: &Namespace,
        nominal: &crate::concrete_types::Nominal,
    ) -> Result<(), String> {
        let declaration = self
            .environment
            .index
            .declarations
            .get(names)
            .ok_or("Missing lexical nominal owner declaration")?;
        let source = self.environment.source(&declaration.path)?;
        let package = &self.environment.census().context.package;

        if nominal.definition_path != names.join("::")
            || nominal.source_identity.as_deref() != Some(package.package_id.as_str())
            || nominal.package_name != package.name
            || nominal.package_version != package.version
            || nominal.manifest_digest != package.manifest_digest
            || nominal.declaration_digest
                != concrete_types::digest(source.range(declaration.span)?.as_bytes())
        {
            return Err("Nominal owner contradicts the exact lexical declaration/package".into());
        }
        Ok(())
    }
}

pub(crate) fn selected_cfgs(facts: &HirFnFacts) -> Result<Option<&[String]>, String> {
    Ok(supports(facts, "rust.copy-nominal-boundary")?.then_some(facts.build.cfgs.as_slice()))
}

fn struct_span(bytes: &str, item: &syn::ItemStruct) -> ByteSpan {
    let start = match item.vis {
        syn::Visibility::Inherited => item.struct_token.span,
        _ => item.vis.span(),
    };
    ByteSpan {
        start: source_span(bytes, start).start,
        end: source_span(bytes, item.span()).end,
    }
}

#[cfg(test)]
mod imports_tests {
    use super::*;

    #[test]
    fn grouped_self_renames_and_external_roots_retain_exact_bindings() {
        let mut values = BTreeMap::new();
        let namespace = vec!["api".into()];
        for source in [
            "use core::fmt::{self, Debug};",
            "use core::cmp::{self as compare, Ordering};",
        ] {
            let item: syn::ItemUse = syn::parse_str(source).unwrap();
            Imports {
                values: &mut values,
                namespace: &namespace,
                selected: true,
            }
            .tree(&item.tree, Vec::new())
            .unwrap();
        }
        assert_eq!(values[&(namespace.clone(), "fmt".into())], ["core", "fmt"]);
        assert_eq!(
            values[&(namespace.clone(), "compare".into())],
            ["core", "cmp"]
        );
        assert_eq!(
            values[&(namespace.clone(), "Debug".into())],
            ["core", "fmt", "Debug"]
        );
        assert!(!values.contains_key(&(namespace.clone(), "self".into())));
        let item: syn::ItemUse = syn::parse_str("use ::core::fmt as external;").unwrap();
        Imports {
            values: &mut values,
            namespace: &namespace,
            selected: true,
        }
        .tree(&item.tree, vec!["<external>".into()])
        .unwrap();
        assert_eq!(
            values[&(namespace, "external".into())],
            ["<external>", "core", "fmt"]
        );
    }
}
