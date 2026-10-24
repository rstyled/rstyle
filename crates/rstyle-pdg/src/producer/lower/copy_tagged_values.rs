//! Save the original Copy input once and retain its wildcard complement.
use super::*;
use crate::nominal_boundary::resolution::CheckedCopyConversion;

impl Builder<'_> {
    pub(super) fn copy_tagged_value(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let resolution = self
            .facts
            .nominal_resolution()
            .expect("checked conversion source");
        let receipt = resolution
            .copy_conversion_source(self.facts.facts())
            .expect("bound conversion source")
            .expect("checked original conversion");

        if receipt.error().is_none() {
            self.copy_tagged_constructor(node, flow, receipt)
        } else {
            self.copy_tagged_match(node, flow, receipt)
        }
    }

    pub(super) fn copy_tagged_constructor(
        &mut self,
        node: &HirNode,
        flow: Flow,
        checked: &CheckedCopyConversion,
    ) -> Flow {
        let tag = crate::copy_tagged_values::constructor(node, self.facts, checked)
            .expect("verified Copy conversion constructor");
        let operation = Operation {
            construct: Some("data.enum-value".into()),
            operator: Some(tag),
            ..template(node, NodeCategory::Computation)
        };
        self.constructed_value(node, operation, flow)
    }

    pub(super) fn copy_tagged_match(
        &mut self,
        node: &HirNode,
        flow: Flow,
        receipt: &CheckedCopyConversion,
    ) -> Flow {
        let checked = crate::copy_tagged_values::matched(node, self.facts, receipt)
            .expect("verified Copy conversion inverse");
        let scrutinee = self.expression(checked.scrutinee, flow);
        let (controller, completed) = self.copy_tagged_arms(node, &checked, scrutinee);

        self.match_join(node, controller, completed)
    }

    fn copy_tagged_arms(
        &mut self,
        node: &HirNode,
        checked: &crate::copy_tagged_values::Match<'_>,
        scrutinee: Flow,
    ) -> (String, Vec<(usize, Flow)>) {
        let saved = scrutinee.value.clone().expect("checked saved tagged input");
        let input = MatchArm {
            node,
            arm: &node.children[1],
            saved: &saved,
            index: 0,
            exhaustive: false,
        };
        let selected = self.copy_tagged_selector(&input, checked, scrutinee);
        let controller = selected.value.clone().expect("checked tagged selector");
        let success = self.copy_tagged_success(&input, checked, arm(&selected, "true"));
        let fallback = MatchArm {
            arm: &node.children[2],
            index: 1,
            exhaustive: true,
            ..input
        };
        let failure = self.copy_tagged_failure(&fallback, checked, arm(&selected, "false"));

        (controller, vec![(0, success), (1, failure)])
    }

    fn copy_tagged_success(
        &mut self,
        input: &MatchArm<'_>,
        checked: &crate::copy_tagged_values::Match<'_>,
        flow: Flow,
    ) -> Flow {
        let payload = self.copy_tagged_payload(input, checked, flow);
        self.expression(checked.success, payload)
    }

    fn copy_tagged_failure(
        &mut self,
        input: &MatchArm<'_>,
        checked: &crate::copy_tagged_values::Match<'_>,
        flow: Flow,
    ) -> Flow {
        let wildcard = self.emit(self.match_selector(input), flow);
        self.expression(checked.failure, wildcard)
    }

    fn copy_tagged_selector(
        &mut self,
        input: &MatchArm<'_>,
        checked: &crate::copy_tagged_values::Match<'_>,
        flow: Flow,
    ) -> Flow {
        let operation = Operation {
            construct_span: input.node.span,
            role: "match_pattern:0".into(),
            ty: "bool".into(),
            construct: Some("match.enum-variant".into()),
            operator: Some(checked.tag.clone()),
            operands: vec![input.saved.into()],
            ..template(checked.pattern, NodeCategory::Control)
        };
        self.emit(operation, flow)
    }

    fn copy_tagged_payload(
        &mut self,
        input: &MatchArm<'_>,
        checked: &crate::copy_tagged_values::Match<'_>,
        flow: Flow,
    ) -> Flow {
        let binding = checked.binding;
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
