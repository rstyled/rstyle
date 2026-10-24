//! Immediate invocation preserves eager arguments and Some-only payload reads.
//! These operations require the registered immediate-callback capability.

use super::*;

pub(super) enum Target<'a> {
    Byte(crate::option_callbacks::Invocation<'a>),
    Helper(crate::helper_option_callbacks::Checked<'a>),
    Named(crate::option_callbacks::Arguments<'a>),
}

struct Body<'a> {
    call: &'a HirNode,
    binding: &'a HirNode,
    expression: &'a HirNode,
    borrowed: bool,
}

impl<'a> Target<'a> {
    fn arguments(&self) -> crate::option_callbacks::Arguments<'a> {
        match self {
            Self::Byte(checked) => checked.arguments(),
            Self::Helper(checked) => checked.arguments,
            Self::Named(checked) => *checked,
        }
    }
}

pub(super) struct SavedArguments {
    pub(super) flow: Flow,
    pub(super) receiver: String,
    pub(super) default: String,
}

impl Builder<'_> {
    pub(super) fn callback_invocation(&mut self, target: Target<'_>, flow: Flow) -> Flow {
        let arguments = self.callback_arguments(&target.arguments(), flow);
        let selected = self.select_callback(&target, &arguments.receiver, arguments.flow);

        self.finish_callback(
            target,
            SavedArguments {
                flow: selected,
                ..arguments
            },
        )
    }

    fn select_callback(&mut self, target: &Target<'_>, saved: &str, flow: Flow) -> Flow {
        match target {
            Target::Byte(checked) => self.callback_select(checked, saved, flow),
            Target::Helper(checked) => self.helper_callback_select(checked, saved, flow),
            Target::Named(checked) => self.named_select(checked, saved, flow),
        }
    }

    fn finish_callback(&mut self, target: Target<'_>, saved: SavedArguments) -> Flow {
        let body = match target {
            Target::Byte(checked) => Body {
                call: checked.call,
                binding: checked.predicate.binding,
                expression: checked.predicate.body,
                borrowed: false,
            },
            Target::Helper(checked) => Body {
                call: checked.arguments.call,
                binding: checked.binding,
                expression: checked.body,
                borrowed: checked.borrowed,
            },
            Target::Named(checked) => return self.named_result(&checked, saved),
        };

        self.closure_result(&body, saved)
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

    fn callback_select(
        &mut self,
        callback: &crate::option_callbacks::Invocation<'_>,
        saved: &str,
        flow: Flow,
    ) -> Flow {
        // This unit-valued operation records environment creation, not a
        // supported closure value. Shared byte storage reads no scalar content.
        let created = self.emit(
            Operation {
                construct_span: callback.call.span,
                construct: Some("callback.shared-byte-environment".into()),
                role: "callback_environment".into(),
                ty: "unit".into(),
                ..template(callback.callback, NodeCategory::Computation)
            },
            flow,
        );

        self.emit(
            Operation {
                construct: Some("callback.option-some".into()),
                role: "callback_selector".into(),
                operands: vec![saved.into()],
                ty: "bool".into(),
                ..template(callback.call, NodeCategory::Control)
            },
            created,
        )
    }

    fn closure_result(&mut self, body: &Body<'_>, arguments: SavedArguments) -> Flow {
        let selected = arguments.flow;
        let controller = selected.value.clone().expect("Option selector");
        let payload = self.callback_payload(body, &arguments.receiver, arm(&selected, "true"));
        let yes = self.expression(body.expression, payload);
        let no = Flow {
            value: Some(arguments.default),
            ..arm(&selected, "false")
        };

        self.join(body.call, controller, Arms { yes, no })
    }

    fn callback_payload(&mut self, body: &Body<'_>, saved: &str, flow: Flow) -> Flow {
        let binding = body.binding;
        let (construct, category) = if body.borrowed {
            ("data.option-payload", NodeCategory::Read)
        } else {
            ("data.option-value-payload", NodeCategory::Computation)
        };
        let read = Operation {
            construct_span: body.call.span,
            construct: Some(construct.into()),
            operands: vec![saved.into()],
            role: "callback_payload_read".into(),
            ..template(binding, category)
        };
        let declaration = Operation {
            construct_span: body.call.span,
            ty: "unit".into(),
            symbol: binding.binding.clone(),
            write_form: Some("initialization".into()),
            role: "callback_payload_binding".into(),
            ..template(binding, NodeCategory::Declaration)
        };

        self.conditional_payload(read, declaration, flow)
    }
}
