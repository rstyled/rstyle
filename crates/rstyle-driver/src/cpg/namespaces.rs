//! The unit's namespaces and imports: each module, and each item whose body holds an
//! import, as a namespace; each `use` leaf and `extern crate` as an import, once per
//! namespace (type, value or macro) its path resolves in.

use super::unit::Unit;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{CRATE_DEF_ID, DefId, LocalDefId};
use rustc_hir::{ItemKind, UseKind, UseTree};
use rustc_span::{Span, kw};
use serde_json::{Map, Value, json};

/// The unit's namespaces and imports, recording what they name in `unit`.
pub fn record(unit: &mut Unit<'_>) -> (Vec<Value>, Vec<Value>) {
    let tcx = unit.tcx;
    let mut recorder = Recorder {
        unit,
        namespaces: Map::new(),
        imports: Vec::new(),
    };
    let modules = tcx.hir_crate_items(()).definitions();
    let modules = modules.filter(|id| tcx.def_kind(*id) == DefKind::Mod);
    for module in std::iter::once(CRATE_DEF_ID).chain(modules) {
        recorder.namespace(module.to_def_id());
    }
    for id in tcx.hir_free_items() {
        let item = tcx.hir_item(id);
        match item.kind {
            ItemKind::Use(tree) => recorder.tree(tree, item.owner_id.def_id, String::new()),
            ItemKind::ExternCrate(original, ident) => recorder.extern_crate(item, original, ident),
            _ => {}
        }
    }

    (
        recorder.namespaces.into_values().collect(),
        recorder.imports,
    )
}

struct Recorder<'u, 'tcx> {
    unit: &'u mut Unit<'tcx>,
    namespaces: Map<String, Value>,
    imports: Vec<Value>,
}

impl Recorder<'_, '_> {
    // rstyle-allow(mutation_per_target): records each namespace once, the namespaces enclosing it first
    /// The namespace ID of a module, or of another item whose body holds imports,
    /// recording it and the namespaces enclosing it.
    fn namespace(&mut self, def_id: DefId) -> String {
        let tcx = self.unit.tcx;
        let id = format!("ns:{}", self.unit.stable_identity(def_id));
        if self.namespaces.contains_key(&id) {
            return id;
        }
        let crate_name = tcx.crate_name(rustc_span::def_id::LOCAL_CRATE);
        let name = match def_id.is_crate_root() {
            true => crate_name.to_string(),
            false => format!("{crate_name}::{}", tcx.def_path_str(def_id)),
        };
        let parent = tcx.opt_parent(def_id).map(|parent| self.namespace(parent));
        let file = self.file(self.defining_span(def_id));
        let record = present(json!({"id": id, "name": name, "parent": parent, "file": file}));

        self.namespaces.insert(id.clone(), record);
        id
    }

    /// Where a namespace is defined: a module's own contents, else its declaration.
    fn defining_span(&self, def_id: DefId) -> Span {
        let tcx = self.unit.tcx;
        let local = def_id.expect_local();
        match tcx.hir_node_by_def_id(local) {
            rustc_hir::Node::Item(item) => match item.kind {
                ItemKind::Mod(_, module) => module.spans.inner_span,
                _ => item.span,
            },
            rustc_hir::Node::Crate(module) => module.spans.inner_span,
            _ => tcx.def_span(def_id),
        }
    }

    fn file(&mut self, span: Span) -> Option<String> {
        let span = self.unit.span(span.source_callsite())?;
        span["file"].as_str().map(str::to_owned)
    }

    /// Each leaf of a `use` tree, its path the written prefix of its ancestors and its own.
    fn tree(&mut self, tree: UseTree<'_>, leaf: LocalDefId, prefix: String) {
        let path = joined(&prefix, &segments(tree));
        match tree.kind {
            UseKind::Nested { items } => {
                for (child, _, definition) in items {
                    self.tree(*child, *definition, path.clone());
                }
            }
            UseKind::Glob => self.import(leaf, joined(&path, "*"), resolutions(tree)),
            UseKind::Single(ident) => {
                let written = match tree.prefix.segments.last() {
                    Some(last) if last.ident.name != ident.name => format!("{path} as {ident}"),
                    _ => path,
                };
                self.import(leaf, written, resolutions(tree));
            }
        }
    }

    fn extern_crate(
        &mut self,
        item: &rustc_hir::Item<'_>,
        original: Option<rustc_span::Symbol>,
        ident: rustc_span::Ident,
    ) {
        let tcx = self.unit.tcx;
        let path = match original {
            Some(original) => format!("{original} as {ident}"),
            None => ident.to_string(),
        };
        let target = tcx
            .extern_mod_stmt_cnum(item.owner_id.def_id)
            .map(|krate| Res::Def(DefKind::Mod, krate.as_def_id()));
        let resolutions = vec![target.unwrap_or(Res::Err)];

        self.import(item.owner_id.def_id, path, resolutions);
    }

    // rstyle-allow(mutation_per_target): records one import per namespace the path resolves in
    /// One import per resolution of a leaf, or one unresolved import when it has none.
    fn import(&mut self, leaf: LocalDefId, path: String, resolutions: Vec<Res>) {
        let tcx = self.unit.tcx;
        let namespace = self.namespace(scope(tcx, leaf));
        let span = self.unit.span(tcx.def_span(leaf).source_callsite());
        let id = format!("import:{}", self.unit.stable_identity(leaf.to_def_id()));
        // A path that resolves nowhere is one unresolved import.
        let mut resolutions: Vec<Res> = resolutions
            .into_iter()
            .filter(|resolution| *resolution != Res::Err)
            .collect();
        if resolutions.is_empty() {
            resolutions.push(Res::Err);
        }
        let suffixed = resolutions.len() > 1;

        for resolution in resolutions {
            let id = match suffixed {
                true => format!("{id}#{}", namespace_of(resolution)),
                false => id.clone(),
            };
            let (target, reason) = match resolution {
                Res::Def(_, target) => (Some(self.unit.definition(target)), None),
                Res::Err => (
                    None,
                    Some("The compiler records no resolution of the path".into()),
                ),
                other => (None, Some(format!("Resolves to {}", other.descr()))),
            };
            let record = json!({"id": id, "namespace": namespace, "path": path,
                                "target": target, "reason": reason, "span": span});
            self.imports.push(present(record));
        }
    }
}

/// The scope that holds an import: a nested leaf belongs to its `use` item, which the
/// enclosing module or item holds.
fn scope(tcx: rustc_middle::ty::TyCtxt<'_>, leaf: LocalDefId) -> DefId {
    let mut scope = tcx.parent(leaf.to_def_id());
    while tcx.def_kind(scope) == DefKind::Use {
        scope = tcx.parent(scope);
    }

    scope
}

/// A record without its absent fields.
fn present(mut record: Value) -> Value {
    if let Value::Object(fields) = &mut record {
        fields.retain(|_, value| !value.is_null());
    }

    record
}

/// A `use` tree's own path segments as written; the root of an absolute path is empty.
fn segments(tree: UseTree<'_>) -> String {
    let names: Vec<String> = tree
        .prefix
        .segments
        .iter()
        .map(|segment| match segment.ident.name {
            kw::PathRoot => String::new(),
            name => name.to_string(),
        })
        .collect();
    names.join("::")
}

fn joined(prefix: &str, rest: &str) -> String {
    match (prefix.is_empty(), rest.is_empty()) {
        (true, _) => rest.to_owned(),
        (false, true) => prefix.to_owned(),
        (false, false) => format!("{prefix}::{rest}"),
    }
}

/// A leaf's resolutions, one per namespace it resolves in.
fn resolutions(tree: UseTree<'_>) -> Vec<Res> {
    tree.prefix.res.iter().flatten().copied().collect()
}

/// The namespace a resolution is in: type, value or macro.
fn namespace_of(resolution: Res) -> &'static str {
    match resolution.ns() {
        Some(rustc_hir::def::Namespace::TypeNS) => "type",
        Some(rustc_hir::def::Namespace::ValueNS) => "value",
        Some(rustc_hir::def::Namespace::MacroNS) => "macro",
        None => "other",
    }
}
