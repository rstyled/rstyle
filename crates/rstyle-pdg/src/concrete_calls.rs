//! Shared signature proof for bounded concrete library calls. Authority, source
//! correspondence and effects remain each operation contract's responsibility.
use crate::{
    HirNode,
    concrete_types::{Observation, TypeTree},
    unit_store::EvidenceFacts,
};

pub(crate) trait ConcreteCall: Copy {
    fn trees(self) -> (Vec<TypeTree>, TypeTree);

    fn signature(self) -> Result<(Vec<String>, String), String> {
        let (inputs, output) = self.trees();
        Ok((
            inputs
                .iter()
                .map(crate::data_types::token)
                .collect::<Result<_, _>>()?,
            crate::data_types::token(&output)?,
        ))
    }

    fn verify_types(
        self,
        node: &HirNode,
        facts: &dyn EvidenceFacts,
        direct: bool,
    ) -> Result<(), String> {
        let (inputs, output) = self.signature()?;
        let dispatch = node
            .dispatch
            .as_ref()
            .ok_or("Missing concrete call dispatch")?;
        let operands = node
            .children
            .iter()
            .map(crate::data_aggregates::token)
            .collect::<Result<Vec<_>, _>>()?;

        if node.expansion.is_some()
            || dispatch.kind
                != if direct {
                    "resolved_direct"
                } else {
                    "resolved_method"
                }
            || node.kind
                != if direct {
                    "expr_call"
                } else {
                    "expr_method_call"
                }
            || dispatch.operand_tys != inputs
            || dispatch.result_ty != output
            || crate::data_aggregates::token(node)? != output
            || operands != inputs
        {
            return Err("Concrete call loses its exact builtin signature".into());
        }
        if !direct {
            self.verify_receiver(node, facts)?;
        }
        Ok(())
    }

    fn verify_receiver(self, node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
        let capture = crate::data_receivers::verify(facts)?;
        let method = capture
            .methods
            .iter()
            .find(|method| method.call == node.hir)
            .ok_or("Missing captured concrete call receiver")?;
        let (inputs, output) = self.trees();
        let observed = |ty| Observation::Captured { ty };
        let receiver = observed(
            inputs
                .first()
                .ok_or("Missing concrete receiver type")?
                .clone(),
        );

        if method.receiver_type != receiver
            || method.adjusted_type != receiver
            || method.signature_inputs != inputs.into_iter().map(observed).collect::<Vec<_>>()
            || method.signature_result != observed(output)
            || !method.adjustments.is_empty()
        {
            return Err("Concrete receiver/signature disagrees with capture".into());
        }
        Ok(())
    }
}
