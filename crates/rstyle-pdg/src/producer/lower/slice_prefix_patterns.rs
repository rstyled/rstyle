//! Original prefix HIR anchors every operation; metadata supplies no authority.
//!
//! Integration must replay the checked source contract: saved-input identity,
//! exact constant regions, ordered true-edge guards and byte READ hazards.
//! Element/suffix computations create borrowed values, never stored-slot READs.
use super::*;
use crate::slice_patterns::semantics::{Checked, Region, SelectorKind};

pub(crate) const LENGTH: &str = "slice.prefix-length";
pub(crate) const BYTE: &str = "slice.prefix-byte";
pub(crate) const DIGIT: &str = "slice.prefix-digit";
pub(crate) const ELEMENT: &str = "slice.prefix-element";
pub(crate) const SUFFIX: &str = "slice.prefix-suffix";

struct PrefixInput<'a> {
    checked: Checked<'a>,
    saved: String,
}

struct PrefixSelection {
    matched: Flow,
    failed: Flow,
    controller: String,
}

impl Builder<'_> {
    pub(super) fn slice_prefix_match(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::slice_patterns::semantics::verify(node, self.facts)
            .expect("verified bounded original prefix match");
        let evaluated = self.expression(checked.scrutinee(), flow);
        let saved = evaluated.value.clone().expect("saved original slice input");
        let input = PrefixInput { checked, saved };
        let selected = self.prefix_selectors(&input, evaluated);

        self.prefix_arms(node, &input, selected)
    }

    fn prefix_selectors(&mut self, input: &PrefixInput<'_>, evaluated: Flow) -> PrefixSelection {
        let selected = self.prefix_length(input, evaluated);
        (0..input.checked.digits().len()).fold(selected, |selected, position| {
            self.prefix_digit(input, position, selected)
        })
    }

    fn prefix_arms(
        &mut self,
        node: &HirNode,
        input: &PrefixInput<'_>,
        selected: PrefixSelection,
    ) -> Flow {
        let yes = self.prefix_success(input, selected.matched);
        let no = self.expression(input.checked.failure(), selected.failed);
        self.match_join(node, selected.controller, vec![(0, yes), (1, no)])
    }

    fn prefix_success(&mut self, input: &PrefixInput<'_>, matched: Flow) -> Flow {
        let bound = self.prefix_bindings(input, matched);
        self.expression(input.checked.success(), bound)
    }

    fn prefix_length(&mut self, input: &PrefixInput<'_>, flow: Flow) -> PrefixSelection {
        let selected = self.emit(
            Operation {
                construct_span: input.checked.matched().span,
                construct: Some(LENGTH.into()),
                role: "prefix_length".into(),
                ty: "bool".into(),
                literal: Some(wire(&SelectorKind::Length {
                    minimum: input.checked.consumed(),
                })),
                operands: vec![input.saved.clone()],
                ..template(input.checked.prefix().pattern(), NodeCategory::Control)
            },
            flow,
        );

        PrefixSelection {
            matched: arm(&selected, "true"),
            failed: arm(&selected, "false"),
            controller: selected.value.expect("prefix length controller"),
        }
    }

    fn prefix_digit(
        &mut self,
        input: &PrefixInput<'_>,
        position: usize,
        selected: PrefixSelection,
    ) -> PrefixSelection {
        let loaded = self.prefix_byte(input, position, selected.matched);
        let range = input.checked.digits()[position].range();
        let predicate = self.emit(
            Operation {
                construct_span: input.checked.matched().span,
                construct: Some(DIGIT.into()),
                role: format!("prefix_digit:{position}"),
                ty: "bool".into(),
                literal: Some(wire(&SelectorKind::Digit {
                    position: position as u8,
                    lower: b'0',
                    upper: b'9',
                })),
                operands: self.operand(&loaded.value),
                ..template(range, NodeCategory::Control)
            },
            loaded,
        );

        PrefixSelection {
            matched: arm(&predicate, "true"),
            failed: combine_frontiers(selected.failed, arm(&predicate, "false")),
            controller: selected.controller,
        }
    }

    fn prefix_byte(&mut self, input: &PrefixInput<'_>, position: usize, flow: Flow) -> Flow {
        self.emit(
            Operation {
                construct_span: input.checked.matched().span,
                construct: Some(BYTE.into()),
                role: format!("prefix_selector_read:{position}"),
                ty: "u8".into(),
                literal: Some(wire(&Region::Element {
                    position: position as u8,
                })),
                operands: vec![input.saved.clone()],
                ..template(input.checked.digits()[position].range(), NodeCategory::Read)
            },
            flow,
        )
    }

    fn prefix_bindings(&mut self, input: &PrefixInput<'_>, flow: Flow) -> Flow {
        let flow =
            input
                .checked
                .digits()
                .iter()
                .enumerate()
                .fold(flow, |flow, (position, digit)| {
                    let value = Operation {
                        construct_span: input.checked.matched().span,
                        construct: Some(ELEMENT.into()),
                        role: format!("prefix_element:{position}"),
                        literal: Some(wire(&Region::Element {
                            position: position as u8,
                        })),
                        operands: vec![input.saved.clone()],
                        ..template(digit.binding(), NodeCategory::Computation)
                    };
                    self.prefix_binding(value, digit.binding(), flow)
                });

        self.prefix_suffix(input, flow)
    }

    fn prefix_suffix(&mut self, input: &PrefixInput<'_>, flow: Flow) -> Flow {
        let remainder = input.checked.prefix().remainder();
        let value = Operation {
            construct_span: input.checked.matched().span,
            construct: Some(SUFFIX.into()),
            role: "prefix_suffix".into(),
            literal: Some(wire(&Region::Suffix {
                start: input.checked.consumed(),
            })),
            operands: vec![input.saved.clone()],
            ..template(remainder.rest(), NodeCategory::Computation)
        };
        let value = Operation {
            ty: crate::data_aggregates::token(remainder.binding())
                .expect("verified borrowed suffix type")
                .into(),
            ..value
        };

        self.prefix_binding(value, remainder.binding(), flow)
    }

    fn prefix_binding(&mut self, value: Operation, binding: &HirNode, flow: Flow) -> Flow {
        let declaration = Operation {
            construct_span: value.construct_span,
            role: "prefix_binding".into(),
            ty: "unit".into(),
            symbol: binding.binding.clone(),
            write_form: Some("initialization".into()),
            ..template(binding, NodeCategory::Declaration)
        };

        self.conditional_payload(value, declaration, flow)
    }
}

fn wire(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("bounded prefix semantic metadata")
}
