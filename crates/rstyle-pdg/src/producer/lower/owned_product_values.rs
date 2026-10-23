//! Copy field replacement yields a new complete local value, then defines it.
use super::*;

impl Builder<'_> {
    pub(super) fn owned_product_update(
        &mut self,
        update: crate::owned_product_locals::CheckedUpdate<'_>,
        flow: Flow,
    ) -> Flow {
        let rhs = self.expression(update.value(), flow);
        let value = rhs.value.clone();

        let old = self.path(update.receiver(), rhs);
        self.replace_owned_product(update, value, old)
    }

    fn replace_owned_product(
        &mut self,
        update: crate::owned_product_locals::CheckedUpdate<'_>,
        value: Option<String>,
        old: Flow,
    ) -> Flow {
        let node = update.assignment();
        let checked =
            crate::owned_product_locals::verify(self.facts).expect("checked affine owner");

        let updated = self.emit(
            Operation {
                construct: Some(crate::owned_product_values::UPDATE.into()),
                ty: checked.product().text().expect("checked product type"),
                literal: Some(update.field_position().to_string()),
                operands: self
                    .operand(&old.value)
                    .into_iter()
                    .chain(self.operand(&value))
                    .collect(),
                ..template(node, NodeCategory::Computation)
            },
            old,
        );

        let operation = Operation {
            operator: Some("=".into()),
            operands: self.operand(&updated.value),
            ..self.write_target(node, update.receiver(), "reassignment")
        };

        self.emit(operation, updated)
    }
}
