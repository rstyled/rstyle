//! Prefix range syntax is a carrier; save receiver before evaluating its end.
use super::*;

impl Builder<'_> {
    pub(super) fn checked_prefix_get(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::checked_slice_views::verify(node, self.facts)
            .expect("verified checked prefix get");
        let receiver = self.expression(checked.receiver, flow);
        let saved = receiver.value.clone();
        let evaluated = self.expression(checked.end, receiver);
        let operands = saved.into_iter().chain(evaluated.value.clone()).collect();

        self.emit(
            Operation {
                operator: Some("core::slice::<impl [T]>::get".into()),
                construct: Some(crate::checked_slice_views::CONSTRUCT.into()),
                operands,
                ..template(node, NodeCategory::Call)
            },
            evaluated,
        )
    }
}
