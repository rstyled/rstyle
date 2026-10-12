//! Source loop transfers; divergence never creates executable fallthrough.

use super::*;

pub(super) struct LoopFrame {
    header: String,
    breaks: Vec<End>,
}

impl Builder<'_> {
    pub(super) fn while_loop(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let header = format!("n{}", self.nodes.len());
        let predicate = self.while_predicate(node, flow);
        let literal = node.children[0].lit.as_deref();
        let incoming = if literal == Some("false") {
            empty_flow()
        } else {
            arm(&predicate, "true")
        };
        let exits = self.loop_body(&node.children[1], incoming, &header);
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
        let ends = self.loop_body(&node.children[0], entered, &header);

        Flow {
            normal: !ends.is_empty(),
            ends,
            value: None,
        }
    }

    fn while_predicate(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let condition = self.expression(&node.children[0], flow);
        let operation = Operation {
            role: "while_predicate".into(),
            ty: "bool".into(),
            construct: Some("while".into()),
            operands: self.operand(&condition.value),
            ..template(node, NodeCategory::Control)
        };

        self.emit(operation, condition)
    }

    fn loop_body(&mut self, node: &HirNode, flow: Flow, header: &str) -> Vec<End> {
        self.enter_loop(header);
        let body = self.expression(node, flow);
        self.finish_loop(&body, header)
    }

    fn finish_loop(&mut self, body: &Flow, header: &str) -> Vec<End> {
        self.back_edges(body, header);

        self.leave_loop()
    }

    fn enter_loop(&mut self, header: &str) {
        self.loops.push(LoopFrame {
            header: header.into(),
            breaks: Vec::new(),
        });
    }

    fn leave_loop(&mut self) -> Vec<End> {
        self.loops.pop().expect("loop frame").breaks
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
