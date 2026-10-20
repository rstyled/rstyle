//! Saved tagged values select exactly one conditional scalar payload.
use super::*;

impl Builder<'_> {
    pub(super) fn enum_match(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::data_enums::matched(node, self.facts).expect("verified enum match");
        let scrutinee = self.expression(checked.scrutinee, flow);
        let saved = scrutinee.value.clone().expect("checked enum value");
        let mut state = MatchState {
            remaining: scrutinee,
            completed: Vec::new(),
            controller: None,
        };

        for (index, arm) in checked.arms.iter().enumerate() {
            let input = MatchArm {
                node,
                arm: &node.children[index + 1],
                saved: &saved,
                index,
                exhaustive: index + 1 == checked.arms.len(),
            };
            state = self.enum_arm(&input, arm, state);
        }
        self.match_join(
            node,
            state.controller.expect("enum selector"),
            state.completed,
        )
    }

    fn enum_arm(
        &mut self,
        input: &MatchArm<'_>,
        checked: &crate::data_enums::Arm<'_>,
        state: MatchState,
    ) -> MatchState {
        let selected = self.emit(
            Operation {
                construct_span: input.node.span,
                role: format!("match_pattern:{}", input.index),
                ty: "bool".into(),
                construct: Some("match.enum-variant".into()),
                operator: Some(checked.tag.clone()),
                operands: vec![input.saved.into()],
                ..template(checked.pattern, NodeCategory::Control)
            },
            state.remaining,
        );
        let controller = state.controller.or_else(|| selected.value.clone());
        let remaining = if input.exhaustive {
            empty_flow()
        } else {
            arm(&selected, "false")
        };
        let matched = arm(&selected, "true");
        let matched = match checked.binding {
            Some(_) => self.enum_payload(input, checked, matched),
            None => matched,
        };
        let body = self.expression(
            input.arm.children.last().expect("verified enum body"),
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

    fn enum_payload(
        &mut self,
        input: &MatchArm<'_>,
        checked: &crate::data_enums::Arm<'_>,
        flow: Flow,
    ) -> Flow {
        let binding = checked.binding.expect("verified enum payload binding");
        let read = Operation {
            construct_span: input.node.span,
            construct: Some("data.enum-payload".into()),
            operator: Some(checked.tag.clone()),
            operands: vec![input.saved.into()],
            role: "enum_payload_read".into(),
            ..template(binding, NodeCategory::Computation)
        };
        let declaration = Operation {
            construct_span: input.node.span,
            ty: "unit".into(),
            symbol: binding.binding.clone(),
            write_form: Some("initialization".into()),
            role: "enum_payload_binding".into(),
            ..template(binding, NodeCategory::Declaration)
        };
        self.conditional_payload(read, declaration, flow)
    }
}
