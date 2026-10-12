//! Source loop transfers; divergence never creates executable fallthrough.

use super::*;

pub(super) struct LoopFrame {
    header: String,
    breaks: Vec<End>,
}

impl Builder<'_> {
    pub(super) fn while_loop(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let header = format!("n{}", self.nodes.len());
        let condition = self.expression(&node.children[0], flow);
        let operation = Operation {
            role: "while_predicate".into(),
            ty: "bool".into(),
            construct: Some("while".into()),
            operands: self.operand(&condition.value),
            ..template(node, NodeCategory::Control)
        };
        let predicate = self.emit(operation, condition);
        self.loops.push(LoopFrame {
            header: header.clone(),
            breaks: Vec::new(),
        });
        let literal = node.children[0].lit.as_deref();
        let incoming = if literal == Some("false") {
            empty_flow()
        } else {
            arm(&predicate, "true")
        };
        let body = self.expression(&node.children[1], incoming);
        self.back_edges(&body, &header);
        let exits = self.loops.pop().expect("loop frame").breaks;
        let mut result = if literal == Some("true") {
            empty_flow()
        } else {
            arm(&predicate, "false")
        };
        result.normal |= !exits.is_empty();
        result.ends.extend(exits);
        result.value = None;

        result
    }

    pub(super) fn infinite_loop(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let operation = Operation {
            role: "loop_header".into(),
            ty: "bool".into(),
            construct: Some("loop".into()),
            ..template(node, NodeCategory::Control)
        };
        let entered = self.emit(operation, flow);
        let header = entered.value.clone().expect("loop header");
        self.loops.push(LoopFrame {
            header: header.clone(),
            breaks: Vec::new(),
        });
        let body = self.expression(&node.children[0], entered);
        self.back_edges(&body, &header);
        let ends = self.loops.pop().expect("loop frame").breaks;

        Flow {
            normal: !ends.is_empty(),
            ends,
            value: None,
        }
    }

    fn back_edges(&mut self, flow: &Flow, header: &str) {
        if !flow.normal {
            return;
        }
        for end in &flow.ends {
            self.transfers.push(Transfer {
                source: end.node.clone(),
                target: header.into(),
                kind: end.kind.clone(),
                loop_role: Some("back_edge".into()),
            });
        }
    }

    pub(super) fn loop_exit(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let construct = if node.kind == "expr_break" {
            "break"
        } else {
            "continue"
        };
        let operation = Operation {
            role: "loop_exit_transfer".into(),
            ty: "unit".into(),
            construct: Some(construct.into()),
            ..template(node, NodeCategory::Control)
        };
        let emitted = self.emit(operation, flow);
        let frame = self.loops.last_mut().expect("resolved loop exit");
        if construct == "break" {
            frame.breaks.extend(emitted.ends.into_iter().map(|end| End {
                kind: "break".into(),
                ..end
            }));
        } else {
            self.transfers.push(Transfer {
                source: emitted.value.expect("continue operation"),
                target: frame.header.clone(),
                kind: "continue".into(),
                loop_role: Some("back_edge".into()),
            });
        }
        empty_flow()
    }
}
