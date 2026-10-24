//! Preserve original grouped use ownership and every resolved leaf endpoint.
use super::*;
use crate::hir_extraction::{crate_association, module_expansions::file_span};
use rstyle_pdg::dependency_namespaces::capture::{Binding, Declaration, Import, Site};
use rstyle_pdg::nominal_boundary::census::expansions::FileSpan;
use rustc_hir::{Item, UseKind, UseTree, def::DefKind, def::Res, def_id::LocalDefId};
use rustc_span::{ExpnId, kw};

pub(super) fn capture(
    mut context: Context<'_, '_>,
    item: &Item<'_>,
    tree: UseTree<'_>,
) -> Result<(), String> {
    let definition = item.owner_id.to_def_id();
    let owner = match original_owner(context.tcx, item) {
        Ok(owner) => owner,
        Err(reason) => return context.unsupported(definition, reason),
    };

    let declaration = match file_span(context.tcx, context.sources, item.span) {
        Ok(source) => source,
        Err(reason) => return context.unsupported(definition, reason),
    };
    let mut capture = Leaves {
        context,
        declaration,
        owner,
    };
    capture.walk(
        tree,
        item.owner_id.def_id,
        Traversal {
            prefix: &[],
            depth: 0,
        },
    )
}

struct Owner {
    definition: DefId,
    scope: Vec<String>,
}

fn original_owner(tcx: TyCtxt<'_>, item: &Item<'_>) -> Result<Owner, String> {
    if item.span.from_expansion()
        || tcx.expn_that_defined(item.owner_id.to_def_id()) != ExpnId::root()
    {
        return Err("Generated dependency namespace import site".into());
    }
    let definition = tcx.parent(item.owner_id.to_def_id());
    let mut cursor = definition;
    let mut scope = Vec::new();
    loop {
        if scope.len() >= MAX_FILES || tcx.def_kind(cursor) != DefKind::Mod {
            return Err("Dependency namespace import is outside an original module scope".into());
        }
        if tcx.expn_that_defined(cursor) != ExpnId::root() {
            return Err("Dependency namespace import has a generated owning module".into());
        }
        if cursor.is_crate_root() {
            break;
        }
        if tcx.def_span(cursor).from_expansion() {
            return Err("Dependency namespace scope has a generated declaration".into());
        }
        scope.push(tcx.item_name(cursor).to_string());
        cursor = tcx.parent(cursor);
    }

    scope.reverse();
    Ok(Owner { definition, scope })
}

struct Leaves<'a, 'tcx> {
    context: Context<'a, 'tcx>,
    declaration: FileSpan,
    owner: Owner,
}

#[derive(Clone, Copy)]
struct Traversal<'a> {
    prefix: &'a [String],
    depth: usize,
}

struct Leaf {
    definition: LocalDefId,
    path: Vec<String>,
    binding: Binding,
}

struct Endpoint {
    definition: LocalDefId,
    kind: DefKind,
    target: DefId,
}

impl Leaves<'_, '_> {
    fn walk(
        &mut self,
        tree: UseTree<'_>,
        definition: LocalDefId,
        traversal: Traversal<'_>,
    ) -> Result<(), String> {
        let Traversal { prefix, depth } = traversal;
        if depth >= MAX_FILES || prefix.len() + tree.prefix.segments.len() > MAX_FILES {
            return Err("Dependency namespace use tree exceeds capture depth budget".into());
        }

        let Some(path) = self.path(tree, definition, traversal)? else {
            return Ok(());
        };

        let binding = match tree.kind {
            UseKind::Nested { items } => {
                return self.children(
                    items,
                    Traversal {
                        prefix: &path,
                        depth,
                    },
                );
            }
            UseKind::Glob => Binding::Glob,
            UseKind::Single(ident) => binding(ident),
        };
        self.leaf(
            tree,
            Leaf {
                definition,
                path,
                binding,
            },
        )
    }

    fn path(
        &mut self,
        tree: UseTree<'_>,
        definition: LocalDefId,
        traversal: Traversal<'_>,
    ) -> Result<Option<Vec<String>>, String> {
        let original = match original_path(self.context.tcx, self.context.sources, tree) {
            Ok(path) => path,
            Err(reason) => {
                self.unsupported(definition, reason)?;
                return Ok(None);
            }
        };
        let mut path = traversal.prefix.to_vec();

        for name in original {
            self.context.budget.text(&name)?;
            path.push(name);
        }
        Ok(Some(path))
    }

    fn children(
        &mut self,
        items: &[(UseTree<'_>, rustc_hir::HirId, LocalDefId)],
        traversal: Traversal<'_>,
    ) -> Result<(), String> {
        for (child, _, definition) in items {
            self.walk(
                *child,
                *definition,
                Traversal {
                    prefix: traversal.prefix,
                    depth: traversal.depth + 1,
                },
            )?;
        }
        Ok(())
    }

    fn leaf(&mut self, tree: UseTree<'_>, leaf: Leaf) -> Result<(), String> {
        let source = match self.leaf_source(tree, leaf.definition) {
            Ok(source) => source,
            Err(reason) => return self.unsupported(leaf.definition, reason),
        };

        let mut resolved = false;
        for resolution in tree.prefix.res.iter().flatten() {
            resolved = true;
            self.resolution(&leaf, &source, *resolution)?;
        }

        if !resolved {
            self.unsupported(
                leaf.definition,
                "Namespace use leaf has no compiler resolution",
            )?;
        }
        Ok(())
    }

    fn resolution(
        &mut self,
        leaf: &Leaf,
        source: &FileSpan,
        resolution: Res,
    ) -> Result<(), String> {
        let Res::Def(kind, target) = resolution else {
            return self.unsupported(
                leaf.definition,
                format!("Unhandled namespace use resolution: {resolution:?}"),
            );
        };
        let endpoint = Endpoint {
            definition: leaf.definition,
            kind,
            target,
        };
        let site = self.site(leaf, source, &endpoint)?;

        self.endpoint(site, endpoint)
    }

    fn leaf_source(&self, tree: UseTree<'_>, definition: LocalDefId) -> Result<FileSpan, String> {
        let span = self.context.tcx.def_span(definition);
        if span.from_expansion() || tree.prefix.span.from_expansion() {
            return Err("Generated dependency namespace use leaf".into());
        }

        let leaf = file_span(self.context.tcx, self.context.sources, span)?;
        if leaf.path != self.declaration.path
            || leaf.digest != self.declaration.digest
            || leaf.span.start < self.declaration.span.start
            || leaf.span.end > self.declaration.span.end
        {
            return Err("Use leaf leaves its original grouped declaration".into());
        }
        Ok(leaf)
    }

    fn site(
        &mut self,
        leaf: &Leaf,
        source: &FileSpan,
        endpoint: &Endpoint,
    ) -> Result<Site, String> {
        let context = &mut self.context;
        context.budget.record()?;
        for text in self.owner.scope.iter().chain(&leaf.path) {
            context.budget.text(text)?;
        }
        let definitions = context
            .budget
            .site_definitions(context.tcx, &self.owner, endpoint)?;

        Ok(Site {
            definition: definitions.leaf,
            declaration: self.declaration.clone(),
            leaf: source.clone(),
            owner: definitions.owner,
            scope: self.owner.scope.clone(),
            path: leaf.path.clone(),
            binding: leaf.binding.clone(),
            public: context.tcx.visibility(leaf.definition).is_public(),
            target: definitions.target,
            target_kind: format!("{:?}", endpoint.kind),
        })
    }

    fn endpoint(&mut self, site: Site, endpoint: Endpoint) -> Result<(), String> {
        let Endpoint {
            definition,
            kind,
            target,
        } = endpoint;
        if target.is_local() {
            self.context.output.declarations.push(Declaration { site });
            return Ok(());
        }

        if kind != DefKind::Mod {
            return self.context.unsupported(
                definition.to_def_id(),
                format!("Foreign namespace use endpoint is not a module: {kind:?}"),
            );
        }
        self.import(site, target)
    }

    fn import(&mut self, site: Site, target: DefId) -> Result<(), String> {
        let Some(artifact) = crate_association::namespace_artifact(self.context.tcx, target) else {
            return self
                .context
                .unsupported(target, "Foreign namespace lacks loaded metadata artifact");
        };
        let inventory = self.import_inventory(target)?;

        self.context.output.imports.push(Import {
            site,
            compiled_dependency: serde_json::to_value(artifact)
                .map_err(|error| error.to_string())?,
            exports: inventory.exports,
            complete: inventory.complete,
        });
        Ok(())
    }

    fn import_inventory(&mut self, target: DefId) -> Result<exports::Inventory, String> {
        let mut inventory = exports::capture(self.context.tcx, target, self.context.budget)?;
        for reason in inventory.unsupported.drain(..) {
            self.context.unsupported(target, reason)?;
        }
        Ok(inventory)
    }

    fn unsupported(
        &mut self,
        definition: LocalDefId,
        reason: impl Into<String>,
    ) -> Result<(), String> {
        self.context.unsupported(definition.to_def_id(), reason)
    }
}

/// HIR removes trailing `self`; read the original path spelling from the
/// existing immutable buffer instead. Unhandled token syntax stays unsupported.
fn original_path(
    tcx: TyCtxt<'_>,
    sources: &Sources,
    tree: UseTree<'_>,
) -> Result<Vec<String>, String> {
    if tree.prefix.segments.is_empty() && matches!(tree.kind, UseKind::Nested { .. }) {
        return Ok(Vec::new());
    }
    let (source, _) = sources
        .range(tcx, tree.prefix.span)
        .ok_or("Namespace path has no original source range")?;

    let mut text = source.trim();
    let mut path = Vec::new();
    if let Some(rest) = text.strip_prefix("::") {
        path.push("::".into());
        text = rest;
    }
    if text.is_empty() {
        return Ok(path);
    }

    for segment in text.split("::") {
        let segment = segment.trim();
        let name = segment.strip_prefix("r#").unwrap_or(segment);
        if !identifier(name) {
            return Err("Namespace path contains unsupported original token syntax".into());
        }
        path.push(segment.into());
        if path.len() > MAX_FILES {
            return Err("Namespace path exceeds capture segment budget".into());
        }
    }
    Ok(path)
}

fn binding(ident: rustc_span::Ident) -> Binding {
    if ident.name == kw::Underscore {
        Binding::Anonymous
    } else {
        Binding::Named(ident.name.to_string())
    }
}

fn identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_alphanumeric())
}

struct SiteDefinitions {
    leaf: rstyle_pdg::generated_sources::capture::Definition,
    owner: rstyle_pdg::generated_sources::capture::Definition,
    target: rstyle_pdg::generated_sources::capture::Definition,
}

impl Budget {
    fn site_definitions(
        &mut self,
        tcx: TyCtxt<'_>,
        owner: &Owner,
        endpoint: &Endpoint,
    ) -> Result<SiteDefinitions, String> {
        let leaf = self.definition(tcx, endpoint.definition.to_def_id())?;
        let owner = self.definition(tcx, owner.definition)?;
        let target = self.definition(tcx, endpoint.target)?;

        Ok(SiteDefinitions {
            leaf,
            owner,
            target,
        })
    }
}
