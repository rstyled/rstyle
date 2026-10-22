//! Save the input Option once; None returns a fresh output-typed None.
use super::*;

impl Builder<'_> {
    pub(super) fn option_propagation(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::option_propagation::verify(node, self.facts)
            .expect("verified concrete Option propagation");
        let evaluated = self.expression(checked.operand, flow);

        self.option_branches(node, checked, evaluated)
    }

    fn option_branches(
        &mut self,
        node: &HirNode,
        checked: crate::option_propagation::Checked<'_>,
        evaluated: Flow,
    ) -> Flow {
        let saved = evaluated.value.clone().expect("saved input Option");
        let selector = self.emit(
            Operation {
                ty: "bool".into(),
                construct: Some("try.option-is-none".into()),
                operands: vec![saved.clone()],
                ..template(node, NodeCategory::Control)
            },
            evaluated,
        );

        self.option_none_exit(node, &checked, &selector);
        self.emit(
            Operation {
                ty: checked.payload_token,
                construct: Some("try.option-some-reference".into()),
                operands: vec![saved],
                ..template(node, NodeCategory::Computation)
            },
            arm(&selector, "false"),
        )
    }
    fn option_none_exit(
        &mut self,
        node: &HirNode,
        checked: &crate::option_propagation::Checked<'_>,
        selector: &Flow,
    ) {
        let failed = self.emit(
            Operation {
                ty: checked.output_token.clone(),
                construct: Some("try.option-none".into()),
                ..template(node, NodeCategory::Computation)
            },
            arm(selector, "true"),
        );
        self.return_operation(checked.early_return, "explicit", failed);
    }
}
