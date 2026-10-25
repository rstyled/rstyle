//! Exact immediate Option callback selection and complete separately owned body.
use crate::structural_json::object;
use crate::{
    HirNode, generated_sources::capture::Definition, parsed_item_callbacks,
    scalar_product_instances::Observation, unit_store::EvidenceFacts,
};
use serde_json::{Value, json};

pub(crate) struct Calls<'a> {
    pub outer: &'a HirNode,
    pub parser: &'a HirNode,
    pub callback: &'a HirNode,
    pub nested: &'a Value,
}

pub(super) fn calls(facts: &dyn EvidenceFacts) -> Result<Calls<'_>, String> {
    let [outer] = facts.body.children.as_slice() else {
        return Err("Nonzero composition requires its complete one-expression body".into());
    };

    let [parser, callback] = outer.children.as_slice() else {
        return Err("Nonzero composition requires one parser and one immediate callback".into());
    };

    if outer.kind != "expr_method_call"
        || parser.kind != "expr_call"
        || callback.kind != "expr_closure"
        || !callback.children.is_empty()
        || [outer, parser, callback]
            .iter()
            .any(|node| node.expansion.is_some() || node.is_unsafe_block)
    {
        return Err("Nonzero composition changes its original call/closure ownership".into());
    }

    let nested = callback_scope(facts, callback)?;

    Ok(Calls {
        outer,
        parser,
        callback,
        nested,
    })
}

fn callback_scope<'a>(
    facts: &'a dyn EvidenceFacts,
    callback: &HirNode,
) -> Result<&'a Value, String> {
    let inventory = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Missing nonzero composition call inventory")?;

    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
    {
        return Err("Nonzero composition call inventory is incomplete".into());
    }

    let [scoped] = inventory["callbacks"]
        .as_array()
        .ok_or("Missing callback inventory")?
        .as_slice()
    else {
        return Err("Nonzero composition requires exactly one separately owned callback".into());
    };

    if scoped["hir"] != json!(callback.hir)
        || scoped["span"] != json!(callback.span)
        || scoped["capture_count"] != 0
        || scoped["scope"]
            != "immediate-core-option-and-then-callback-owning-body; nested owners excluded"
    {
        return Err("Nonzero composition changes its exact empty callback environment".into());
    }

    let [nested] = scoped["calls"]
        .as_array()
        .ok_or("Missing callback calls")?
        .as_slice()
    else {
        return Err("Nonzero callback requires one complete original member call".into());
    };

    Ok(nested)
}

pub(super) fn verify(
    facts: &dyn EvidenceFacts,
    calls: &Calls<'_>,
    input: &Observation,
    output: &Observation,
) -> Result<parsed_item_callbacks::Checked, String> {
    let inventory = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .unwrap();

    let [_] = inventory["calls"]
        .as_array()
        .ok_or("Missing root calls")?
        .as_slice()
    else {
        return Err("Nonzero composition requires exactly its one generic root call".into());
    };

    let outer = unique(&inventory["calls"], calls.outer)?;
    option_method(outer, calls.outer)?;

    let arguments = substitutions(outer)?;

    let product: Definition = decode(&arguments[0]["identity"]["definition"])?;

    if !product.local
        || product.path != input.nominal.definition_path
        || product.definition_identity.is_empty()
        || product.stable_crate_id.is_empty()
    {
        return Err(
            "Nonzero callback product differs from its checked original declaration".into(),
        );
    }

    verify_endpoints(facts, calls, input, &product)?;

    let receipt = member(facts, calls, &product, output)?;

    if receipt.argument() != input.argument {
        return Err("Callback scalar substitution differs from the verified parser result".into());
    }

    Ok(receipt)
}

fn verify_endpoints(
    facts: &dyn EvidenceFacts,
    calls: &Calls<'_>,
    input: &Observation,
    product: &Definition,
) -> Result<(), String> {
    let inventory = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .unwrap();
    let outer = unique(&inventory["calls"], calls.outer)?;
    let arguments = substitutions(outer)?;

    let scalar = object([
        ("kind", json!("builtin")),
        ("name", json!(input.argument.as_str())),
    ]);

    let nonzero = &calls.nested["selected_arguments"][2]["identity"];

    let expected_input = product_type(product, scalar);

    let expected_output = product_type(product, nonzero.clone());

    if arguments[0]["identity"] != expected_input || arguments[1]["identity"] != expected_output {
        return Err("Option callback changes its exact input/output product substitutions".into());
    }

    let scoped = &inventory["callbacks"][0];
    closure(
        &arguments[2],
        scoped,
        &Endpoints {
            input: expected_input,
            output: expected_output,
        },
        facts,
    )
}

fn member(
    facts: &dyn EvidenceFacts,
    calls: &Calls<'_>,
    product: &Definition,
    output: &Observation,
) -> Result<parsed_item_callbacks::Checked, String> {
    let nonzero = &calls.nested["selected_arguments"][2]["identity"];

    let member: Definition = decode(&calls.nested["selected"])?;

    if member.path != format!("{}::<'a, T>::flat_map", product.path) {
        return Err("Callback selects another original product member".into());
    }

    let constructor = &calls.nested["selected_arguments"][3]["identity"];

    let context = parsed_item_callbacks::Context {
        member: &member,
        product,
        option: &decode(&constructor["signature"]["output"]["definition"])?,
        nonzero: &decode(&nonzero["definition"])?,
        constructor: &decode(&constructor["definition"])?,
    };

    let receipt = parsed_item_callbacks::verify_in(facts, calls.nested, &context)?;
    member_origin(facts, &member, output)?;

    Ok(receipt)
}

fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

fn unique<'a>(calls: &'a Value, node: &HirNode) -> Result<&'a Value, String> {
    let mut matches = calls
        .as_array()
        .ok_or("Missing original calls")?
        .iter()
        .filter(|call| call["hir"] == json!(node.hir));

    let found = matches.next().ok_or("Missing exact original call")?;

    if matches.next().is_some() || found["span"] != json!(node.span) {
        return Err("Ambiguous original call or mismatched source span".into());
    }

    Ok(found)
}

fn substitutions(call: &Value) -> Result<&[Value], String> {
    let requested = call["requested_arguments"]
        .as_array()
        .ok_or("Missing requested substitutions")?;

    let selected = call["selected_arguments"]
        .as_array()
        .ok_or("Missing selected substitutions")?;

    if call["status"] != "resolved"
        || call["expanded"] != false
        || requested.len() != 3
        || selected.len() != 3
        || requested.iter().zip(selected).any(|(left, right)| {
            left["kind"] != "type"
                || right["kind"] != "type"
                || left["identity_status"] != "known"
                || right["identity_status"] != "known"
                || left["identity"] != right["identity"]
        })
    {
        return Err("Option callback requires all three exact concrete substitutions".into());
    }

    Ok(selected)
}

// The pinned core Option::and_then matches self, invokes f once only on Some,
// and returns None otherwise. Exact empty closure/function-item environments and
// separately proved no-drop product fields discharge cleanup on both branches.
fn option_method(call: &Value, node: &HirNode) -> Result<(), String> {
    let path = call["selected"]["path"]
        .as_str()
        .ok_or("Missing Option method path")?;

    if !matches!(
        path,
        "core::option::Option::<T>::and_then" | "std::option::Option::<T>::and_then"
    ) {
        return Err("Option callback selects another library method".into());
    }

    let identity = object([
        ("crate", json!("core")),
        ("local", json!(false)),
        ("path", json!(path)),
        (
            "stable_crate_id",
            json!("StableCrateId(5614226571623188065)"),
        ),
        (
            "definition_identity",
            json!("DefPathHash(Fingerprint(5614226571623188065, 6372046201973383046))"),
        ),
    ]);

    let resolution = object([
        ("crate", json!("core")),
        ("crate_hash", json!("bdb4eda0487b663ec13960d84fc76cc8")),
        ("path", json!(path)),
        (
            "definition_hash",
            json!("DefPathHash(Fingerprint(5614226571623188065, 6372046201973383046))"),
        ),
    ]);

    let resolved = node
        .res
        .as_ref()
        .ok_or("Missing Option method resolution")?;

    if call["requested"] != identity
        || call["selected"] != identity
        || resolved.kind != "resolved_callee"
        || resolved.target.is_some()
    {
        return Err("Option callback lacks its exact pinned core method".into());
    }
    method_signature(node, path, &resolution)
}

fn method_signature(node: &HirNode, path: &str, resolution: &Value) -> Result<(), String> {
    let resolved = node
        .res
        .as_ref()
        .ok_or("Missing Option method resolution")?;

    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing Option method dispatch")?;

    let actual: Value =
        serde_json::from_str(&resolved.description).map_err(|error| error.to_string())?;

    if &actual != resolution
        || dispatch.kind != "resolved_method"
        || dispatch.op != path
        || dispatch.operand_tys.len() != 2
        || dispatch
            .operand_tys
            .iter()
            .zip(&node.children)
            .any(|(ty, node)| {
                node.ty
                    .as_ref()
                    .is_none_or(|observed| &observed.observed != ty)
            })
        || node
            .ty
            .as_ref()
            .is_none_or(|ty| ty.observed != dispatch.result_ty)
    {
        return Err("Option callback method signature or selected core authority disagrees".into());
    }

    Ok(())
}

fn product_type(product: &Definition, payload: Value) -> Value {
    object([
        ("kind", json!("adt")),
        ("definition", json!(product)),
        ("arguments", json!([{"kind":"erased_region"},payload])),
    ])
}

struct Endpoints {
    input: Value,
    output: Value,
}

fn closure(
    argument: &Value,
    scoped: &Value,
    endpoints: &Endpoints,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let input = &endpoints.input;

    let definition: Definition = decode(&scoped["definition"])?;

    let product: Definition = decode(&input["definition"])?;

    if !definition.local
        || definition.crate_name != product.crate_name
        || definition.stable_crate_id != product.stable_crate_id
        || definition.path != format!("{}::{{closure#0}}", facts.name)
        || definition.definition_identity.is_empty()
    {
        return Err("Callback closure belongs to another original caller/build".into());
    }

    let expected = closure_type(&definition, scoped, endpoints);

    let [captured] = scoped["callable"]
        .as_array()
        .ok_or("Missing callback callable type")?
        .as_slice()
    else {
        return Err("Callback requires exactly one structural callable type".into());
    };

    if argument["identity"] != expected
        || captured["identity"] != expected
        || captured["identity_status"] != "known"
        || captured["kind"] != "type"
    {
        return Err(
            "Callback closure changes its signature, captured storage or instance identity".into(),
        );
    }

    Ok(())
}

fn closure_type(definition: &Definition, scoped: &Value, endpoints: &Endpoints) -> Value {
    object([
        ("kind", json!("noncapturing_closure")),
        ("definition", json!(definition)),
        ("capture_count", json!(0)),
        ("closure_kind", json!("FnOnce")),
        ("parent_arguments", json!([])),
        (
            "upvars",
            object([("kind", json!("tuple")), ("elements", json!([]))]),
        ),
        ("signature", closure_signature(scoped, endpoints)),
    ])
}
fn closure_signature(scoped: &Value, endpoints: &Endpoints) -> Value {
    let input = &endpoints.input;
    let output = &endpoints.output;

    object([
        ("abi", json!("\"rust-call\"")),
        ("bound_variables", json!(0)),
        ("safety", json!("Safe")),
        ("variadic", json!(false)),
        ("inputs", json!([{"kind":"tuple", "elements":[input]}])),
        (
            "output",
            object([
                ("kind", json!("adt")),
                (
                    "definition",
                    json!(
                        scoped["calls"][0]["selected_arguments"][3]["identity"]["signature"]["output"]
                            ["definition"]
                    ),
                ),
                ("arguments", json!([output])),
            ]),
        ),
    ])
}

fn member_origin(
    facts: &dyn EvidenceFacts,
    member: &Definition,
    output: &Observation,
) -> Result<(), String> {
    let original = facts
        .original_local_member(&member.definition_identity)?
        .ok_or("Missing original member")?;

    let origins = facts
        .build
        .frontend_options
        .data::<crate::declaration_origins::Origins>()?;

    let selected = origins
        .declarations
        .iter()
        .filter(|origin| origin.nominal == output.nominal)
        .collect::<Vec<_>>();

    let [origin] = selected.as_slice() else {
        return Err("Callback product requires its exact original declaration file".into());
    };

    if original
        .build
        .frontend_options
        .get("source_file")
        .and_then(Value::as_str)
        != Some(origin.source_file.as_str())
    {
        return Err(
            "Callback member and checked original declaration have different source ownership"
                .into(),
        );
    }

    Ok(())
}
