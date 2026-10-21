//! Evaluate the Option once, bind its scalar only on Some, then join normal flow.

use super::*;

impl Builder<'_> {
    pub(super) fn option_if_let(
        &mut self,
        node: &HirNode,
        checked: crate::checked_options::IfLet<'_>,
        flow: Flow,
    ) -> Flow {
        let selected = self.if_let_selection(&checked, flow);
        let controller = selected.flow.value.clone().expect("Option if-let selector");

        let arms = self.if_let_arms(&checked, selected);
        self.join(node, controller, arms)
    }

    fn if_let_selection(
        &mut self,
        checked: &crate::checked_options::IfLet<'_>,
        flow: Flow,
    ) -> Condition {
        let evaluated = self.expression(checked.scrutinee, flow);
        let saved = evaluated
            .value
            .clone()
            .expect("verified scalar Option initializer");

        let selector = Operation {
            construct_span: checked.source.span,
            construct: Some("if.option-some".into()),
            operands: vec![saved.clone()],
            role: "if_let_selector".into(),
            ty: "bool".into(),
            ..template(checked.condition, NodeCategory::Control)
        };
        let flow = self.emit(selector, evaluated);

        Condition {
            flow,
            value: Some(saved),
        }
    }

    fn if_let_arms(
        &mut self,
        checked: &crate::checked_options::IfLet<'_>,
        selected: Condition,
    ) -> Arms {
        let saved = selected.value.expect("saved scalar Option initializer");
        let payload = self.if_let_payload(checked, saved, arm(&selected.flow, "true"));
        let yes = self.expression(checked.then_branch, payload);
        let no = self.expression(checked.else_branch, arm(&selected.flow, "false"));

        Arms { yes, no }
    }

    fn if_let_payload(
        &mut self,
        checked: &crate::checked_options::IfLet<'_>,
        saved: String,
        flow: Flow,
    ) -> Flow {
        let binding = checked.binding;
        let node = checked.source;

        let read = Operation {
            construct_span: node.span,
            construct: Some("data.option-value-payload".into()),
            operands: vec![saved],
            role: "if_let_payload".into(),
            ..template(binding, NodeCategory::Computation)
        };
        let declaration = Operation {
            construct_span: node.span,
            ty: "unit".into(),
            symbol: binding.binding.clone(),
            write_form: Some("initialization".into()),
            role: "if_let_binding".into(),
            ..template(binding, NodeCategory::Declaration)
        };

        self.conditional_payload(read, declaration, flow)
    }
}
