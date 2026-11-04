//! The compiler's generated-source observations as document extensions: the unit's
//! local bang expansions and the definitions they emit, and each generated function's
//! sites, recorded completely so the reader's selection and provenance walk need
//! nothing else.

use super::provenance::{definition_record, identity};
use super::{Roots, implementation};
use crate::hir_extraction::{Options, module_expansions::file_span, sources::Sources};
use rstyle_pdg::generated_sources::capture::{Mark, Transparency};
use rustc_hir::{Node, PatKind, StmtKind, def_id::LocalDefId};
use rustc_middle::ty::TyCtxt;
use rustc_span::hygiene::{ExpnKind, MacroKind, Transparency as CompilerTransparency};
use rustc_span::{ExpnId, Span};
use serde_json::{Map, Value, json};

/// Expansion ancestry the reader's walk can reach before its own budget stops it.
const MAX_DEPTH: usize = 256;

/// Whether the profile, or the diagnostic switch, requests the capture.
fn requested(options: &Options<'_>) -> bool {
    options
        .selected()
        .supports(rstyle_pdg::result_maps::CAPABILITY)
        || rstyle_pdg::diagnostics::switch(
            rstyle_pdg::cpg_reader::generated_sources::DIAGNOSTIC_ENV,
        )
}

/// The unit's `rust:generated-sources` extension: each local definition with a local
/// bang root, in compiler order, or whose root could not be found, and the expansions
/// they reach.
pub(in crate::hir_extraction) fn unit(tcx: TyCtxt<'_>, options: &Options<'_>) -> Option<Value> {
    requested(options).then_some(())?;
    let mut roots = Roots::new(tcx);
    let mut recorder = Recorder::new(tcx, &options.sources);
    let mut definitions = Vec::new();
    for id in tcx.iter_local_def_id() {
        let definition = definition_record(tcx, id.to_def_id());
        let root = match roots.definition(id) {
            Ok(None) => continue,
            Ok(Some(root)) => root,
            Err(error) => {
                definitions.push(json!({"definition": definition, "root": {"error": error}}));
                continue;
            }
        };
        definitions.push(recorder.candidate(id, root, definition));
    }

    Some(json!({"expansions": recorder.expansions, "definitions": definitions}))
}

// rstyle-allow(mutation_per_target): the recorder accumulates every expansion the function's sites reach
/// A generated function's `rust:generated-source` extension: an expanded callable with
/// its selected implementation, its root or why it has none, and the sites of its
/// signature, body and every node of its owner.
pub(in crate::hir_extraction) fn function(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    id: LocalDefId,
) -> Option<Value> {
    requested(options).then_some(())?;
    let implementation = implementation(tcx, id)?;
    let node = tcx.hir_node_by_def_id(id);
    let signature = node
        .fn_sig()
        .filter(|signature| signature.span.from_expansion())?;
    let body = tcx.hir_body(node.body_id()?);
    let mut recorder = Recorder::new(tcx, &options.sources);
    let root = match Roots::new(tcx).definition(id) {
        Ok(Some(root)) => {
            recorder.expansion(root, 0);
            json!(identity(root))
        }
        Ok(None) => {
            json!({"error": "Expanded callable has no unique original local bang expansion"})
        }
        Err(error) => json!({"error": error}),
    };
    let signature = recorder.site(signature.span);
    let body = recorder.site(body.value.span);
    let nodes = recorder.nodes(id);

    Some(
        json!({"root": root, "owner": definition_record(tcx, id.to_def_id()),
                "implementation": definition_record(tcx, implementation),
                "signature": signature, "body": body, "nodes": nodes,
                "expansions": recorder.expansions}),
    )
}

struct Recorder<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    sources: &'a Sources,
    expansions: Map<String, Value>,
}

impl<'a, 'tcx> Recorder<'a, 'tcx> {
    fn new(tcx: TyCtxt<'tcx>, sources: &'a Sources) -> Self {
        Self {
            tcx,
            sources,
            expansions: Map::new(),
        }
    }

    /// A definition whose root is a local bang expansion, with what its roster entry needs.
    fn candidate(
        &mut self,
        id: LocalDefId,
        root: ExpnId,
        definition: impl serde::Serialize,
    ) -> Value {
        let tcx = self.tcx;
        self.expansion(root, 0);
        let defined = tcx.expn_that_defined(id.to_def_id());
        self.expansion(defined, 0);
        let site = self.site(tcx.def_span(id));

        json!({"definition": definition, "root": identity(root),
               "parent": definition_record(tcx, tcx.parent(id.to_def_id())),
               "kind": format!("{:?}", tcx.def_kind(id)),
               "defined": (defined != ExpnId::root()).then(|| identity(defined)),
               "site": site})
    }

    /// Where a span is: dummy, unlocatable, or its file span with its context's marks.
    fn site(&mut self, span: Span) -> Value {
        if span.is_dummy() {
            return json!({"dummy": true});
        }
        let raw = match file_span(self.tcx, self.sources, span) {
            Ok(raw) => raw,
            Err(error) => return json!({"error": error}),
        };
        let marks: Vec<Mark> = span
            .ctxt()
            .marks()
            .into_iter()
            .map(|(expansion, transparency)| {
                self.expansion(expansion, 0);
                Mark {
                    expansion: identity(expansion),
                    transparency: match transparency {
                        CompilerTransparency::Opaque => Transparency::Opaque,
                        CompilerTransparency::SemiOpaque => Transparency::SemiOpaque,
                        CompilerTransparency::Transparent => Transparency::Transparent,
                    },
                }
            })
            .collect();

        json!({"raw": raw, "marks": marks})
    }

    // rstyle-allow(mutation_per_target): records each expansion once, ancestry first
    /// Records an expansion and the ancestry its record reaches.
    fn expansion(&mut self, id: ExpnId, depth: usize) {
        let key = identity(id);
        if id == ExpnId::root() || depth > MAX_DEPTH || self.expansions.contains_key(&key) {
            return;
        }
        self.expansions.insert(key.clone(), Value::Null);
        let data = id.expn_data();
        self.expansion(data.parent, depth + 1);
        let call_site = self.site(data.call_site);
        let definition = data
            .macro_def_id
            .filter(|id| id.is_local())
            .map(|definition| {
                match file_span(self.tcx, self.sources, self.tcx.def_span(definition)) {
                    Ok(span) => json!(span),
                    Err(error) => json!({"error": error}),
                }
            });
        let (kind, name) = match data.kind {
            ExpnKind::Macro(kind, name) => (
                json!(match kind {
                    MacroKind::Bang => "Bang",
                    MacroKind::Attr => "Attr",
                    MacroKind::Derive => "Derive",
                }),
                name.to_string(),
            ),
            ExpnKind::Desugaring(kind) => (json!("Desugaring"), format!("{kind:?}")),
            ExpnKind::AstPass(kind) => (json!("AstPass"), format!("{kind:?}")),
            ExpnKind::Root => (
                json!({"error": "Root expansion cannot supply generated ownership"}),
                String::new(),
            ),
        };
        let record = json!({"parent": (data.parent != ExpnId::root()).then(|| identity(data.parent)),
                            "call_site": call_site, "definition": definition, "kind": kind, "name": name,
                            "macro_definition": data.macro_def_id.map(|id| definition_record(self.tcx, id)),
                            "edition": data.edition.to_string(),
                            "local_inner_macros": data.local_inner_macros});

        self.expansions.insert(key, record);
    }

    /// The sites of every HIR node of the function's owner, by node ID.
    fn nodes(&mut self, id: LocalDefId) -> Map<String, Value> {
        let tcx = self.tcx;
        let owner = tcx.hir_owner_nodes(rustc_hir::OwnerId { def_id: id });
        let mut nodes = Map::new();
        for (local, _) in owner.nodes.iter_enumerated() {
            let hir_id = rustc_hir::HirId {
                owner: rustc_hir::OwnerId { def_id: id },
                local_id: local,
            };
            let key = format!("{}.{}", id.local_def_index.as_u32(), local.as_u32());
            nodes.insert(key, self.node(tcx.hir_node(hir_id)));
        }

        nodes
    }

    /// A node's own span, and the identifier or item span a binding without a
    /// subpattern or an item statement is placed at.
    fn node(&mut self, node: Node<'_>) -> Value {
        let (span, ident, item) = match node {
            Node::Expr(expression) => (expression.span, None, None),
            Node::Pat(pattern) => match pattern.kind {
                PatKind::Binding(_, _, ident, None) => (pattern.span, Some(ident.span), None),
                _ => (pattern.span, None, None),
            },
            Node::Stmt(statement) => match statement.kind {
                StmtKind::Item(item) => (statement.span, None, Some(self.tcx.hir_item(item).span)),
                _ => (statement.span, None, None),
            },
            Node::Block(block) => (block.span, None, None),
            Node::Arm(arm) => (arm.span, None, None),
            Node::LetStmt(local) => (local.span, None, None),
            Node::Ty(ty) => (ty.span, None, None),
            _ => return json!({"unsupported": true}),
        };
        let mut sites = json!({"span": self.site(span)});
        if let Some(ident) = ident {
            sites["ident"] = self.site(ident);
        }
        if let Some(item) = item {
            sites["item"] = self.site(item);
        }

        sites
    }
}
