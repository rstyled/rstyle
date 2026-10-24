//! Bounded IEEE scalar kernels: exact values, ordered binary64 work and narrowing.
pub(crate) mod control;
mod invocation;
mod syntax;
mod values;
pub use values::FloatValue;

use crate::{HirNode, ProfileId, frontend::normalized, unit_store::EvidenceFacts};
use serde_json::Value;

pub const CAPABILITY: &str = "rust.float-kernel-scalars";
pub(crate) const SUBTRACTION: &str = "rust.float-subtraction";
pub const RESOLUTION: &str = "resolved_float_literal";
pub(crate) const LITERAL: &str = "data.float-literal";
pub(crate) const CONSTANT: &str = "data.float-constant";
pub(crate) const BINARY: &str = "data.float-binary";
pub(crate) const NEGATE: &str = "data.float-negate";
pub(crate) const NARROW: &str = "data.float-narrow";
pub(crate) const KEY: &str = "rstyle.data:float-scalar";
const IDENTITY: &str = "rstyle.rust.float-scalar/1";

pub fn is_float(ty: &str) -> bool {
    width(ty).is_some()
}

pub fn width(ty: &str) -> Option<u32> {
    match ty {
        "f32" => Some(32),
        "f64" => Some(64),
        _ => None,
    }
}

pub(crate) fn uses(node: &HirNode) -> bool {
    observed_literal(node)
        || normalized(node).is_some_and(|ty| width(ty).is_some())
        || node
            .children
            .iter()
            .any(|child| normalized(child).is_some_and(|ty| width(ty).is_some()))
}

pub(crate) fn computation(node: &HirNode) -> bool {
    normalized(node).is_some_and(|ty| width(ty).is_some())
        && matches!(
            node.kind.as_str(),
            "expr_lit" | "expr_binary" | "expr_unary" | "expr_cast"
        )
        || node.kind == "expr_path"
            && node.res.as_ref().is_some_and(|res| {
                res.kind == crate::scalar_constants::RESOLUTION
                    && normalized(node).is_some_and(|ty| width(ty).is_some())
            })
}

pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<(), String> {
    if function(facts) {
        authority(facts)?;
    }
    Ok(())
}

fn authority(facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || crate::data_library::registry_for(facts).is_none()
    {
        return Err(
            "Float kernels require their registered policy and pinned compiler/target".into(),
        );
    }
    invocation::verify(facts)
}

pub(crate) fn verify_node(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !uses(node) && !original_literal(node, facts) {
        return Ok(());
    }
    authority(facts)?;

    if node.expansion.is_some() || node.is_unsafe_block {
        return Err("Float kernels require complete original safe source operations".into());
    }
    syntax::verify(node, facts)?;
    match node.kind.as_str() {
        "expr_lit" => literal(node, facts).map(|_| ()),
        "expr_path" if computation(node) => constant(node).map(|_| ()),
        "expr_path" => Ok(()),
        "expr_binary" => binary(node, facts),
        "expr_unary" => negate(node),
        "expr_cast" => narrow(node),
        // Existing call, control, binding and type verifiers remain responsible
        // for their complete generic semantics; this leaf authorizes no bypass.
        _ => Ok(()),
    }
}

pub(crate) fn literal(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<FloatValue, String> {
    let ty = normalized(node).ok_or("Missing float literal width")?;
    let source =
        crate::byte_literals::source(node, facts).ok_or("Missing original float literal")?;
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing compiler float literal bits")?;

    let captured: FloatValue = crate::scalar_constants::decode_record(resolution, RESOLUTION)?;
    let expected = FloatValue::parse(source, ty)?;
    if !node.children.is_empty() || node.lit.as_deref() != Some(source) || captured != expected {
        return Err("Float literal loses its original spelling, width or exact bits".into());
    }
    Ok(captured)
}

pub(crate) fn constant(node: &HirNode) -> Result<FloatValue, String> {
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing float constant resolution")?;
    let record = crate::scalar_constants::ScalarConstant::decode(resolution)?;
    if node.kind != "expr_path"
        || !node.children.is_empty()
        || normalized(node) != Some(record.ty.as_str())
    {
        return Err("Float constant loses its resolved scalar use".into());
    }
    values::constant(&record)
}

fn binary(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let [_, _] = node.children.as_slice() else {
        return Err("Float arithmetic requires two ordered saved operands".into());
    };
    if node
        .dispatch
        .as_ref()
        .is_some_and(|dispatch| dispatch.op == "-")
        && !crate::extraction_profile::facts_support(facts, SUBTRACTION)
    {
        return Err("Float subtraction requires its registered capability".into());
    }
    if !crate::builtin_scalars::matches(node, &["f64", "f64"], "f64", &["+", "*", "-"]) {
        return Err("Float kernels require registered builtin binary64 arithmetic".into());
    }
    Ok(())
}

pub(crate) fn negate(node: &HirNode) -> Result<(), String> {
    let [_] = node.children.as_slice() else {
        return Err("Float negation requires one saved operand".into());
    };
    let ty = normalized(node).ok_or("Missing float negation width")?;
    if width(ty).is_none() || !crate::builtin_scalars::matches(node, &[ty], ty, &["-"]) {
        return Err("Float negation requires an exact builtin width-preserving sign flip".into());
    }
    Ok(())
}

pub(crate) fn narrow(node: &HirNode) -> Result<(), String> {
    let [_] = node.children.as_slice() else {
        return Err("Float narrowing requires one saved operand".into());
    };
    if !crate::builtin_scalars::matches(node, &["f64"], "f32", &["as"]) {
        return Err("Float kernels support only builtin binary64-to-binary32 narrowing".into());
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct Semantics<'a> {
    identity: &'static str,
    result_type: Option<&'a str>,
    operation: &'a str,
    rounding: &'static str,
    contraction: &'static str,
    nan: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    bits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    overflow: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    underflow: Option<&'static str>,
}

pub(crate) fn semantics(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Value, String> {
    verify_node(node, facts)?;
    let base = Semantics {
        identity: IDENTITY,
        result_type: normalized(node),
        operation: "",
        rounding: "nearest-ties-even-per-operation",
        contraction: "forbidden",
        nan: "pinned-rust-preferred-quieted-or-unchanged;sign-unspecified",
        bits: None,
        overflow: None,
        underflow: None,
    };

    let record = match node.kind.as_str() {
        "expr_lit" => value_record(base, "literal", literal(node, facts)?),
        "expr_path" => value_record(base, "constant", constant(node)?),
        "expr_binary" => Semantics {
            operation: &node.dispatch.as_ref().ok_or("Missing float operator")?.op,
            ..base
        },
        "expr_unary" => Semantics {
            operation: "negate",
            rounding: "exact-sign-bit-toggle",
            nan: "payload-preserved;sign-toggled",
            ..base
        },
        "expr_cast" => Semantics {
            operation: "narrow-f64-f32",
            overflow: Some("signed-infinity"),
            underflow: Some("gradual-including-signed-zero"),
            nan: "pinned-rust-truncated-payload;never-infinity;sign-unspecified",
            ..base
        },
        _ => return Err("Expected a float scalar computation".into()),
    };
    serde_json::to_value(record).map_err(|error| error.to_string())
}

fn value_record<'a>(base: Semantics<'a>, operation: &'a str, value: FloatValue) -> Semantics<'a> {
    Semantics {
        operation,
        bits: Some(value.bits),
        rounding: "exact-captured-value",
        nan: "not-applicable-original-finite-literal",
        ..base
    }
}

pub(crate) fn verify_document_policy(
    policy: &ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        policy,
        CAPABILITY,
        || function(facts),
        "Float scalars require their registered document policy",
    )?;
    crate::extraction_profile::require_document(
        policy,
        SUBTRACTION,
        || contains_subtraction(&facts.body),
        "Float subtraction requires its registered document policy",
    )
}

fn contains_subtraction(node: &HirNode) -> bool {
    node.kind == "expr_binary"
        && uses(node)
        && node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.op == "-")
        || node.children.iter().any(contains_subtraction)
}

fn function(facts: &dyn EvidenceFacts) -> bool {
    facts.signature.params.iter().any(|parameter| {
        parameter
            .ty
            .normalized
            .as_deref()
            .is_some_and(|ty| width(ty).is_some())
    }) || facts
        .signature
        .return_ty
        .normalized
        .as_deref()
        .is_some_and(|ty| width(ty).is_some())
        || facts.bindings.iter().any(|binding| {
            binding
                .ty
                .normalized
                .as_deref()
                .is_some_and(|ty| width(ty).is_some())
        })
        || contains(&facts.body, facts)
}

fn contains(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    uses(node)
        || original_literal(node, facts)
        || node.children.iter().any(|child| contains(child, facts))
}

fn observed_literal(node: &HirNode) -> bool {
    node.res
        .as_ref()
        .is_some_and(|resolution| resolution.kind == RESOLUTION)
        || matches!(node.kind.as_str(), "expr_lit" | "pat_literal")
            && node.lit.as_deref().is_some_and(float_spelling)
}

fn float_spelling(source: &str) -> bool {
    (source
        .bytes()
        .any(|byte| matches!(byte, b'.' | b'e' | b'E'))
        || source.ends_with("f32")
        || source.ends_with("f64"))
        && syn::parse_str::<syn::LitFloat>(source).is_ok()
}

fn original_literal(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    matches!(node.kind.as_str(), "expr_lit" | "pat_literal")
        && crate::byte_literals::source(node, facts).is_some_and(float_spelling)
}

pub(crate) fn verify_constant(
    record: &crate::scalar_constants::ScalarConstant,
) -> Result<(), String> {
    values::constant(record).map(|_| ())
}

pub(crate) fn construct(node: &HirNode) -> Option<&'static str> {
    if !computation(node) {
        return None;
    }
    match node.kind.as_str() {
        "expr_lit" => Some(LITERAL),
        "expr_path" => Some(CONSTANT),
        "expr_binary" => Some(BINARY),
        "expr_unary" => Some(NEGATE),
        "expr_cast" => Some(NARROW),
        _ => None,
    }
}
