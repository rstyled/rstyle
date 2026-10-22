//! Exact same-error propagation forwards the saved whole Err value once.
use super::*;

struct ResultBranches<'a> {
    source: &'a HirNode,
    early_return: &'a HirNode,
    saved: String,
}

impl Builder<'_> {
    pub(super) fn result_propagation(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::result_propagation::verify(node, self.facts)
            .expect("verified same-error Result propagation");
        let evaluated = self.expression(checked.operand, flow);
        let saved = evaluated.value.clone().expect("saved Result operand");
        let selector = self.emit(
            Operation {
                ty: "bool".into(),
                construct: Some("try.result-is-err".into()),
                operands: vec![saved.clone()],
                ..template(node, NodeCategory::Control)
            },
            evaluated,
        );

        self.result_continuations(
            ResultBranches {
                source: node,
                early_return: checked.early_return,
                saved,
            },
            selector,
        )
    }

    fn result_continuations(&mut self, branches: ResultBranches<'_>, selector: Flow) -> Flow {
        let ResultBranches {
            source,
            early_return,
            saved,
        } = branches;
        let failed = Flow {
            value: Some(saved.clone()),
            ..arm(&selector, "true")
        };
        self.return_operation(early_return, "explicit", failed);
        self.emit(
            Operation {
                ty: "unit".into(),
                construct: Some("try.result-ok-unit".into()),
                literal: Some("()".into()),
                operands: vec![saved],
                ..template(source, NodeCategory::Computation)
            },
            arm(&selector, "false"),
        )
    }
}
