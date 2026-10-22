//! Save the input once, inspect its tag, and rewrap only the selected error.
use super::*;

impl Builder<'_> {
    pub(super) fn borrowed_result_propagation(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::borrowed_result_propagation::verify(node, self.facts)
            .expect("verified borrowed Result propagation");
        let evaluated = self.expression(checked.operand, flow);
        self.borrowed_result_branches(node, checked, evaluated)
    }

    fn borrowed_result_branches(
        &mut self,
        node: &HirNode,
        checked: crate::borrowed_result_propagation::Checked<'_>,
        evaluated: Flow,
    ) -> Flow {
        let branch = self.borrowed_result_error(node, &checked, evaluated);

        self.emit(
            Operation {
                ty: checked.payload_token,
                construct: Some("try.borrowed-result-ok-reference".into()),
                operands: vec![branch.input],
                ..template(node, NodeCategory::Computation)
            },
            arm(&branch.selector, "false"),
        )
    }

    fn borrowed_result_error(
        &mut self,
        node: &HirNode,
        checked: &crate::borrowed_result_propagation::Checked<'_>,
        evaluated: Flow,
    ) -> SavedResult {
        let saved = evaluated.value.clone().expect("saved input Result");
        let selector = self.emit(
            Operation {
                ty: "bool".into(),
                construct: Some("try.borrowed-result-is-err".into()),
                operands: vec![saved.clone()],
                ..template(node, NodeCategory::Control)
            },
            evaluated,
        );
        let error = self.emit(
            Operation {
                ty: checked.output_token.clone(),
                construct: Some("try.borrowed-result-error".into()),
                operands: vec![saved.clone()],
                ..template(node, NodeCategory::Computation)
            },
            arm(&selector, "true"),
        );
        self.return_operation(checked.early_return, "explicit", error);

        SavedResult {
            selector,
            input: saved,
        }
    }
}

struct SavedResult {
    selector: Flow,
    input: String,
}
