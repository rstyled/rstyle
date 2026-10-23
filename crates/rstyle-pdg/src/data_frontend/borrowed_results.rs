//! Complete verified operand boundaries for checked borrowed Result operations.
use crate::{HirNode, unit_store::EvidenceFacts};

pub(crate) fn operands<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Option<Result<Vec<&'a HirNode>, String>> {
    if crate::scalar_result_propagation::candidate(node, facts) {
        return Some(
            crate::scalar_result_propagation::verify(node, facts)
                .map(|checked| vec![checked.operand]),
        );
    }
    if crate::checked_slice_views::suffix::candidate(node, facts) {
        return Some(
            crate::checked_slice_views::suffix::verify(node, facts)
                .map(|checked| vec![checked.receiver, checked.start]),
        );
    }
    if crate::borrowed_result_propagation::candidate(node, facts) {
        return Some(
            crate::borrowed_result_propagation::verify(node, facts)
                .map(|checked| vec![checked.operand]),
        );
    }

    if crate::checked_slice_views::candidate(node, facts) {
        return Some(
            crate::checked_slice_views::verify(node, facts)
                .map(|checked| vec![checked.receiver, checked.end]),
        );
    }

    if crate::borrowed_result_errors::candidate(node, facts) {
        return Some(
            crate::borrowed_result_errors::verify(node, facts)
                .map(|checked| vec![checked.receiver, checked.error]),
        );
    }

    if crate::borrowed_result_errors::error_candidate(node, facts) {
        return Some(crate::borrowed_result_errors::verify_error(node, facts).map(|_| Vec::new()));
    }

    None
}

pub(super) fn used(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    crate::scalar_result_propagation::candidate(node, facts)
        || crate::checked_slice_views::suffix::candidate(node, facts)
        || crate::borrowed_result_propagation::candidate(node, facts)
        || crate::checked_slice_views::candidate(node, facts)
        || crate::borrowed_result_errors::candidate(node, facts)
        || crate::borrowed_result_errors::error_candidate(node, facts)
        || node.children.iter().any(|child| used(child, facts))
}
