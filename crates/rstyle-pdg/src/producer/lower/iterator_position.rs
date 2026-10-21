//! Fresh iterator and environment creation precede conditional prefix traversal.
use super::*;
use crate::iterator_position as contract;

impl Builder<'_> {
    pub(super) fn byte_position(&mut self, checked: contract::Checked<'_>, flow: Flow) -> Flow {
        let evaluated = self.expression(checked.receiver, flow);
        let receiver = evaluated.value.clone().expect("verified shared byte view");
        let selected = self.position_start(&checked, receiver.clone(), evaluated);

        self.position_finish(&checked, receiver, selected)
    }

    fn position_start(
        &mut self,
        checked: &contract::Checked<'_>,
        receiver: String,
        evaluated: Flow,
    ) -> Flow {
        let created = self.emit(
            Operation {
                construct: Some("iterator.fresh-shared-bytes".into()),
                ty: "unit".into(),
                operands: vec![receiver.clone()],
                ..template(checked.iterator, NodeCategory::Computation)
            },
            evaluated,
        );
        let environment = self.emit(
            Operation {
                construct: Some("iterator.shared-byte-environment".into()),
                ty: "unit".into(),
                ..template(checked.callback, NodeCategory::Computation)
            },
            created,
        );
        self.emit(
            Operation {
                construct: Some("iterator.nonempty".into()),
                ty: "bool".into(),
                operands: vec![receiver.clone()],
                ..template(checked.call, NodeCategory::Control)
            },
            environment,
        )
    }

    fn position_finish(
        &mut self,
        checked: &contract::Checked<'_>,
        receiver: String,
        selected: Flow,
    ) -> Flow {
        let controller = selected.value.clone().expect("nonempty selector");
        let yes = self.position_prefix(checked, receiver, arm(&selected, "true"));
        let no = self.emit(
            Operation {
                construct: Some("iterator.empty-position".into()),
                ty: contract::result_type(),
                ..template(&checked.callback.children[0], NodeCategory::Computation)
            },
            arm(&selected, "false"),
        );
        self.join(checked.call, controller, Arms { yes, no })
    }
    fn position_prefix(
        &mut self,
        checked: &contract::Checked<'_>,
        receiver: String,
        flow: Flow,
    ) -> Flow {
        let captured = self.read(checked.predicate.captured_value, flow);
        let value = captured.value.clone().expect("verified byte capture");

        self.emit(
            Operation {
                construct: Some(contract::PREFIX.into()),
                original_kind: "rstyle.iterator-prefix".into(),
                operands: vec![receiver, value],
                ty: contract::result_type(),
                ..template(checked.call, NodeCategory::Read)
            },
            captured,
        )
    }
}
