//! Conditional Ok-payload origins of the bounded borrowed conversion.
use super::*;
use crate::data_origins::Origin;

pub(crate) fn source_origins(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    mut resolve: impl FnMut(&HirNode) -> Result<Vec<Origin>, String>,
) -> Option<Result<Vec<Origin>, String>> {
    let kind = kind(node)?;
    if !matches!(kind, Kind::Borrow | Kind::UnwrapBorrow) {
        return None;
    }
    Some((|| {
        verify(node, facts)?;
        let operand = &node.children[0];
        let base = if kind == Kind::Borrow && crate::data_subslices::candidate(operand, facts) {
            crate::data_subslices::verify(operand, facts)?.base
        } else {
            operand
        };
        Ok(project(kind, resolve(base)?))
    })())
}

pub(crate) fn project(kind: Kind, mut roots: Vec<Origin>) -> Vec<Origin> {
    if kind == Kind::Borrow {
        for origin in &mut roots {
            origin
                .path
                .push(crate::field_locations::Projection::ByteWindow {
                    operand_position: 0,
                    length: 8,
                });
        }
    }
    roots
}
