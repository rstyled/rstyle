//! Materialize the checked complete value at its original constant-use operation.
use super::*;

impl Builder<'_> {
    pub(super) fn product_constant(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let value = crate::product_constant_values::verify(node, self.facts)
            .expect("verified source-owned product constant");
        let literal = crate::product_constant_values::literal(&value)
            .expect("checked typed scalar field serializes");
        self.emit(
            Operation {
                construct: Some(crate::product_constant_values::CONSTRUCT.into()),
                literal: Some(literal),
                ty: value.owner_type,
                ..template(node, NodeCategory::Computation)
            },
            flow,
        )
    }
}
