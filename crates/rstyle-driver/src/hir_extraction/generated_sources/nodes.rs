//! Exact final facts-tree roles, anchored to uncollapsed compiler HIR spans.
use super::*;
use rustc_hir::{self as hir, Node, PatKind, StmtKind};

pub(super) fn capture(
    provenance: &mut Provenance<'_, '_>,
    facts: &HirFnFacts,
) -> Result<Vec<NodeOrigin>, String> {
    let mut traversal = Traversal {
        provenance,
        records: Vec::new(),
    };
    for (index, parameter) in facts.signature.params.iter().enumerate() {
        let index = u32::try_from(index).map_err(|_| "Generated parameter index exceeds u32")?;
        traversal.walk(&parameter.pat, &mut vec![0, index])?;
    }

    traversal.walk(&facts.body, &mut vec![1])?;
    Ok(traversal.records)
}

struct Traversal<'a, 'source, 'tcx> {
    provenance: &'a mut Provenance<'source, 'tcx>,
    records: Vec<NodeOrigin>,
}

impl Traversal<'_, '_, '_> {
    fn walk(&mut self, node: &rstyle_pdg::HirNode, path: &mut Vec<u32>) -> Result<(), String> {
        if self.records.len() >= 32768 || path.len() > 256 {
            return Err("Generated facts role inventory exceeds diagnostic budget".into());
        }

        self.record(node, path)?;
        for (index, child) in node.children.iter().enumerate() {
            self.child(index, child, path)?;
        }
        Ok(())
    }

    fn record(&mut self, node: &rstyle_pdg::HirNode, path: &[u32]) -> Result<(), String> {
        let id = super::super::instances::diagnostic_support::hir_id(
            &node.hir,
            "Generated node belongs to another compiler crate",
        )?;
        let raw = raw_span(self.provenance.tcx, id, &node.kind)?;
        let origin = self.provenance.origin(raw)?;

        self.records.push(NodeOrigin {
            path: path.to_vec(),
            hir: node.hir.clone(),
            kind: node.kind.clone(),
            origin,
        });
        Ok(())
    }

    fn child(
        &mut self,
        index: usize,
        node: &rstyle_pdg::HirNode,
        path: &mut Vec<u32>,
    ) -> Result<(), String> {
        path.push(u32::try_from(index).map_err(|_| "Generated child index exceeds u32")?);
        self.walk(node, path)?;
        path.pop();
        Ok(())
    }
}

fn raw_span(tcx: TyCtxt<'_>, id: hir::HirId, kind: &str) -> Result<Span, String> {
    match tcx.hir_node(id) {
        Node::Expr(expression) => Ok(expression.span),
        Node::Pat(pattern) => Ok(match pattern.kind {
            PatKind::Binding(_, _, ident, None) if kind == "pat_binding" => ident.span,
            _ => pattern.span,
        }),
        Node::Stmt(statement) => Ok(match statement.kind {
            StmtKind::Item(item) if kind == "stmt_item" => tcx.hir_item(item).span,
            _ => statement.span,
        }),
        Node::Block(block) => Ok(block.span),
        Node::Arm(arm) => Ok(arm.span),
        Node::LetStmt(local) => Ok(local.span),
        Node::Ty(ty) => Ok(ty.span),
        _ => Err(format!(
            "Generated facts role {kind} has no exact raw HIR span capture"
        )),
    }
}
