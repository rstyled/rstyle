//! Join independently checked parser, caller cleanup and consuming callback evidence.
use super::{selection, source};
use crate::{
    HirNode,
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    helper_calls::VerifiedDependency,
    scalar_product_instances,
    unit_store::EvidenceFacts,
};

pub(crate) struct Checked<'a> {
    pub calls: selection::Calls<'a>,
    pub input: scalar_product_instances::Observation,
    pub output: scalar_product_instances::Observation,
    pub dependency: VerifiedDependency<'a>,
}

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<Checked<'_>, String> {
    facts.verify_source_inputs()?;

    let calls = selection::calls(facts)?;

    let output = result(facts)?;
    source::shape(facts, output.argument)?;
    source::correspondence(facts, &calls)?;

    let resolution = facts.nominal_resolution()?;

    if !resolution.checked_scalar_product(
        facts.facts(),
        &TypeTree::ScalarProductInstance(Box::new(output.clone())),
    )? {
        return Err("Nonzero caller lacks its exact output cleanup/source authority".into());
    }

    let dependency = crate::helper_calls::verify_use(calls.parser, facts)?;

    let VerifiedDependency::Shared(view) = &dependency else {
        return Err("Nonzero composition requires its complete same-unit parser graph".into());
    };

    let callee = view.frontend();

    let checked = scalar_product_instances::activation::verify(&callee)?;

    if !crate::helper_calls::digit_products::candidate(&callee)
        || dependency.summary().component_outputs.is_none()
        || dependency.summary().memory_reads.is_none()
    {
        return Err("Nonzero composition lacks its complete guarded parser summary".into());
    }

    let input = checked.instance().clone();
    scalar_product_instances::transformation::verify(&input, &output, facts)?;
    selection::verify(facts, &calls, &input, &output)?;

    let member_identity = calls.nested["selected"]["definition_identity"]
        .as_str()
        .ok_or("Missing selected callback member")?;

    let member = facts
        .original_local_member(member_identity)?
        .ok_or("Missing original callback member")?;
    resolution.verify_nonzero_member_scope(member, &output.nominal)?;
    endpoints(facts, &calls, &input)?;

    Ok(Checked {
        calls,
        input,
        output,
        dependency,
    })
}

fn result(facts: &dyn EvidenceFacts) -> Result<scalar_product_instances::Observation, String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    interface.validate()?;

    let [Observation::Captured { ty }] = interface.parameters.as_slice() else {
        return Err("Nonzero caller lacks its complete formal type".into());
    };

    if !crate::concrete_types::byte_views::observed(ty) {
        return Err("Nonzero composition requires one shared byte-view formal".into());
    }

    let Observation::Captured {
        ty: TypeTree::Option(payload),
    } = &interface.result
    else {
        return Err("Nonzero composition requires its complete Option result".into());
    };

    let TypeTree::ScalarProductInstance(instance) = payload.as_ref() else {
        return Err("Nonzero composition requires its concrete product result".into());
    };
    instance.check_shape()?;

    Ok(instance.as_ref().clone())
}

impl Checked<'_> {
    pub(crate) fn parser(&self) -> &HirNode {
        self.calls.parser
    }
}

fn endpoints(
    facts: &dyn EvidenceFacts,
    calls: &selection::Calls<'_>,
    input: &scalar_product_instances::Observation,
) -> Result<(), String> {
    let [actual] = calls.parser.children.as_slice() else {
        return Err("Nonzero parser requires exactly one saved input".into());
    };

    let [formal] = facts.signature.params.as_slice() else {
        return Err("Nonzero caller requires one complete formal".into());
    };
    source::binding(facts, actual, formal)?;

    let input_token = crate::data_types::TypeToken::from_tree(&TypeTree::Option(Box::new(
        TypeTree::ScalarProductInstance(Box::new(input.clone())),
    )))
    .text()?;

    if calls
        .parser
        .ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        != Some(input_token.as_str())
        || calls.outer.ty != facts.body.ty
        || calls
            .outer
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_deref())
            != facts.signature.return_ty.normalized.as_deref()
    {
        return Err("Nonzero composition changes its complete resolved endpoints".into());
    }

    Ok(())
}
