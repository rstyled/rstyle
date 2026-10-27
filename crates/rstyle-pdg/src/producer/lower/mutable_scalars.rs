//! Separate referent loads/stores and checked failures preserve source order.
use super::*;

impl Builder<'_> {
    pub(super) fn scalar_referent(&mut self, node: &HirNode, root: u32, flow: Flow) -> Flow {
        let pointer = self.expression(&node.children[0], flow);
        let operation = Operation {
            position: Some(u64::from(root)),
            construct: Some(crate::mutable_scalars::LOAD.into()),
            operands: self.operand(&pointer.value),
            ..template(node, NodeCategory::Read)
        };
        self.emit(operation, pointer)
    }

    pub(super) fn scalar_compound(&mut self, node: &HirNode, root: u32, flow: Flow) -> Flow {
        let rhs = self.expression(&node.children[1], flow);
        let rhs_value = rhs.value.clone();
        let loaded = self.scalar_referent(&node.children[0], root, rhs);
        let evaluated = Operands {
            values: self
                .operand(&loaded.value)
                .into_iter()
                .chain(self.operand(&rhs_value))
                .collect(),
            flow: loaded,
        };

        self.scalar_compound_value(node, root, evaluated)
    }

    fn scalar_compound_value(&mut self, node: &HirNode, root: u32, evaluated: Operands) -> Flow {
        let dispatch = node
            .dispatch
            .as_ref()
            .expect("checked builtin scalar compound");
        let operation = Operation {
            ty: "usize".into(),
            operator: Some(dispatch.op.trim_end_matches('=').into()),
            role: "compound_computation".into(),
            operands: evaluated.values,
            ..template(node, NodeCategory::Computation)
        };
        let computed = self.emit(operation, evaluated.flow);

        let store = Operation {
            position: Some(u64::from(root)),
            construct: Some(crate::mutable_scalars::STORE.into()),
            role: "compound_write".into(),
            ty: "unit".into(),
            write_form: Some("compound_assignment".into()),
            operator: Some(dispatch.op.clone()),
            operands: self.operand(&computed.value),
            ..template(node, NodeCategory::Assignment)
        };
        self.emit(store, computed)
    }

    pub(super) fn index_check(&mut self, node: &HirNode, values: &[String], flow: Flow) -> Flow {
        if !crate::mutable_scalars::enabled(self.facts) {
            return flow;
        }
        let check = Operation {
            construct: Some("data.mutable-index-check".into()),
            role: "index_bounds_check".into(),
            ty: "unit".into(),
            operands: values.to_vec(),
            ..template(node, NodeCategory::Computation)
        };
        self.emit(check, flow)
    }

    pub(super) fn checked_failure(&mut self, operation: &Operation) {
        if !crate::mutable_scalars::enabled(self.facts) {
            return;
        }
        let arithmetic = operation.category == NodeCategory::Computation
            && matches!(operation.ty.as_str(), "usize" | "u8")
            && operation.operator.as_deref().is_some_and(|operator| {
                matches!(operator, "/" | "%")
                    || self.facts.build.overflow_checks.value == Some(true)
                        && matches!(operator, "+" | "-" | "*" | "<<" | ">>")
            });
        if arithmetic || operation.construct.as_deref() == Some("data.mutable-index-check") {
            self.transfers.push(Transfer {
                source: operation.id.clone(),
                target: format!("P:{}", operation.id),
                kind: "exception".into(),
                loop_role: None,
            });
        }
    }
}
