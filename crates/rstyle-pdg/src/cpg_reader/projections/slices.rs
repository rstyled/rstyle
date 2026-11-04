//! Shared-slice instance projections: each candidate that a complete same-unit call
//! instantiates at `u8`, walked over its read facts with the typed observations its
//! document records.

use crate::data_receivers as records;
use crate::frontend::{HirFnFacts, HirNode, HirRef};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) const KEY: &str = "rstyle.rust.shared-slice-instance-projections";

/// Each candidate's projection envelope, by its position among `facts`.
pub(super) fn projections<'f>(
    facts: &'f [HirFnFacts],
    recorded: impl Iterator<Item = (usize, &'f Value)>,
) -> Vec<(usize, Value)> {
    let receipts = receipts(facts);
    if receipts.is_empty() {
        return Vec::new();
    }
    recorded
        .filter_map(|(index, record)| {
            let original = &facts[index];
            let identity = original
                .build
                .frontend_options
                .get("definition_identity")?
                .as_str()?;
            let calls = receipts.get(identity)?;
            let outcome = match observation(original, record) {
                Ok(projection) => json!({"status": "observed", "projection": projection}),
                Err(reason) => json!({"status": "unsupported", "reason": reason}),
            };
            Some((index, json!({
                "identity": "rstyle.rust.shared-slice-instance-projections/1",
                "scope": "selected local u8 shared-slice original-owner type overlay; no obligation, source verification or admission authority",
                "obligations_verified": false, "call_receipts": calls, "outcome": outcome})))
        })
        .collect()
}

/// The complete same-unit `u8` call receipts of each requested definition.
pub fn receipts(facts: &[HirFnFacts]) -> BTreeMap<String, Vec<Value>> {
    let mut grouped: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for (identity, receipt) in facts.iter().flat_map(receipt_pairs) {
        grouped.entry(identity).or_default().push(receipt);
    }

    grouped
}

fn receipt_pairs(caller: &HirFnFacts) -> Vec<(String, Value)> {
    let options = &caller.build.frontend_options;
    let Some(inventory) = options.get("rstyle.rust.call-instances") else {
        return Vec::new();
    };
    if inventory.get("identity").and_then(Value::as_str)
        != Some("rstyle.rust.call-instance-observations/1")
        || inventory.get("complete") != Some(&Value::Bool(true))
    {
        return Vec::new();
    }
    let Some(calls) = inventory.get("calls").and_then(Value::as_array) else {
        return Vec::new();
    };

    calls
        .iter()
        .filter_map(|call| {
            let identity = receipt_identity(call)?;
            Some((
                identity.into(),
                json!({"caller": options.get("definition_identity"),
                       "caller_source_digest": caller.source.digest,
                       "caller_fn_span": caller.fn_span, "call": call}),
            ))
        })
        .collect()
}

fn receipt_identity(call: &Value) -> Option<&str> {
    let requested = call.get("requested")?;
    if call.get("status").and_then(Value::as_str) != Some("resolved")
        || call.get("expanded") != Some(&Value::Bool(false))
        || requested.get("local") != Some(&Value::Bool(true))
        || call.get("selected") != Some(requested)
        || !u8_arguments(call.get("requested_arguments"))
        || !u8_arguments(call.get("selected_arguments"))
    {
        return None;
    }

    requested.get("definition_identity").and_then(Value::as_str)
}

/// Whether a call's arguments are exactly the builtin `u8`, by structural identity.
pub fn u8_arguments(value: Option<&Value>) -> bool {
    let Some(arguments) = value.and_then(Value::as_array) else {
        return false;
    };
    matches!(arguments.as_slice(), [argument]
        if argument.get("kind").and_then(Value::as_str) == Some("type")
        && argument.get("identity_status").and_then(Value::as_str) == Some("known")
        && argument.get("identity") == Some(&json!({"kind":"builtin","name":"u8"})))
}

use super::recorded as get;

fn observation(original: &HirFnFacts, record: &Value) -> Result<Value, String> {
    if record.get("available") != Some(&Value::Bool(true)) {
        return Err("Selected instance original source context is unavailable".into());
    }
    crate::slice_length_instances::source::verify(&original.source.bytes)?;
    if original.bindings.len() > 256 {
        return Err("Shared-slice projection exceeds the original binding budget".into());
    }
    let lens = &record["lens"];
    if let Some(error) = lens.get("error").and_then(Value::as_str) {
        return Err(error.to_owned());
    }
    let walk = Walk {
        owner: original.body.hir.owner,
        lens,
    };
    let parameters = original
        .signature
        .params
        .iter()
        .map(|parameter| walk.typed(&parameter.pat.hir))
        .collect::<Result<Vec<_>, _>>()?;
    let bindings = original
        .bindings
        .iter()
        .map(|binding| walk.typed(&binding.hir))
        .collect::<Result<Vec<_>, _>>()?;
    let mut nodes = Vec::new();
    walk.nodes(&original.body, (0, &mut 4096), &mut nodes)?;
    let result = get(&lens["result"])?;
    let methods = walk.methods(original)?;

    Ok(
        json!({"definition": lens["definition"], "type_parameter": lens["type_parameter"],
              "owner": lens["owner"], "source_digest": original.source.digest,
              "fn_span": original.fn_span,
              "original_source_scan_complete": original.source_scan_complete,
              "arguments": lens["arguments"], "parameters": parameters, "result": result,
              "bindings": bindings, "nodes": nodes, "methods": methods}),
    )
}

struct Walk<'r> {
    owner: u32,
    lens: &'r Value,
}

impl Walk<'_> {
    fn typed(&self, reference: &HirRef) -> Result<Value, String> {
        if reference.krate != 0 {
            return Err("Shared-slice projection HIR is not local".into());
        }
        let key = format!("{}.{}", reference.owner, reference.local);
        let record = self.lens["nodes"]
            .get(&key)
            .ok_or("original typeck lacks the requested node type")?;

        get(record).cloned()
    }

    /// Each typed node of the body, depth first, within the frontend's budgets.
    fn nodes(
        &self,
        node: &HirNode,
        (depth, budget): (usize, &mut usize),
        records: &mut Vec<Value>,
    ) -> Result<(), String> {
        if *budget == 0 || depth >= 128 {
            return Err("Shared-slice projection exceeds original HIR node budget".into());
        }
        *budget -= 1;
        if node.ty.is_some() {
            records.push(self.typed(&node.hir)?);
        }
        node.children
            .iter()
            .try_for_each(|child| self.nodes(child, (depth + 1, &mut *budget), records))
    }

    /// The body's one method call, its receiver observations paired with the original
    /// method record.
    fn methods(&self, original: &HirFnFacts) -> Result<Vec<Value>, String> {
        let capture = original.build.frontend_options.data::<records::Capture>()?;
        let [method] = capture.methods.as_slice() else {
            return Err(
                "Shared-slice projection requires exactly one original method record".into(),
            );
        };
        if capture.identity != records::IDENTITY || count_methods(&original.body) != 1 {
            return Err("Shared-slice original method inventory disagrees".into());
        }
        if method.call.krate != 0 {
            return Err("Projected method HIR is not local".into());
        }
        if method.call.owner != self.owner {
            return Err("Projected method and original typeck owners differ".into());
        }
        let key = format!("{}.{}", method.call.owner, method.call.local);
        let record = self.lens["methods"]
            .get(&key)
            .ok_or("Projected method HIR is not an expression")?;

        Ok(vec![
            json!({"original": method, "concrete": concrete(record, method)?}),
        ])
    }
}

/// How many method calls a body holds.
pub fn count_methods(node: &HirNode) -> usize {
    usize::from(node.kind == "expr_method_call")
        + node.children.iter().map(count_methods).sum::<usize>()
}

fn concrete(record: &Value, method: &records::Method) -> Result<Value, String> {
    if record.get("method_call") != Some(&Value::Bool(true)) {
        return Err("Projected original method HIR kind disagrees".into());
    }
    if record["source_span"] != json!(method.source_span)
        || record["receiver"] != json!(method.receiver)
    {
        return Err("Projected original method source/receiver identity disagrees".into());
    }
    let observed = get(&record["concrete"])?;
    let receiver_type = get(&observed["receiver_type"])?;
    let adjusted_type = get(&observed["adjusted_type"])?;
    let adjustments = observed["adjustments"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if adjustments.len() != method.adjustments.len() || adjustments.len() > records::MAX_ADJUSTMENTS
    {
        return Err("Projected receiver adjustment inventory disagrees".into());
    }
    let adjustments = adjustments
        .iter()
        .zip(&method.adjustments)
        .map(|(adjustment, original)| {
            let agrees = matches!(
                (adjustment["kind"].as_str(), &original.kind),
                (
                    Some("builtin_dereference"),
                    records::Kind::BuiltinDereference
                ) | (Some("shared_borrow"), records::Kind::SharedBorrow)
            );
            if !agrees {
                return Err(
                    "Projected receiver requires original builtin/shared adjustment kinds"
                        .to_owned(),
                );
            }
            Ok(json!({"kind": original.kind, "target": get(&adjustment["target"])?}))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let signature_inputs = get(&observed["signature_inputs"])?;
    let signature_result = get(&observed["signature_result"])?;

    Ok(
        json!({"call": method.call, "source_span": method.source_span, "receiver": method.receiver,
              "receiver_type": receiver_type, "adjusted_type": adjusted_type,
              "adjustments": adjustments, "signature_inputs": signature_inputs,
              "signature_result": signature_result}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn concrete_request_uses_structural_identity_instead_of_display() {
        let mut arguments = json!([{
            "kind":"type", "identity_status":"known",
            "identity":{"kind":"builtin","name":"u8"},
            "value":"display is not authority", "display_only":true,
        }]);
        assert!(u8_arguments(Some(&arguments)));
        arguments[0]["value"] = json!("u8");
        arguments[0]["identity"]["name"] = json!("u16");
        assert!(!u8_arguments(Some(&arguments)));
        arguments[0]["identity"]["name"] = json!("u8");
        arguments[0]["identity_status"] = json!("unsupported");
        assert!(!u8_arguments(Some(&arguments)));
        assert!(!u8_arguments(Some(&json!([]))));
        assert!(!u8_arguments(Some(&json!([arguments[0], arguments[0]]))));
    }
}
