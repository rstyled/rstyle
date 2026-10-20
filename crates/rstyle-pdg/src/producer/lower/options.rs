//! Option selection preserves the checked value and loads only on Some.
use super::*;

impl Builder<'_> {
    pub(super) fn option_match(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked =
            crate::checked_options::verify_match(node, self.facts).expect("verified Option match");
        let scrutinee = self.expression(checked.scrutinee, flow);
        let saved = scrutinee.value.clone().expect("checked Option value");
        let mut state = MatchState {
            remaining: scrutinee,
            completed: Vec::new(),
            controller: None,
        };

        for (index, arm_node) in node.children[1..].iter().enumerate() {
            state = self.option_arm(
                MatchArm {
                    node,
                    arm: arm_node,
                    saved: &saved,
                    index,
                    exhaustive: false,
                },
                &checked,
                state,
            );
        }

        self.match_join(
            node,
            state.controller.expect("Option selector"),
            state.completed,
        )
    }

    fn option_arm(
        &mut self,
        input: MatchArm<'_>,
        checked: &crate::checked_options::Match<'_>,
        state: MatchState,
    ) -> MatchState {
        let pattern = &input.arm.children[0];
        let operation = Operation {
            construct_span: input.node.span,
            role: format!("match_pattern:{}", input.index),
            ty: "bool".into(),
            construct: Some(
                if input.index == checked.some_arm {
                    "match.option-some"
                } else {
                    "match.option-none"
                }
                .into(),
            ),
            operands: vec![input.saved.into()],
            ..template(pattern, NodeCategory::Control)
        };
        let selected = self.emit(operation, state.remaining);
        let controller = state.controller.or_else(|| selected.value.clone());
        let remaining = if input.index == 0 {
            arm(&selected, "false")
        } else {
            empty_flow()
        };
        let matched = arm(&selected, "true");
        let matched = if input.index == checked.some_arm {
            self.option_payload(checked.binding, &input, matched)
        } else {
            matched
        };

        let body = self.expression(
            input.arm.children.last().expect("verified Option body"),
            matched,
        );
        MatchState {
            remaining,
            controller,
            completed: state
                .completed
                .into_iter()
                .chain([(input.index, body)])
                .collect(),
        }
    }

    fn option_payload(&mut self, binding: &HirNode, input: &MatchArm<'_>, flow: Flow) -> Flow {
        let read = Operation {
            construct_span: input.node.span,
            construct: Some("data.option-payload".into()),
            operands: vec![input.saved.into()],
            role: "option_payload_read".into(),
            ..template(binding, NodeCategory::Read)
        };
        let flow = self.emit(read, flow);
        let declaration = Operation {
            construct_span: input.node.span,
            ty: "unit".into(),
            symbol: binding.binding.clone(),
            write_form: Some("initialization".into()),
            role: "option_payload_binding".into(),
            operands: self.operand(&flow.value),
            ..template(binding, NodeCategory::Declaration)
        };
        self.emit(declaration, flow)
    }
}
