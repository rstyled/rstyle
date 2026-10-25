//! Exact whole-result digit forwarding; callee guards remain in their own graph.
use crate::{
    HirNode,
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    copy_tagged_values::call_wrappers::WholeCall,
    helper_calls::VerifiedDependency,
    unit_store::EvidenceFacts,
};

pub(crate) const CAPABILITY: &str = "rust.checked-digit-product-forwarders";

pub(crate) struct Checked<'a> {
    call: &'a HirNode,
    actual: &'a HirNode,
    formal: &'a HirNode,
    input: TypeTree,
    output: TypeTree,
    dependency: VerifiedDependency<'a>,
}

impl WholeCall for Checked<'_> {
    fn call(&self) -> &HirNode {
        self.call
    }
    fn actual(&self) -> &HirNode {
        self.actual
    }
    fn formal(&self) -> &HirNode {
        self.formal
    }
    fn input(&self) -> &TypeTree {
        &self.input
    }
    fn output(&self) -> &TypeTree {
        &self.output
    }
    fn permits_const(&self) -> bool {
        true
    }
}

impl Checked<'_> {
    pub(crate) fn dependency(&self) -> &VerifiedDependency<'_> {
        &self.dependency
    }
}

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && facts.body.kind == "expr_block"
        && matches!(facts.body.children.as_slice(), [call] if call.kind == "expr_call")
        && facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .and_then(|text| crate::data_types::parse(text).ok())
            .is_some_and(|ty| {
                matches!(ty, crate::data_types::TypeToken::Option(payload)
                if matches!(*payload, crate::data_types::TypeToken::ScalarProductInstance { .. }))
            })
}

pub(crate) fn source_shape(facts: &dyn EvidenceFacts) -> Result<(), String> {
    let source = syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    crate::copy_tagged_values::call_wrappers::source::shape(&source, true)
}

pub(crate) fn verify(facts: &dyn EvidenceFacts) -> Result<Option<Checked<'_>>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    facts.verify_source_inputs()?;
    crate::source_evidence::verify(facts)?;
    let call = &facts.body.children[0];
    let [actual] = call.children.as_slice() else {
        return Err("Digit forwarder requires exactly one saved argument".into());
    };
    let dependency = crate::helper_calls::verify_use(call, facts)?;
    let VerifiedDependency::Shared(view) = &dependency else {
        return Err("Digit forwarder requires its verified selected unit graph".into());
    };

    let callee = view.frontend();
    if !crate::helper_calls::digit_products::candidate(&callee) {
        return Err("Digit forwarder selected another helper family".into());
    }
    let interface = callee
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let [Observation::Captured { ty: input }] = interface.parameters.as_slice() else {
        return Err("Digit forwarder callee lacks its complete formal".into());
    };
    let Observation::Captured { ty: output } = &interface.result else {
        return Err("Digit forwarder callee lacks its complete result".into());
    };
    let checked = Checked {
        call,
        actual,
        formal: &facts
            .signature
            .params
            .first()
            .ok_or("Missing digit forwarder formal")?
            .pat,
        input: input.clone(),
        output: output.clone(),
        dependency,
    };
    crate::copy_tagged_values::call_wrappers::source::verify(facts, &checked)?;
    Ok(Some(checked))
}

pub(crate) fn permits_type(text: &str, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if !candidate(facts) || !text.contains("\"kind\":\"scalar_product_instance\"") {
        return Ok(false);
    }
    let Some(checked) = verify(facts)? else {
        return Ok(false);
    };
    Ok(
        crate::data_types::parse(text)?
            == crate::data_types::TypeToken::from_tree(checked.output()),
    )
}
