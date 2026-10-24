//! Exact two-operand builtin scalar equality and Debug; no user dispatch.
use super::*;

pub(super) fn verify(node: &HirNode, assertion: &Assertion, source: &str) -> Result<(), String> {
    let [left, right] = node.children.as_slice() else {
        return Err("Comparison assertion requires exactly two saved operands".into());
    };
    let scalar = normalized(left).ok_or("Missing comparison scalar type")?;
    if !crate::concrete_types::scalar(scalar)
        || scalar == "unit"
        || normalized(right) != Some(scalar)
        || right.span
            != assertion
                .right_span(source, node.span.start)
                .ok_or("Missing right source operand")?
        || assertion.diagnostic.is_some()
    {
        return Err(
            "Comparison assertion requires matching builtin scalar operands and no message".into(),
        );
    }
    Ok(())
}

impl Assertion {
    pub fn right_span(&self, source: &str, offset: u32) -> Option<ByteSpan> {
        let span = self.right.as_ref()?.span();
        Some(ByteSpan {
            start: offset + byte_offset(source, span.start()) as u32,
            end: offset + byte_offset(source, span.end()) as u32,
        })
    }
}
