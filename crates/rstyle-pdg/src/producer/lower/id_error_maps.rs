//! Save receiver and immutable environment once; consume the owned error on Err only.
use super::*;
use crate::id_error_maps::{self as contract, Checked};

struct Saved {
    input: String,
    environment: String,
    selector: Flow,
}

impl Builder<'_> {
    pub(super) fn id_error_map(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = contract::verify(node, self.facts).expect("verified immediate ID error map");

        let saved = self.save_id_map(&checked, flow);

        self.id_map_arms(node, &checked, saved)
    }

    fn id_map_arms(&mut self, node: &HirNode, checked: &Checked<'_>, saved: Saved) -> Flow {
        let yes = self.id_map_error(checked, &saved);

        let no = self.id_map_success(checked, &saved);

        let controller = saved.selector.value.expect("saved ID map discriminator");

        self.join(node, controller, Arms { yes, no })
    }

    fn save_id_map(&mut self, checked: &Checked<'_>, flow: Flow) -> Saved {
        let receiver = self.expression(checked.receiver, flow);

        let input = receiver.value.clone().expect("saved decoder Result");

        let environment = self.emit(
            Operation {
                construct_span: checked.source.span,
                construct: Some(contract::ENVIRONMENT.into()),
                role: "id_map_environment".into(),
                ty: crate::data_types::token(&checked.storage).expect("checked callback storage"),
                symbol: Some(checked.message.symbol.clone()),
                ..template(checked.closure, NodeCategory::Read)
            },
            receiver,
        );

        let saved_environment = environment
            .value
            .clone()
            .expect("saved immutable environment");

        let selector = self.emit(
            Operation {
                construct: Some(contract::SELECTOR.into()),
                operands: vec![input.clone()],
                ty: "bool".into(),
                role: "id_map_selector".into(),
                ..template(checked.source, NodeCategory::Control)
            },
            environment,
        );

        Saved {
            input,
            environment: saved_environment,
            selector,
        }
    }

    fn id_map_error(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let error = self.id_map_owned_error(checked, saved);

        let constructed = self.id_map_constructor(checked, saved, error);

        self.id_map_wrap(checked, contract::ERROR, constructed)
    }

    fn id_map_owned_error(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let bound = self.id_map_binding(checked, saved);

        self.expression(checked.error_read, bound)
    }

    fn id_map_constructor(&mut self, checked: &Checked<'_>, saved: &Saved, error: Flow) -> Flow {
        let saved_error = error.value.clone().expect("one owned error read");

        let message = self.emit(
            Operation {
                construct: Some(contract::MESSAGE.into()),
                operands: vec![saved.environment.clone()],
                role: "id_map_capture_read".into(),
                ..template(checked.message_read, NodeCategory::Computation)
            },
            error,
        );

        let arguments = vec![
            saved_error,
            message.value.clone().expect("saved captured message"),
        ];

        self.emit(
            Operation {
                operator: crate::helper_calls::callee(checked.constructor, self.facts),
                construct: self.computation_construct(checked.constructor),
                operands: arguments,
                ..template(checked.constructor, NodeCategory::Call)
            },
            message,
        )
    }

    fn id_map_binding(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let payload = Operation {
            construct_span: checked.source.span,
            construct: Some(contract::PAYLOAD.into()),
            role: "id_map_payload".into(),
            operands: vec![saved.input.clone()],
            ..template(checked.binding, NodeCategory::Computation)
        };

        let declaration = Operation {
            construct_span: checked.source.span,
            construct: Some(contract::BINDING.into()),
            role: "id_map_binding".into(),
            ty: "unit".into(),
            symbol: checked.binding.binding.clone(),
            write_form: Some("initialization".into()),
            ..template(checked.binding, NodeCategory::Declaration)
        };

        self.conditional_payload(payload, declaration, arm(&saved.selector, "true"))
    }

    fn id_map_success(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let success = self.emit(
            Operation {
                construct: Some(contract::SUCCESS.into()),
                role: "id_map_success".into(),
                ty: crate::data_types::token(checked.success()).expect("checked Copy ID"),
                operands: vec![saved.input.clone()],
                ..template(checked.source, NodeCategory::Computation)
            },
            arm(&saved.selector, "false"),
        );

        self.id_map_wrap(checked, contract::OK, success)
    }

    fn id_map_wrap(&mut self, checked: &Checked<'_>, construct: &str, flow: Flow) -> Flow {
        self.emit(
            Operation {
                construct: Some(construct.into()),
                role: "id_map_wrap".into(),
                ty: crate::data_types::token(&checked.output).expect("checked mapped Result"),
                operands: self.operand(&flow.value),
                ..template(checked.source, NodeCategory::Computation)
            },
            flow,
        )
    }
}
