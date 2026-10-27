//! Explicit guarded reader continuations preserve saved values and opaque errors.
use super::*;
use crate::widening_readers::*;

struct Saved {
    input: String,
    environment: String,
    selector: Flow,
}

impl Builder<'_> {
    pub(super) fn widening_reader(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::widening_readers::verify(self.facts)
            .expect("verified complete u32 widening reader");
        let saved = self.widening_outer_save(&checked, flow);
        self.widening_outer_arms(node, &checked, saved)
    }

    fn widening_outer_arms(&mut self, node: &HirNode, checked: &Checked<'_>, saved: Saved) -> Flow {
        let yes = self.widening_continuation(checked, &saved);
        let no = self.emit(
            Operation {
                construct: Some(READER_ERROR.into()),
                role: "widening_reader_error".into(),
                operands: vec![saved.input.clone()],
                ty: token(&checked.proof.output),
                ..template(checked.roles.source, NodeCategory::Computation)
            },
            arm(&saved.selector, "false"),
        );
        self.join(
            node,
            saved.selector.value.expect("reader selector"),
            Arms { yes, no },
        )
    }

    fn widening_outer_save(&mut self, checked: &Checked<'_>, flow: Flow) -> Saved {
        let receiver = self.expression(checked.roles.reader, flow);
        let input = receiver
            .value
            .clone()
            .expect("saved named-error reader Result");
        let created = self.emit(
            Operation {
                construct_span: checked.roles.source.span,
                construct: Some(OUTER_ENVIRONMENT.into()),
                role: "widening_outer_environment".into(),
                ty: token(&checked.proof.storage[0]),
                symbol: Some(checked.roles.message.symbol.clone()),
                ..template(checked.roles.continuation, NodeCategory::Read)
            },
            receiver,
        );
        let environment = created
            .value
            .clone()
            .expect("saved outer callback environment");

        let selector = self.emit(
            Operation {
                construct: Some(OUTER_SELECTOR.into()),
                role: "widening_outer_selector".into(),
                ty: "bool".into(),
                operands: vec![input.clone()],
                ..template(checked.roles.source, NodeCategory::Control)
            },
            created,
        );
        Saved {
            input,
            environment,
            selector,
        }
    }

    fn widening_continuation(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let mapped = self.widening_mapped_receiver(checked, saved);
        let failure = self.widening_failure_save(checked, &saved.environment, mapped);
        self.widening_failure_arms(checked, &failure)
    }

    fn widening_mapped_receiver(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let converted = self.widening_reader_conversion(checked, saved);
        let map = self.widening_map_save(checked, converted);
        self.widening_map_arms(checked, &map)
    }

    fn widening_reader_conversion(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let bound = self.widening_reader_bindings(checked, saved);
        self.widening_conversion(checked, bound)
    }

    fn widening_reader_bindings(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let payload = self.emit(
            Operation {
                construct_span: checked.roles.source.span,
                construct: Some(READER_PAYLOAD.into()),
                role: "widening_reader_payload".into(),
                ty: token(&crate::concrete_types::TypeTree::Tuple(vec![
                    crate::concrete_types::TypeTree::Scalar(checked.roles.width.scalar().into()),
                    crate::concrete_types::TypeTree::Scalar("usize".into()),
                ])),
                operands: vec![saved.input.clone()],
                ..template(checked.roles.payload, NodeCategory::Computation)
            },
            arm(&saved.selector, "true"),
        );
        let tuple = payload.value.clone().expect("saved reader success tuple");
        [
            (
                checked.roles.decoded,
                DECODED_PAYLOAD,
                "0",
                "widening_decoded_payload",
            ),
            (
                checked.roles.count,
                COUNT_PAYLOAD,
                "1",
                "widening_count_payload",
            ),
        ]
        .into_iter()
        .fold(payload, |flow, (binding, construct, position, role)| {
            let operation = Operation {
                construct_span: checked.roles.source.span,
                construct: Some(construct.into()),
                role: role.into(),
                literal: Some(position.into()),
                operands: vec![tuple.clone()],
                ..template(binding, NodeCategory::Computation)
            };
            self.widening_binding(operation, binding, flow)
        })
    }

    fn widening_binding(&mut self, payload: Operation, binding: &HirNode, flow: Flow) -> Flow {
        let declaration = Operation {
            construct_span: payload.construct_span,
            construct: Some(BINDING.into()),
            role: format!("{}_binding", payload.role),
            ty: "unit".into(),
            symbol: binding.binding.clone(),
            write_form: Some("initialization".into()),
            ..template(binding, NodeCategory::Declaration)
        };
        self.conditional_payload(payload, declaration, flow)
    }

    fn widening_conversion(&mut self, checked: &Checked<'_>, flow: Flow) -> Flow {
        let summary = crate::widening_readers::conversion(&checked.roles, self.facts)
            .expect("scoped pinned u32 widening conversion");
        let operand = self.expression(checked.roles.decoded_read, flow);
        self.emit(
            Operation {
                construct: Some(
                    if checked.roles.width == crate::native_reader_width::Width::U16 {
                        "data.integer.lossless-try-u16-usize".into()
                    } else {
                        crate::lossless_integer_calls::Kind::ConvertU32Usize
                            .construct()
                            .into()
                    },
                ),
                operator: Some(summary.callee_path),
                role: "widening_conversion".into(),
                operands: self.operand(&operand.value),
                ty: summary.result_type,
                ..template(checked.roles.conversion, NodeCategory::Call)
            },
            operand,
        )
    }

    fn widening_map_save(&mut self, checked: &Checked<'_>, converted: Flow) -> Saved {
        let input = converted.value.clone().expect("saved conversion Result");
        let created = self.emit(
            Operation {
                construct_span: checked.roles.map.span,
                construct: Some(MAP_ENVIRONMENT.into()),
                role: "widening_map_environment".into(),
                ty: token(&checked.proof.storage[1]),
                symbol: checked.roles.count.binding.clone(),
                ..template(checked.roles.success, NodeCategory::Read)
            },
            converted,
        );
        let environment = created.value.clone().expect("saved map environment");

        let selector = self.emit(
            Operation {
                construct: Some(MAP_SELECTOR.into()),
                role: "widening_map_selector".into(),
                ty: "bool".into(),
                operands: vec![input.clone()],
                ..template(checked.roles.map, NodeCategory::Control)
            },
            created,
        );
        Saved {
            input,
            environment,
            selector,
        }
    }

    fn widening_map_arms(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let yes = self.widening_map_success(checked, saved);
        let no = self.emit(
            Operation {
                construct: Some(MAP_ERROR.into()),
                role: "widening_map_error".into(),
                ty: token(&checked.proof.intermediate),
                operands: vec![saved.input.clone()],
                ..template(checked.roles.map, NodeCategory::Computation)
            },
            arm(&saved.selector, "false"),
        );
        self.join(
            checked.roles.map,
            saved.selector.value.clone().expect("map selector"),
            Arms { yes, no },
        )
    }

    fn widening_map_success(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let tuple = self.widening_success_tuple(checked, saved);
        self.emit(
            Operation {
                construct: Some(MAP_OK.into()),
                role: "widening_map_ok".into(),
                ty: token(&checked.proof.intermediate),
                operands: self.operand(&tuple.value),
                ..template(checked.roles.map, NodeCategory::Computation)
            },
            tuple,
        )
    }

    fn widening_success_values(&mut self, checked: &Checked<'_>, saved: &Saved) -> Operands {
        let payload = Operation {
            construct_span: checked.roles.map.span,
            construct: Some(CONVERSION_PAYLOAD.into()),
            role: "widening_converted_payload".into(),
            ty: "usize".into(),
            operands: vec![saved.input.clone()],
            ..template(checked.roles.converted, NodeCategory::Computation)
        };
        let bound = self.widening_binding(
            payload,
            checked.roles.converted,
            arm(&saved.selector, "true"),
        );
        let value = self.expression(checked.roles.value_read, bound);
        let converted = value.value.clone().expect("saved converted usize");

        let count = self.emit(
            Operation {
                construct: Some(COUNT_CAPTURE.into()),
                role: "widening_count_capture".into(),
                ty: "usize".into(),
                operands: vec![saved.environment.clone()],
                ..template(checked.roles.count_read, NodeCategory::Computation)
            },
            value,
        );
        Operands {
            values: vec![
                converted,
                count.value.clone().expect("saved captured count"),
            ],
            flow: count,
        }
    }

    fn widening_success_tuple(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let values = self.widening_success_values(checked, saved);
        self.emit(
            Operation {
                construct: Some(SUCCESS_TUPLE.into()),
                role: "widening_success_tuple".into(),
                ty: token(&crate::concrete_types::TypeTree::Tuple(vec![
                    crate::concrete_types::TypeTree::Scalar("usize".into()); 2
                ])),
                operands: values.values,
                ..template(checked.roles.tuple, NodeCategory::Computation)
            },
            values.flow,
        )
    }

    fn widening_failure_save(&mut self, checked: &Checked<'_>, outer: &str, mapped: Flow) -> Saved {
        let input = mapped
            .value
            .clone()
            .expect("saved mapped conversion Result");
        let created = self.emit(
            Operation {
                construct_span: checked.roles.map_err.span,
                construct: Some(FAILURE_ENVIRONMENT.into()),
                role: "widening_failure_environment".into(),
                ty: token(&checked.proof.storage[2]),
                operands: vec![outer.into()],
                ..template(checked.roles.failure, NodeCategory::Computation)
            },
            mapped,
        );
        let environment = created.value.clone().expect("saved failure environment");

        let selector = self.emit(
            Operation {
                construct: Some(FAILURE_SELECTOR.into()),
                role: "widening_failure_selector".into(),
                ty: "bool".into(),
                operands: vec![input.clone()],
                ..template(checked.roles.map_err, NodeCategory::Control)
            },
            created,
        );
        Saved {
            input,
            environment,
            selector,
        }
    }

    fn widening_failure_arms(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let yes = self.widening_final_error(checked, saved);
        let no = self.widening_final_success(checked, saved);
        self.join(
            checked.roles.map_err,
            saved.selector.value.clone().expect("failure selector"),
            Arms { yes, no },
        )
    }

    fn widening_final_error(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let error = self.widening_error_constructor(checked, saved);
        self.emit(
            Operation {
                construct: Some(FINAL_ERROR.into()),
                role: "widening_final_error".into(),
                ty: token(&checked.proof.output),
                operands: self.operand(&error.value),
                ..template(checked.roles.map_err, NodeCategory::Computation)
            },
            error,
        )
    }

    fn widening_discard_error(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        // This operation consumes the opaque error inside its guarded carrier.
        // Its unit result is the discard completion, never an error value.
        self.emit(
            Operation {
                construct_span: checked.roles.map_err.span,
                construct: Some(ERROR_DISCARD.into()),
                role: "widening_error_discard".into(),
                ty: "unit".into(),
                operands: vec![saved.input.clone()],
                ..template(checked.roles.wildcard, NodeCategory::Computation)
            },
            arm(&saved.selector, "true"),
        )
    }

    fn widening_error_constructor(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let discarded = self.widening_discard_error(checked, saved);
        let message = self.emit(
            Operation {
                construct: Some(MESSAGE_CAPTURE.into()),
                role: "widening_message_capture".into(),
                ty: crate::data_aggregates::token(checked.roles.message_read)
                    .expect("static message type")
                    .into(),
                operands: vec![saved.environment.clone()],
                ..template(checked.roles.message_read, NodeCategory::Computation)
            },
            discarded,
        );
        self.emit(
            Operation {
                construct: self.computation_construct(checked.roles.constructor),
                operator: crate::helper_calls::callee(checked.roles.constructor, self.facts),
                role: "widening_error_constructor".into(),
                operands: self.operand(&message.value),
                ..template(checked.roles.constructor, NodeCategory::Call)
            },
            message,
        )
    }

    fn widening_final_success(&mut self, checked: &Checked<'_>, saved: &Saved) -> Flow {
        let success = self.emit(
            Operation {
                construct: Some(FINAL_PAYLOAD.into()),
                role: "widening_final_payload".into(),
                ty: token(&crate::concrete_types::TypeTree::Tuple(vec![
                    crate::concrete_types::TypeTree::Scalar("usize".into()); 2
                ])),
                operands: vec![saved.input.clone()],
                ..template(checked.roles.map_err, NodeCategory::Computation)
            },
            arm(&saved.selector, "false"),
        );
        self.emit(
            Operation {
                construct: Some(FINAL_OK.into()),
                role: "widening_final_ok".into(),
                ty: token(&checked.proof.output),
                operands: self.operand(&success.value),
                ..template(checked.roles.map_err, NodeCategory::Computation)
            },
            success,
        )
    }
}

fn token(ty: &crate::concrete_types::TypeTree) -> String {
    crate::data_types::token(ty).expect("verified widening reader concrete type")
}
