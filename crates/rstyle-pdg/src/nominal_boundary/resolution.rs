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
use std::{collections::BTreeMap, path::PathBuf};
use syn::spanned::Spanned;

type Namespace = Vec<String>;

pub struct Resolution {
    pub namespace: Namespace,
    pub receiver: Option<TypeTree>,
    trees: BTreeMap<Namespace, TypeTree>,
    imports: BTreeMap<(Namespace, String), Namespace>,
}

struct Declaration {
    path: PathBuf,
    span: ByteSpan,
    namespace: Namespace,
}

pub fn verify(facts: &HirFnFacts, interface: &InterfaceObservations) -> Result<Resolution, String> {
    let census: census::Census = evidence(facts, census::KEY)?;
    let context: UnitContext = evidence(facts, CONTEXT_KEY)?;
    if context != census.context {
        return Err("Nominal census belongs to another selected unit".into());
    }
    let modules = census::verify(&census)?;
    let index = build_index(&modules, &census)?;

    let origins: declaration_origins::Origins = evidence(facts, declaration_origins::KEY)?;
    declaration_origins::verify(interface, &origins)?;
    let mut resolution = Resolution {
        namespace: Vec::new(),
        receiver: None,
        trees: BTreeMap::new(),
        imports: index.imports,
    };

    for observation in interface.parameters.iter().chain([&interface.result]) {
        if let Observation::Captured { ty } = observation {
            collect(ty, &mut resolution.trees)?;
        }
    }
    verify_declarations(&resolution, &index.declarations, &origins, &census)?;
    function_scope(facts, &modules, &census, &mut resolution)?;
    Ok(resolution)
}

fn file_bytes<'a>(census: &'a census::Census, path: &std::path::Path) -> Result<&'a str, String> {
    census
        .files
        .iter()
        .find(|file| file.path == path)
        .map(|file| file.bytes.as_str())
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
}
fn build_index(modules: &[census::Module], census: &census::Census) -> Result<Index, String> {
    let mut index = Index {
        declarations: BTreeMap::new(),
        imports: BTreeMap::new(),
    };
    for module in modules {
        index.items(module, file_bytes(census, &module.path)?)?;
    }
    for (namespace, name) in index.imports.keys() {
        let key: Vec<_> = namespace.iter().cloned().chain([name.clone()]).collect();
        if index.declarations.contains_key(&key) {
            return Err("Import shadows a nominal declaration".into());
        }
    }
    Ok(index)
}
impl Index {
    fn items(&mut self, module: &census::Module, bytes: &str) -> Result<(), String> {
        for item in &module.items {
            match item {
                syn::Item::Struct(item) => {
                    let mut key = module.namespace.clone();
                    key.push(item.ident.to_string());
                    if concrete_types::lifetimes::parameter_count(&item.generics).is_err()
                        || self
                            .declarations
                            .insert(
                                key,
                                Declaration {
                                    path: module.path.clone(),
                                    span: source_span(bytes, item.span()),
                                    namespace: module.namespace.clone(),
                                },
                            )
                            .is_some()
                    {
                        return Err("Generic or ambiguous nominal declaration".into());
                    }
                }
                syn::Item::Use(item) if item.leading_colon.is_none() => {
                    imports_tree(&item.tree, Vec::new(), &module.namespace, &mut self.imports)?;
                }
                syn::Item::Use(_) => {
                    return Err("External import lacks captured nominal evidence".into());
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn imports_tree(
    tree: &syn::UseTree,
    mut path: Namespace,
    namespace: &Namespace,
    imports: &mut BTreeMap<(Namespace, String), Namespace>,
) -> Result<(), String> {
    match tree {
        syn::UseTree::Path(item) => {
            path.push(item.ident.to_string());
            imports_tree(&item.tree, path, namespace, imports)
        }
        syn::UseTree::Group(item) => {
            for child in &item.items {
                imports_tree(child, path.clone(), namespace, imports)?;
            }
            Ok(())
        }
        leaf => {
            let (name, alias) = match leaf {
                syn::UseTree::Name(item) => (item.ident.to_string(), item.ident.to_string()),
                syn::UseTree::Rename(item) => (item.ident.to_string(), item.rename.to_string()),
                _ => {
                    return Err(
                        "Glob imports require unavailable lexical resolution evidence".into(),
                    );
                }
            };
            path.push(name);
            insert_import(alias, path, namespace, imports)
        }
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
        return Err("Ambiguous named import".into());
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
            || origin.file_bytes != file_bytes(census, &declaration.path)?
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
        candidates.extend(module_functions(module, bytes, facts));
    }

    let [(namespace, receiver)] = candidates.as_slice() else {
        return Err("Missing or ambiguous exact lexical function ownership".into());
    };
    resolution.namespace = namespace.clone();
    if let Some(receiver) = receiver {
        let syn::Type::Path(path) = receiver.as_ref() else {
            return Err("Unsupported inherent receiver type".into());
        };
        resolution.receiver = Some(resolution.resolve(path)?);
    }
    Ok(())
}

fn module_functions(
    module: &census::Module,
    bytes: &str,
    facts: &HirFnFacts,
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
                candidates.push((module.namespace.clone(), None))
            }
            syn::Item::Impl(item) => {
                for member in &item.items {
                    if let syn::ImplItem::Fn(method) = member
                        && (FunctionSource {
                            vis: &method.vis,
                            sig: &method.sig,
                            body: &method.block,
                        })
                        .exact(bytes, facts)
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
            && bytes.get(range.start as usize..range.end as usize) == Some(&facts.source.bytes)
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
                .receiver
                .clone()
                .ok_or_else(|| "Self lacks exact owning impl proof".into());
        }
        if let Some(first) = names.first()
            && let Some(import) = self
                .resolution
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
