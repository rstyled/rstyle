//! Verify the dedicated payload before lowering shared Result branch topology.
use super::result_propagation::RewrappedResult;
use super::*;

impl Builder<'_> {
    pub(super) fn named_error_result_propagation(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::named_error_result_propagation::verify(node, self.facts)
            .expect("verified named error reader Result propagation");
        let evaluated = self.expression(checked.operand, flow);

        self.rewrapped_result(
            RewrappedResult {
                source: node,
                early_return: checked.early_return,
                output_type: checked.output_token,
                payload_type: checked.payload_token,
                constructs: [
                    "try.reader-result-is-err",
                    "try.reader-result-error",
                    "try.reader-result-ok-unit",
                ],
            },
            evaluated,
        )
    }
}
