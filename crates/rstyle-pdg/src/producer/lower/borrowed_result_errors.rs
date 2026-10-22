//! Preserve eager receiver/error evaluation and independently proved fresh errors.
use super::*;

impl Builder<'_> {
    pub(super) fn fresh_error_variant(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::borrowed_result_errors::verify_error(node, self.facts)
            .expect("verified fresh no-drop error variant");

        self.emit(
            Operation {
                ty: crate::data_aggregates::token(node)
                    .expect("verified error token")
                    .into(),
                literal: Some(checked.variant),
                construct: Some(crate::borrowed_result_errors::ERROR.into()),
                ..template(node, NodeCategory::Computation)
            },
            flow,
        )
    }

    pub(super) fn borrowed_option_ok_or(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::borrowed_result_errors::verify(node, self.facts)
            .expect("verified eager Option conversion");
        let receiver = self.expression(checked.receiver, flow);
        let saved = receiver.value.clone();
        let evaluated = self.expression(checked.error, receiver);
        let operands = saved.into_iter().chain(evaluated.value.clone()).collect();
        let summary = crate::borrowed_result_errors::resolve(node, self.facts)
            .expect("verified eager Option conversion summary");

        self.emit(
            Operation {
                operator: Some(summary.callee_path),
                construct: Some(crate::borrowed_result_errors::CONSTRUCT.into()),
                operands,
                ..template(node, NodeCategory::Call)
            },
            evaluated,
        )
    }
}
