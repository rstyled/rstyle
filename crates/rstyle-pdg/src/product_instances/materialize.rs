//! Normalize only the independently proved original S formal and its sole read.
use super::{SpanArgument, conversion, source};
use crate::{
    HirFnFacts, HirNode, HirRef,
    concrete_types::{self, InterfaceObservations, Observation, TypeTree},
};
use serde_json::Value;
#[cfg(test)]
mod tests;

/// A complete source/type/conversion projection, separate from frontend admission.
pub struct CheckedProjection {
    facts: HirFnFacts,
    argument: SpanArgument,
    operation: HirNode,
    source: crate::nominal_boundary::resolution::CheckedCopyProductSource,
}

impl CheckedProjection {
    pub fn facts(&self) -> &HirFnFacts {
        &self.facts
    }

    pub fn argument(&self) -> SpanArgument {
        self.argument
    }

    pub(crate) fn copy_source(
        &self,
    ) -> &crate::nominal_boundary::resolution::CheckedCopyProductSource {
        &self.source
    }

    pub fn operation(&self) -> &HirNode {
        &self.operation
    }

    pub fn receiver(&self) -> &HirNode {
        &self.operation.children[0]
    }
}

pub fn verify(
    canonical: &HirFnFacts,
    observation: &Value,
    argument: SpanArgument,
) -> Result<CheckedProjection, String> {
    let mut interface = canonical
        .build
        .frontend_options
        .data::<InterfaceObservations>()?
        .as_ref()
        .clone();
    let Observation::Captured { ty: result } = &interface.result else {
        return Err("Product instance requires its complete captured result".into());
    };

    let source = source::verify(canonical, observation, result)?;
    let copy_source = crate::nominal_boundary::resolution::checked_copy_product_source(canonical)?;
    nominal_types(canonical, observation, result)?;
    let span = span(result)?;
    let conversion = conversion::verify(canonical, observation, span, argument)?;
    if conversion.call() != source.conversion() || conversion.receiver() != source.receiver() {
        return Err("Product conversion and source proofs identify different operations".into());
    }
    verify_original_receiver(canonical, conversion.call())?;
    let token = crate::data_types::token(span)?;
    let formal = source.input().hir.clone();
    let facts = concrete_facts(canonical, &formal, &token)?;
    let operation = concrete_operation(&facts, conversion.call(), &formal, &token)?;
    let [_, input] = interface.parameters.as_mut_slice() else {
        return Err("Product instance requires its exact two-formal interface".into());
    };
    if !matches!(input, Observation::Unsupported { .. }) {
        return Err(
            "Product input must retain its original unsupported generic observation".into(),
        );
    }
    *input = Observation::Captured { ty: span.clone() };
    let mut facts = facts;
    facts.build.frontend_options.insert(
        concrete_types::KEY.into(),
        serde_json::to_value(interface).map_err(|error| error.to_string())?,
    );
    Ok(CheckedProjection {
        facts,
        argument,
        operation,
        source: copy_source,
    })
}

fn verify_original_receiver(canonical: &HirFnFacts, call: &HirNode) -> Result<(), String> {
    let capture = crate::data_receivers::verify_capture(canonical)?;
    let [method] = capture.methods.as_slice() else {
        return Err(
            "Product constructor requires its exact original conversion receiver roster".into(),
        );
    };
    if method.call != call.hir
        || !method.adjustments.is_empty()
        || method.signature_inputs.as_slice() != [method.receiver_type.clone()]
        || !matches!(&method.receiver_type, Observation::Unsupported { reason } if !reason.is_empty())
    {
        return Err("Product constructor receiver differs from its proved conversion".into());
    }
    Ok(())
}

fn concrete_operation(
    facts: &HirFnFacts,
    original: &HirNode,
    formal: &HirRef,
    token: &str,
) -> Result<HirNode, String> {
    let mut expected = original.clone();
    if normalize_read(&mut expected, formal, token)? != 1 || expected.children.len() != 1 {
        return Err(
            "Product conversion must retain its single normalized original receiver".into(),
        );
    }
    let operations =
        crate::field_locations::operations(&facts.body, &["expr_call", "expr_method_call"]);
    let matching: Vec<_> = operations
        .into_iter()
        .filter(|node| node.hir == original.hir)
        .collect();
    if matching.as_slice() != [&expected] {
        return Err("Product conversion differs from its checked normalized operation".into());
    }

    Ok(expected)
}

fn nominal_types(
    canonical: &HirFnFacts,
    observed: &Value,
    result: &TypeTree,
) -> Result<(), String> {
    let TypeTree::Struct { fields, .. } = result else {
        return Err("Product instance requires its complete nominal result".into());
    };
    let [pattern, _] = fields.as_slice() else {
        return Err("Product instance requires both original components".into());
    };
    for (projected, original, tree) in [
        (
            &observed["projected_inputs"][0],
            &observed["original_signature"]["inputs"][0],
            &pattern.ty,
        ),
        (
            &observed["projected_result"],
            &observed["original_signature"]["output"],
            result,
        ),
    ] {
        conversion::verify_local_argument(canonical, projected, original, tree)?;
    }
    Ok(())
}

fn span(result: &TypeTree) -> Result<&TypeTree, String> {
    let TypeTree::Struct { fields, .. } = result else {
        return Err("Product result lacks its complete nominal structure".into());
    };
    let [pattern, span] = fields.as_slice() else {
        return Err("Product result requires both complete original components".into());
    };
    if pattern.name != "pattern" || span.name != "span" {
        return Err("Product components differ from original constructor order".into());
    }
    Ok(&span.ty)
}

fn concrete_facts(
    canonical: &HirFnFacts,
    formal: &HirRef,
    token: &str,
) -> Result<HirFnFacts, String> {
    let mut facts = canonical.clone();
    normalize_parameter(&mut facts.signature.params[1], token)?;
    normalize_binding(&mut facts.bindings, formal, token)?;

    if normalize_read(&mut facts.body, formal, token)? != 1 {
        return Err("Product instance requires its single original generic value read".into());
    }
    Ok(facts)
}

fn normalize_parameter(parameter: &mut crate::ParamFact, token: &str) -> Result<(), String> {
    generic_type(&mut parameter.ty, token)?;
    generic_type(
        parameter
            .pat
            .ty
            .as_mut()
            .ok_or("Missing product formal type")?,
        token,
    )
}

fn normalize_binding(
    bindings: &mut [crate::BindingFact],
    formal: &HirRef,
    token: &str,
) -> Result<(), String> {
    let mut bindings: Vec<_> = bindings
        .iter_mut()
        .filter(|binding| binding.hir == *formal)
        .collect();
    let [binding] = bindings.as_mut_slice() else {
        return Err("Product formal binding inventory is missing or ambiguous".into());
    };

    generic_type(&mut binding.ty, token)
}

fn generic_type(ty: &mut crate::ResolvedTy, token: &str) -> Result<(), String> {
    if ty.normalized.is_some() || ty.observed != "S" {
        return Err("Product normalization may change only its original unresolved S type".into());
    }
    ty.normalized = Some(token.into());
    Ok(())
}

fn normalize_read(node: &mut HirNode, formal: &HirRef, token: &str) -> Result<usize, String> {
    let own = if node.res.as_ref().is_some_and(|resolution| {
        resolution.kind == "local" && resolution.target.as_ref() == Some(formal)
    }) {
        generic_type(
            node.ty
                .as_mut()
                .ok_or("Missing original generic read type")?,
            token,
        )?;
        1
    } else {
        0
    };

    node.children.iter_mut().try_fold(own, |count, child| {
        normalize_read(child, formal, token).map(|children| count + children)
    })
}
