//! Immediate invocation preserves eager arguments and Some-only payload reads.
//! These operations require the registered immediate-callback capability.

use super::*;

struct Callback<'a> {
    node: &'a HirNode,
    checked: crate::option_callbacks::Invocation<'a>,
}

pub(super) struct SavedArguments {
    pub(super) flow: Flow,
    pub(super) receiver: String,
    pub(super) default: String,
}

impl Builder<'_> {
    pub(super) fn option_callback(
        &mut self,
        node: &HirNode,
        checked: crate::option_callbacks::Invocation<'_>,
        flow: Flow,
    ) -> Flow {
        let callback = Callback { node, checked };
        let arguments = self.callback_arguments(&callback.checked.arguments(), flow);
        let selected = self.callback_select(&callback, &arguments.receiver, arguments.flow);

        self.callback_result(
            &callback,
            SavedArguments {
                flow: selected,
                ..arguments
            },
        )
    }

    pub(super) fn callback_arguments(
        &mut self,
        callback: &crate::option_callbacks::Arguments<'_>,
        flow: Flow,
    ) -> SavedArguments {
        let receiver = self.expression(callback.receiver, flow);
        let saved_receiver = receiver.value.clone().expect("owned Option value");
        let default = self.expression(callback.default, receiver);
        let saved_default = default.value.clone().expect("eager bool default");

        SavedArguments {
            flow: default,
            receiver: saved_receiver,
            default: saved_default,
        }
    }

    fn callback_select(&mut self, callback: &Callback<'_>, saved: &str, flow: Flow) -> Flow {
        // This unit-valued operation records environment creation, not a
        // supported closure value. Shared byte storage reads no scalar content.
        let created = self.emit(
            Operation {
                construct_span: callback.node.span,
                construct: Some("callback.shared-byte-environment".into()),
                role: "callback_environment".into(),
                ty: "unit".into(),
                ..template(callback.checked.callback, NodeCategory::Computation)
            },
            flow,
        );

        self.emit(
            Operation {
                construct: Some("callback.option-some".into()),
                role: "callback_selector".into(),
                operands: vec![saved.into()],
                ty: "bool".into(),
                ..template(callback.node, NodeCategory::Control)
            },
            created,
        )
    }

    fn callback_result(&mut self, callback: &Callback<'_>, arguments: SavedArguments) -> Flow {
        let selected = arguments.flow;
        let controller = selected.value.clone().expect("Option selector");
        let payload = self.callback_payload(callback, &arguments.receiver, arm(&selected, "true"));
        let yes = self.expression(callback.checked.predicate.body, payload);
        let no = Flow {
            value: Some(arguments.default),
            ..arm(&selected, "false")
        };

        self.join(callback.node, controller, Arms { yes, no })
    }

    fn callback_payload(&mut self, callback: &Callback<'_>, saved: &str, flow: Flow) -> Flow {
        let binding = callback.checked.predicate.binding;
        let read = Operation {
            construct_span: callback.node.span,
            construct: Some("data.option-value-payload".into()),
            operands: vec![saved.into()],
            role: "callback_payload_read".into(),
            ..template(binding, NodeCategory::Computation)
        };
        let declaration = Operation {
            construct_span: callback.node.span,
            ty: "unit".into(),
            symbol: binding.binding.clone(),
            write_form: Some("initialization".into()),
            role: "callback_payload_binding".into(),
            ..template(binding, NodeCategory::Declaration)
        };

        self.conditional_payload(read, declaration, flow)
    }
}
