//! Bounded consuming callback: save suffix first, then branch on one scalar callback.
use super::*;
use crate::nonzero_digit_calls::Checked;

struct SavedPayload<'a> {
    scalar: &'a str,
    suffix: &'a str,
}
struct SummarySource<'a> {
    node: &'a HirNode,
}

impl Builder<'_> {
    pub(super) fn nonzero_digits(&mut self, checked: &Checked<'_>, flow: Flow) -> Flow {
        let (selected, saved) = self.nonzero_parser(checked, flow);
        self.nonzero_outer_branch(checked, &saved, selected)
    }

    fn nonzero_outer_branch(&mut self, checked: &Checked<'_>, saved: &str, selected: Flow) -> Flow {
        let controller = selected.value.clone().expect("Option selector");
        let yes = self.nonzero_some(checked, saved, arm(&selected, "true"));
        let no = self.nonzero_none(checked.calls.outer, "parser-none", arm(&selected, "false"));
        self.join(checked.calls.outer, controller, Arms { yes, no })
    }

    fn nonzero_parser(&mut self, checked: &Checked<'_>, flow: Flow) -> (Flow, String) {
        let parsed = self.expression(checked.parser(), flow);
        let saved = parsed.value.clone().expect("verified parser result");
        let callback = SummarySource {
            node: checked.calls.callback,
        };
        let created = self.emit(callback.value("environment", "unit", vec![]), parsed);

        let source = SummarySource {
            node: checked.calls.outer,
        };
        (self.emit(source.control("some", &saved), created), saved)
    }

    fn nonzero_some(&mut self, checked: &Checked<'_>, saved: &str, flow: Flow) -> Flow {
        let (scalar, suffix) = self.nonzero_payload(checked, saved, flow);
        let value = scalar.value.clone().expect("saved scalar");
        let source = SummarySource {
            node: checked.calls.outer,
        };
        let selected = self.emit(source.control("nonzero", &value), scalar);

        let payload = SavedPayload {
            scalar: &value,
            suffix: &suffix,
        };
        self.nonzero_branch(checked, payload, selected)
    }

    fn nonzero_payload(
        &mut self,
        checked: &Checked<'_>,
        saved: &str,
        flow: Flow,
    ) -> (Flow, String) {
        let source = SummarySource {
            node: checked.calls.outer,
        };
        let input = checked.input.type_token();
        let payload = self.emit(source.value("payload", &input, vec![saved.into()]), flow);
        let product = payload.value.clone().expect("Some payload");

        let suffix_ty = crate::data_types::concrete_text(&checked.input.fields[0].ty);
        let suffix = self.emit(
            source.value("suffix", &suffix_ty, vec![product.clone()]),
            payload,
        );
        let saved_suffix = suffix.value.clone().expect("saved suffix");

        let scalar = self.emit(
            source.value("scalar", checked.input.argument.as_str(), vec![product]),
            suffix,
        );
        (scalar, saved_suffix)
    }

    fn nonzero_branch(
        &mut self,
        checked: &Checked<'_>,
        payload: SavedPayload<'_>,
        selected: Flow,
    ) -> Flow {
        let controller = selected.value.clone().expect("NonZero selector");
        let yes = self.nonzero_success(checked, payload, arm(&selected, "true"));
        let no = self.nonzero_none(checked.calls.outer, "zero-none", arm(&selected, "false"));
        self.nonzero_join(checked.calls.outer, controller, Arms { yes, no })
    }

    fn nonzero_join(&mut self, node: &HirNode, controller: String, arms: Arms) -> Flow {
        let merged = self.join(node, controller, arms);
        self.nonzero_merge_role(&merged);
        merged
    }

    fn nonzero_merge_role(&mut self, merged: &Flow) {
        let id = merged
            .value
            .as_ref()
            .expect("complete nonzero payload merge");

        self.nodes
            .iter_mut()
            .find(|operation| &operation.id == id)
            .expect("new payload merge")
            .role = "nonzero_payload_merge".into();
    }

    fn nonzero_success(
        &mut self,
        checked: &Checked<'_>,
        saved: SavedPayload<'_>,
        flow: Flow,
    ) -> Flow {
        let node = checked.calls.outer;

        let source = SummarySource { node };

        let nonzero_ty = crate::data_types::concrete_text(&checked.output.fields[1].ty);

        let nonzero = self.emit(
            source.value("value", &nonzero_ty, vec![saved.scalar.into()]),
            flow,
        );

        let payload = nonzero.value.clone().expect("nonzero scalar");

        let product_ty = checked.output.type_token();

        let product = self.emit(
            source.value("product", &product_ty, vec![saved.suffix.into(), payload]),
            nonzero,
        );

        let saved = product.value.clone().expect("complete output product");

        self.emit(
            source.value("success", output_token(node), vec![saved]),
            product,
        )
    }

    fn nonzero_none(&mut self, node: &HirNode, role: &str, flow: Flow) -> Flow {
        self.emit(
            SummarySource { node }.value(role, output_token(node), vec![]),
            flow,
        )
    }
}

impl SummarySource<'_> {
    fn control(&self, role: &str, saved: &str) -> Operation {
        Operation {
            ty: "bool".into(),
            operands: vec![saved.into()],
            ..self.operation(role, NodeCategory::Control)
        }
    }

    fn value(&self, role: &str, ty: &str, operands: Vec<String>) -> Operation {
        Operation {
            ty: ty.into(),
            operands,
            ..self.operation(role, NodeCategory::Computation)
        }
    }

    fn operation(&self, role: &str, category: NodeCategory) -> Operation {
        Operation {
            construct: Some(format!("callback.nonzero-{role}")),
            role: format!("nonzero_{role}"),
            ..template(self.node, category)
        }
    }
}

fn output_token(node: &HirNode) -> &str {
    node.ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .expect("complete result type")
}
