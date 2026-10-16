//! Pair-local trace observations from the scoring pass, never another refinement.
use super::{PreparedFunction, graph_of};
use crate::wl::{LabelNames, LabelPartition, WlRefinement, refine_compact, refine_observed};

pub(super) struct Refined {
    pub refinement: WlRefinement,
    pub partitions: Option<Vec<LabelPartition>>,
}

pub(super) fn refine(
    first: &PreparedFunction<'_>,
    second: &PreparedFunction<'_>,
    depth: usize,
) -> Refined {
    let graph_a = graph_of(first);
    let graph_b = graph_of(second);
    let names = first
        .projection
        .as_ref()
        .ok()
        .and_then(|data| data.trace_names.as_deref())
        .zip(
            second
                .projection
                .as_ref()
                .ok()
                .and_then(|data| data.trace_names.as_deref()),
        );
    if let Some((first, second)) = names {
        let names = LabelNames { first, second };
        let mut partitions = Vec::new();
        let refinement = refine_observed(graph_a, graph_b, depth, |first, second| {
            partitions.push(names.partition(first, second));
        });
        Refined {
            refinement,
            partitions: Some(partitions),
        }
    } else {
        Refined {
            refinement: refine_compact(graph_a, graph_b, depth),
            partitions: None,
        }
    }
}
