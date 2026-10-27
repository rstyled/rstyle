//! Function-item materialization is eager; invocation occurs only on Some.
use super::callbacks::SavedArguments;
use super::*;
use crate::{named_callbacks as contract, option_callbacks::Arguments};

impl Builder<'_> {
    pub(super) fn named_result(
        &mut self,
        checked: &Arguments<'_>,
        arguments: SavedArguments,
    ) -> Flow {
        let selected = arguments.flow;
        let controller = selected.value.clone().expect("Option selector");
        let yes = self.named_invoke(checked, arguments.receiver, arm(&selected, "true"));
        let no = Flow {
            value: Some(arguments.default),
            ..arm(&selected, "false")
        };

        self.join(checked.call, controller, Arms { yes, no })
    }

    pub(super) fn named_select(
        &mut self,
        checked: &Arguments<'_>,
        receiver: &str,
        flow: Flow,
    ) -> Flow {
        let item = self.emit(
            Operation {
                construct: Some(contract::ITEM.into()),
                ty: "unit".into(),
                role: "function_item".into(),
                ..template(checked.callback, NodeCategory::Computation)
            },
            flow,
        );
        self.emit(
            Operation {
                construct: Some(contract::SELECTOR.into()),
                operands: vec![receiver.into()],
                ty: "bool".into(),
                role: "named_callback_selector".into(),
                ..template(checked.call, NodeCategory::Control)
            },
            item,
        )
    }

    fn named_invoke(&mut self, checked: &Arguments<'_>, receiver: String, flow: Flow) -> Flow {
        let payload = self.emit(
            Operation {
                construct: Some(contract::PAYLOAD.into()),
                operands: vec![receiver],
                ty: "u8".into(),
                role: "named_callback_payload".into(),
                ..template(checked.callback, NodeCategory::Computation)
            },
            flow,
        );
        let saved = payload.value.clone().expect("Some payload");
        self.emit(
            Operation {
                construct: Some(contract::INVOKE.into()),
                operands: vec![saved],
                operator: checked
                    .callback
                    .dispatch
                    .as_ref()
                    .map(|dispatch| dispatch.op.clone()),
                ty: "bool".into(),
                role: "named_callback_invocation".into(),
                ..template(checked.callback, NodeCategory::Call)
            },
            payload,
        )
    }
}
