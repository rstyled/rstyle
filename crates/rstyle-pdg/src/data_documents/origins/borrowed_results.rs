//! Result branches expose success roots only; verified Err paths have none.
use super::*;
use crate::unit_store::EvidenceFacts;

impl Graph<'_> {
    pub(super) fn borrowed_result_operation(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        let construct = node
            .attributes
            .construct
            .as_deref()
            .ok_or("Missing Result operation construct")?;
        let source = source_node(node, self.facts)?;

        if matches!(construct, "data.result-ok" | "data.result-err") {
            return self.result_constructor(node, selectors, visiting);
        }
        if construct.starts_with("try.") {
            return self.propagated_result(node, selectors, visiting);
        }

        no_selectors(selectors)?;
        let receiver = self.checked_receiver(node, source, construct)?;
        self.origins(receiver, &[], visiting)
    }

    fn checked_receiver(
        &self,
        node: &PdgNode,
        source: &HirNode,
        construct: &str,
    ) -> Result<&PdgNode, String> {
        let receiver = if construct == "data.option-get-suffix" {
            crate::checked_slice_views::suffix::verify(source, self.facts)?.receiver
        } else if construct == "data.option-get-prefix" {
            crate::checked_slice_views::verify(source, self.facts)?.receiver
        } else {
            let checked = crate::borrowed_result_errors::verify(source, self.facts)?;
            self.saved_operand(node, checked.error, 1)?;
            empty_error_payload(checked.error)?;
            checked.receiver
        };

        self.saved_operand(node, receiver, 0)
    }

    fn result_constructor(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        let source = source_node(node, self.facts)?;

        let checked = crate::concrete_results::verify_constructor(source, self.facts)?;

        if Some(checked.construct) != node.attributes.construct.as_deref() {
            return Err("Result graph constructor disagrees with its exact source variant".into());
        }

        let payload = self.saved_operand(node, checked.payload, 0)?;

        if checked.construct == "data.result-err" {
            no_selectors(selectors)?;
            return empty_error_payload(checked.payload);
        }
        self.origins(payload, selectors, visiting)
    }

    fn propagated_result(
        &self,
        node: &PdgNode,
        selectors: &[usize],
        visiting: &mut BTreeSet<String>,
    ) -> Result<Vec<Origin>, String> {
        let construct = node
            .attributes
            .construct
            .as_deref()
            .ok_or("Missing Result operation construct")?;
        let source = source_node(node, self.facts)?;

        no_selectors(selectors)?;

        if construct == "try.scalar-result-error" {
            let checked = crate::scalar_result_propagation::verify(source, self.facts)?;
            self.saved_operand(node, checked.operand, 0)?;
            if wire_type(node)? != checked.output_token {
                return Err("Scalar propagated error loses complete output Result type".into());
            }
            return Ok(Vec::new());
        }
        let checked = crate::borrowed_result_propagation::verify(source, self.facts)?;

        let saved = self.saved_operand(node, checked.operand, 0)?;

        if construct == "try.borrowed-result-error" {
            if wire_type(node)? != checked.output_token {
                return Err(
                    "Propagated error graph must have the complete output Result type".into(),
                );
            }
            return Ok(Vec::new());
        }

        if wire_type(node)? != checked.payload_token {
            return Err(
                "Propagated success graph must have the exact borrowed payload type".into(),
            );
        }
        self.origins(saved, &[], visiting)
    }

    fn saved_operand(
        &self,
        node: &PdgNode,
        source: &HirNode,
        position: u64,
    ) -> Result<&PdgNode, String> {
        let actual = self.operand(node, position)?;

        let expected = super::super::value_operation(self.pair, source)?;

        if actual.id != expected.id {
            return Err(
                "Result reference path substitutes a different saved source operand".into(),
            );
        }
        Ok(actual)
    }
}

fn no_selectors(selectors: &[usize]) -> Result<(), String> {
    if !selectors.is_empty() {
        return Err("Borrowed Result slice/error has no aggregate component selector".into());
    }
    Ok(())
}

fn empty_error_payload(source: &HirNode) -> Result<Vec<Origin>, String> {
    if data_origins::shared(&crate::data_aggregates::parsed_type(source)?) {
        return Err("Result error unexpectedly contains reference origins".into());
    }
    Ok(Vec::new())
}

/// This is an explicit source-proved exception for Err, never a missing-root default.
pub(super) fn verified_empty(node: &PdgNode, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let token = crate::data_types::parse(&wire_type(node)?)?;

    let crate::data_types::TypeToken::Result { ok, err } = token else {
        return Ok(false);
    };

    if crate::concrete_results::fresh_products::GRAMMAR.permits(&ok, &err) {
        // A structural product shape grants no empty-path exception: prove the
        // complete success owner and actual Copy error before examining Err.
        let boundary = crate::concrete_results::fresh_products::product_boundary(facts)?;
        if boundary.ok != *ok || boundary.error != *err {
            return Err(
                "Empty product Result path differs from its complete return boundary".into(),
            );
        }
    } else if !crate::borrowed_result_types::GRAMMAR.permits(&ok, &err)
        && !crate::concrete_results::copy_errors::GRAMMAR.permits(&ok, &err)
    {
        return Ok(false);
    }

    let source = source_node(node, facts)?;

    if node.attributes.construct.as_deref() == Some("try.scalar-result-error") {
        let checked = crate::scalar_result_propagation::verify(source, facts)?;
        return Ok(wire_type(node)? == checked.output_token);
    }
    if node.attributes.construct.as_deref() == Some("try.borrowed-result-error") {
        let checked = crate::borrowed_result_propagation::verify(source, facts)?;

        return Ok(wire_type(node)? == checked.output_token);
    }

    if generated_error_return(source, facts)? {
        return Ok(node.category == NodeCategory::Return);
    }
    source_error(source, facts)
}

fn source_error(source: &HirNode, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if crate::concrete_results::constructor_candidate(source, facts) {
        let checked = crate::concrete_results::verify_constructor(source, facts)?;

        empty_error_payload_if_err(&checked)
    } else if matches!(source.kind.as_str(), "expr_ret" | "expr_block") {
        source
            .children
            .last()
            .map_or(Ok(false), |value| source_error(value, facts))
    } else {
        Ok(false)
    }
}

fn empty_error_payload_if_err(
    checked: &crate::concrete_results::Constructor<'_>,
) -> Result<bool, String> {
    if checked.construct != "data.result-err" {
        return Ok(false);
    }
    empty_error_payload(checked.payload)?;

    Ok(true)
}

pub(super) fn generated_error_return(
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    if source.kind != "expr_ret" {
        return Ok(false);
    }

    for question_mark in crate::field_locations::selected_operations(
        facts,
        &["expr_try_desugar"],
        crate::scalar_result_propagation::candidate,
    ) {
        let checked = crate::scalar_result_propagation::verify(question_mark, facts)?;
        if checked.early_return.hir == source.hir && checked.early_return.span == source.span {
            return Ok(true);
        }
    }
    for question_mark in crate::field_locations::selected_operations(
        facts,
        &["expr_try_desugar"],
        crate::borrowed_result_propagation::candidate,
    ) {
        let checked = crate::borrowed_result_propagation::verify(question_mark, facts)?;

        if checked.early_return.hir == source.hir && checked.early_return.span == source.span {
            return Ok(true);
        }
    }
    Ok(false)
}
