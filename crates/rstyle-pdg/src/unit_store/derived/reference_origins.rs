//! Reuse successful source-origin queries only for exact owned source nodes.
use crate::{HirNode, data_origins::Origin, unit_store::EvidenceFacts};
use std::{cell::RefCell, collections::BTreeMap};

const MAX_RETAINED_STEPS: usize = 64 * 1024;
type Key = (usize, Vec<usize>);

#[derive(Default)]
struct Results {
    values: BTreeMap<Key, Vec<Origin>>,
    retained_steps: usize,
}

#[derive(Default)]
pub(super) struct Queries([RefCell<Results>; 2]);

impl Queries {
    pub(super) fn select(
        &self,
        node: &HirNode,
        selectors: &[usize],
        facts: &dyn EvidenceFacts,
    ) -> Result<Vec<Origin>, String> {
        // A caller can supply an edited clone with the same HIR identifiers.
        // Only exact nodes borrowed from this immutable function may reuse work.
        if !owns(&facts.body, node) {
            return crate::data_reference_locals::compute_origins(node, selectors, facts);
        }
        self.query(node, selectors, || {
            crate::data_reference_locals::compute_origins(node, selectors, facts)
        })
    }

    fn query(
        &self,
        node: &HirNode,
        selectors: &[usize],
        compute: impl FnOnce() -> Result<Vec<Origin>, String>,
    ) -> Result<Vec<Origin>, String> {
        // Addresses are handles inside this analysis only, never wire identity.
        let key = (std::ptr::from_ref(node).addr(), selectors.to_vec());
        let results = &self.0[usize::from(crate::verification_policy::is_generated())];
        if let Some(origins) = results.borrow().values.get(&key) {
            return Ok(origins.clone());
        }

        let origins = compute()?;
        results.borrow_mut().remember(key, &origins);
        Ok(origins)
    }
}

impl Results {
    fn remember(&mut self, key: Key, origins: &[Origin]) {
        let steps = origins
            .iter()
            .fold(key.1.len().saturating_add(1), |sum, origin| {
                sum.saturating_add(origin.path.len()).saturating_add(1)
            });
        if self.retained_steps.saturating_add(steps) <= MAX_RETAINED_STEPS {
            self.retained_steps += steps;
            self.values.insert(key, origins.to_vec());
        }
    }
}

fn owns(root: &HirNode, node: &HirNode) -> bool {
    std::ptr::eq(root, node) || root.children.iter().any(|child| owns(child, node))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn node() -> HirNode {
        serde_json::from_value(serde_json::json!({
            "hir": {"krate": 0, "owner": 1, "local": 2},
            "kind": "expr_path", "span": {"start": 0, "end": 1}, "snippet": "a",
            "ty": null, "res": null, "dispatch": null, "expansion": null,
            "binding": null, "binding_mut": null, "lit": null, "param_index": null,
            "children": []
        }))
        .unwrap()
    }

    #[test]
    fn origins_are_local_to_exact_nodes_selectors_analysis_and_policy() {
        let root = node();
        let copied = root.clone();
        assert!(owns(&root, &root));
        assert!(!owns(&root, &copied));
        let queries = Queries::default();
        let visits = Cell::new(0);
        let compute = || {
            visits.set(visits.get() + 1);
            Ok(vec![Origin {
                parameter: 7,
                path: Vec::new(),
            }])
        };
        assert_eq!(queries.query(&root, &[], compute).unwrap()[0].parameter, 7);
        queries
            .query(&root, &[], || panic!("already checked"))
            .unwrap();
        queries.query(&root, &[0], compute).unwrap();
        queries.query(&copied, &[], compute).unwrap();
        crate::verification_policy::generated(|| {
            queries.query(&root, &[], compute).unwrap();
        });
        Queries::default().query(&root, &[], compute).unwrap();
        assert_eq!(visits.get(), 5);
    }

    #[test]
    fn failed_interrupted_and_over_budget_queries_do_not_publish_results() {
        let root = node();
        let queries = Queries::default();
        assert!(
            queries
                .query(&root, &[], || Err("missing source".into()))
                .is_err()
        );
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            queries.query(&root, &[], || panic!("interrupted"))
        }));
        assert!(panic.is_err());
        assert!(queries.0[0].borrow().values.is_empty());
        queries.0[0].borrow_mut().retained_steps = MAX_RETAINED_STEPS;
        assert!(
            queries
                .query(&root, &[], || Ok(Vec::new()))
                .unwrap()
                .is_empty()
        );
        assert!(queries.0[0].borrow().values.is_empty());
        assert!(
            queries
                .query(&root, &[], || Err("recomputed".into()))
                .is_err()
        );
    }
}
