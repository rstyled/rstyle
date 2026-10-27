//! Evaluate once, then initialize scalar locals from saved tuple positions.
use super::*;

impl Builder<'_> {
    pub(super) fn scalar_tuple_binding(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::scalar_tuple_bindings::verify(node, self.facts)
            .expect("independently checked scalar tuple binding");
        let flow = self.expression(checked.initializer, flow);
        if !flow.normal {
            return flow;
        }
        let saved = flow.value.clone().expect("saved scalar tuple initializer");

        checked.entries.iter().fold(flow, |flow, entry| {
            self.scalar_tuple_component(entry, &saved, flow)
        })
    }

    fn scalar_tuple_component(
        &mut self,
        entry: &crate::scalar_tuple_bindings::Entry<'_>,
        saved: &str,
        flow: Flow,
    ) -> Flow {
        let flow = self.emit(
            Operation {
                construct: Some(crate::scalar_tuple_bindings::SELECTOR.into()),
                literal: Some(entry.position.to_string()),
                operands: vec![saved.into()],
                ty: entry.token.into(),
                ..template(entry.pattern, NodeCategory::Computation)
            },
            flow,
        );
        let declaration = Operation {
            construct: Some(crate::scalar_tuple_bindings::DECLARATION.into()),
            ty: "unit".into(),
            symbol: Some(entry.binding.symbol.clone()),
            write_form: Some("initialization".into()),
            operands: self.operand(&flow.value),
            ..template(entry.pattern, NodeCategory::Declaration)
        };
        self.emit(declaration, flow)
    }
}
