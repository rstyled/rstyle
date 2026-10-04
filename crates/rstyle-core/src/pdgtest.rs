//! Hand-built dependence graphs shared by the unit tests of `wl`, `align`,
//! `cluster` and `candidates`.

use crate::facts::{EdgeKind, NodeKind, Pdg, PdgEdge, PdgNode, Receiver, SigFacts, TyShape};
use crate::shape::fixtures::{adt, by_ref, method, string};

pub fn node(kind: NodeKind) -> PdgNode {
    PdgNode {
        kind,
        ty: None,
        callee_id: None,
        callee_sig: None,
        macro_name: None,
        lit_kind: None,
        detail: String::new(),
        line: 1,
    }
}

pub fn edge(from: usize, to: usize, kind: EdgeKind, arg_pos: usize) -> PdgEdge {
    PdgEdge {
        from,
        to,
        kind,
        arg_pos,
    }
}

pub fn data(from: usize, to: usize, arg_pos: usize) -> PdgEdge {
    edge(from, to, EdgeKind::Data, arg_pos)
}

pub fn ctrl(from: usize, to: usize, arg_pos: usize) -> PdgEdge {
    edge(from, to, EdgeKind::Ctrl, arg_pos)
}

pub fn call(id: &str, sig: SigFacts) -> PdgNode {
    PdgNode {
        callee_id: Some(id.into()),
        callee_sig: Some(sig),
        ..node(NodeKind::Call)
    }
}

pub fn push_call() -> PdgNode {
    let sig = SigFacts {
        params: vec![TyShape::Param(1)],
        ret: TyShape::Tuple(vec![]),
        ..method("alloc::vec::Vec", Receiver::RefMut, vec![], string())
    };
    call("alloc::vec::Vec::push", sig)
}

pub fn getter_call(ty: &str, name: &str) -> PdgNode {
    let sig = method(&format!("k::{ty}"), Receiver::Ref, vec![], string());
    call(&format!("k::{ty}::{name}"), sig)
}

fn slice_of(ty: &str) -> TyShape {
    by_ref(TyShape::Slice(Box::new(adt(&format!("k::{ty}")))))
}

/// `for x in xs { out.push(x.method()) }` over `&[ty]`.
pub fn emit_loop(ty: &str, name: &str) -> Pdg {
    let param = |t: TyShape| PdgNode {
        ty: Some(t),
        ..node(NodeKind::Param)
    };
    let out_ty = TyShape::Ref {
        mutable: true,
        inner: Box::new(TyShape::Adt {
            id: "alloc::vec::Vec".into(),
            args: vec![string()],
        }),
    };
    let iterate = PdgNode {
        ty: Some(by_ref(adt(&format!("k::{ty}")))),
        ..node(NodeKind::Iterate)
    };
    Pdg {
        nodes: vec![
            param(slice_of(ty)),
            param(out_ty),
            iterate,
            push_call(),
            getter_call(ty, name),
            node(NodeKind::Return),
        ],
        edges: vec![
            data(0, 2, 0),
            data(1, 3, 0),
            data(2, 4, 0),
            data(4, 3, 1),
            ctrl(2, 3, 0),
            ctrl(2, 4, 0),
            data(2, 5, 0),
        ],
    }
}

/// Same graph with node indices rotated, to prove index independence.
pub fn permuted(pdg: &Pdg, shift: usize) -> Pdg {
    let n = pdg.nodes.len();
    let to_new = |i: usize| (i + shift) % n;
    let mut nodes = pdg.nodes.clone();
    for (old, node) in pdg.nodes.iter().enumerate() {
        nodes[to_new(old)] = node.clone();
    }
    let edges = pdg
        .edges
        .iter()
        .map(|e| edge(to_new(e.from), to_new(e.to), e.kind, e.arg_pos))
        .collect();
    Pdg { nodes, edges }
}
