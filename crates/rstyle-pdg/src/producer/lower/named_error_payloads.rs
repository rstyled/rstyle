//! Fresh named error fields retain declaration slots and source evaluation order.
use super::*;

impl Builder<'_> {
    pub(super) fn named_error_constructor(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::named_error_payloads::constructor(node, self.facts)
            .expect("verified named error constructor")
            .expect("selected named error constructor");
        let (construct, operator, literal) = match checked {
            crate::named_error_payloads::Constructor::Named {
                variant, fields, ..
            } => {
                let slots: Vec<_> = fields
                    .iter()
                    .map(|field| (field.position, field.name.as_str()))
                    .collect();
                (
                    "data.named-error",
                    Some(variant),
                    serde_json::to_string(&slots).expect("declaration slots"),
                )
            }
            crate::named_error_payloads::Constructor::Wrapper { .. } => {
                ("data.named-error-wrapper", None, "0".into())
            }
        };
        let operation = Operation {
            construct: Some(construct.into()),
            operator,
            literal: Some(literal),
            ..template(node, NodeCategory::Computation)
        };
        self.constructed_value(node, operation, flow)
    }
}
