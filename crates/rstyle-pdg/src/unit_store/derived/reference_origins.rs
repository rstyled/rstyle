//! Reuse successful source-origin queries only for exact owned source nodes.
use crate::data_reference_locals::OwnedIndex;
use crate::{HirNode, data_origins::Origin, unit_store::EvidenceFacts};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

const MAX_RETAINED_STEPS: usize = 64 * 1024;
type Key = (usize, usize, Vec<usize>);

#[derive(Default)]
struct Results {
    values: BTreeMap<Key, Vec<Origin>>,
    retained_steps: usize,
    index: Option<Rc<OwnedIndex>>,
}

#[derive(Default)]
pub(in crate::unit_store) struct Queries([RefCell<Results>; 2]);

#[derive(Clone, Copy)]
struct Request<'a> {
    body: &'a HirNode,
    node: &'a HirNode,
    selectors: &'a [usize],
}

impl Queries {
    pub(in crate::unit_store) fn select(
        &self,
        node: &HirNode,
        selectors: &[usize],
        facts: &dyn EvidenceFacts,
    ) -> Result<Vec<Origin>, String> {
        self.indexed(
            Request {
                body: &facts.body,
                node,
                selectors,
            },
            || OwnedIndex::new(facts),
            |index| crate::data_reference_locals::indexed_origins(node, selectors, facts, index),
        )
    }

    fn indexed(
        &self,
        request: Request<'_>,
        build: impl FnOnce() -> Result<OwnedIndex, String>,
        compute: impl FnOnce(&OwnedIndex) -> Result<Vec<Origin>, String>,
    ) -> Result<Vec<Origin>, String> {
        if let Some(origins) = self.cached(request) {
            return Ok(origins);
        }
        let Request { body, node, .. } = request;
        let results = &self.0[usize::from(crate::verification_policy::is_generated())];
        let retained = results.borrow().index.clone();
        let index = match retained {
            Some(index) if index.owns(body, node) => index,
            Some(_) => return compute(&build()?),
            None => Rc::new(build()?),
        };
        // Edited clones never inherit either results or owned source handles.
        if !index.owns(body, node) {
            return compute(&index);
        }

        let origins = self.query(request, || compute(&index))?;
        results.borrow_mut().remember_index(index);
        Ok(origins)
    }

    // Entries are published only after exact immutable body/node membership proof.
    fn cached(&self, request: Request<'_>) -> Option<Vec<Origin>> {
        let key = (
            std::ptr::from_ref(request.body).addr(),
            std::ptr::from_ref(request.node).addr(),
            request.selectors.to_vec(),
        );
        self.0[usize::from(crate::verification_policy::is_generated())]
            .borrow()
            .values
            .get(&key)
            .cloned()
    }

    fn query(
        &self,
        request: Request<'_>,
        compute: impl FnOnce() -> Result<Vec<Origin>, String>,
    ) -> Result<Vec<Origin>, String> {
        // Addresses are handles inside this analysis only, never wire identity.
        let key = (
            std::ptr::from_ref(request.body).addr(),
            std::ptr::from_ref(request.node).addr(),
            request.selectors.to_vec(),
        );
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
    fn remember_index(&mut self, index: Rc<OwnedIndex>) {
        if self.index.is_some() {
            return;
        }
        let steps = index.retained_steps();
        if self.retained_steps.saturating_add(steps) <= MAX_RETAINED_STEPS {
            self.retained_steps += steps;
            self.index = Some(index);
        }
    }

    fn remember(&mut self, key: Key, origins: &[Origin]) {
        let steps = origins
            .iter()
            .fold(key.2.len().saturating_add(2), |sum, origin| {
                sum.saturating_add(origin.path.len()).saturating_add(1)
            });
        if self.retained_steps.saturating_add(steps) <= MAX_RETAINED_STEPS {
            self.retained_steps += steps;
            self.values.insert(key, origins.to_vec());
        }
    }
}

#[cfg(test)]
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

    fn request<'a>(root: &'a HirNode, selectors: &'a [usize]) -> Request<'a> {
        Request {
            body: root,
            node: root,
            selectors,
        }
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
        assert_eq!(
            queries.query(request(&root, &[]), compute).unwrap()[0].parameter,
            7
        );
        queries
            .query(request(&root, &[]), || panic!("already checked"))
            .unwrap();
        queries.query(request(&root, &[0]), compute).unwrap();
        queries.query(request(&copied, &[]), compute).unwrap();
        crate::verification_policy::generated(|| {
            queries.query(request(&root, &[]), compute).unwrap();
        });
        Queries::default()
            .query(request(&root, &[]), compute)
            .unwrap();
        assert_eq!(visits.get(), 5);
    }

    #[test]
    fn failed_interrupted_and_over_budget_queries_do_not_publish_results() {
        let root = node();
        let queries = Queries::default();
        assert!(
            queries
                .query(request(&root, &[]), || Err("missing source".into()))
                .is_err()
        );
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            queries.query(request(&root, &[]), || panic!("interrupted"))
        }));
        assert!(panic.is_err());
        assert!(queries.0[0].borrow().values.is_empty());
        queries.0[0].borrow_mut().retained_steps = MAX_RETAINED_STEPS;
        assert!(
            queries
                .query(request(&root, &[]), || Ok(Vec::new()))
                .unwrap()
                .is_empty()
        );
        assert!(queries.0[0].borrow().values.is_empty());
        assert!(
            queries
                .query(request(&root, &[]), || Err("recomputed".into()))
                .is_err()
        );
    }
    #[test]
    fn distinct_queries_build_one_owned_flow_index_per_owner_and_policy() {
        for count in [16, 64, 256] {
            let mut root = node();
            root.children = (0..count)
                .map(|position| {
                    let mut child = node();
                    child.hir.local = position + 10;
                    child
                })
                .collect();
            let queries = Queries::default();
            let builds = Cell::new(0);
            let build = || {
                builds.set(builds.get() + 1);
                let index = OwnedIndex::for_body(&root);
                assert_eq!(index.retained_steps(), count as usize + 1);
                Ok(index)
            };
            for child in &root.children {
                queries
                    .indexed(
                        Request {
                            body: &root,
                            node: child,
                            selectors: &[],
                        },
                        build,
                        |_| Ok(Vec::new()),
                    )
                    .unwrap();
                queries
                    .indexed(
                        Request {
                            body: &root,
                            node: child,
                            selectors: &[],
                        },
                        || panic!("retained index"),
                        |_| panic!("retained origins"),
                    )
                    .unwrap();
            }
            assert_eq!(builds.get(), 1);
            check_policy_and_owner(&queries, &root, &builds, build);
        }
    }

    fn check_policy_and_owner(
        queries: &Queries,
        root: &HirNode,
        builds: &Cell<usize>,
        build: impl Fn() -> Result<OwnedIndex, String> + Copy,
    ) {
        crate::verification_policy::generated(|| {
            queries
                .indexed(
                    Request {
                        body: root,
                        node: &root.children[0],
                        selectors: &[],
                    },
                    build,
                    |_| Ok(Vec::new()),
                )
                .unwrap();
        });
        assert_eq!(builds.get(), 2);
        check_edited_owner(queries, root, builds, build);
        Queries::default()
            .indexed(
                Request {
                    body: root,
                    node: &root.children[0],
                    selectors: &[],
                },
                build,
                |_| Ok(Vec::new()),
            )
            .unwrap();
        assert_eq!(builds.get(), 4);
    }

    fn check_edited_owner(
        queries: &Queries,
        root: &HirNode,
        builds: &Cell<usize>,
        build: impl Fn() -> Result<OwnedIndex, String>,
    ) {
        let edited = root.children[0].clone();
        let roots = queries
            .indexed(
                Request {
                    body: root,
                    node: &edited,
                    selectors: &[],
                },
                build,
                |_| {
                    Ok(vec![Origin {
                        parameter: 99,
                        path: Vec::new(),
                    }])
                },
            )
            .unwrap();
        assert_eq!(roots[0].parameter, 99);
        assert_eq!(builds.get(), 3);
    }

    #[test]
    fn retained_origins_skip_unretained_index_builds() {
        let mut root = node();
        root.children = vec![node(); 256];
        let queries = Queries::default();
        queries.0[0].borrow_mut().retained_steps = MAX_RETAINED_STEPS - 128;
        queries
            .indexed(
                request(&root, &[]),
                || Ok(OwnedIndex::for_body(&root)),
                |_| Ok(Vec::new()),
            )
            .unwrap();
        assert!(queries.0[0].borrow().index.is_none());
        assert_eq!(queries.0[0].borrow().values.len(), 1);
        queries
            .indexed(
                request(&root, &[]),
                || panic!("exact certified origin hit rebuilt flow"),
                |_| panic!("exact certified origin hit recomputed"),
            )
            .unwrap();
        check_foreign_requests(&queries, &root);
    }

    fn check_foreign_requests(queries: &Queries, root: &HirNode) {
        let edited = root.clone();
        for request in [
            Request {
                body: root,
                node: &edited,
                selectors: &[],
            },
            Request {
                body: &edited,
                node: root,
                selectors: &[],
            },
        ] {
            for _ in 0..2 {
                assert!(
                    queries
                        .indexed(
                            request,
                            || Err("foreign query must build fresh evidence".into()),
                            |_| panic!("missing evidence must fail"),
                        )
                        .is_err()
                );
                queries
                    .indexed(
                        request,
                        || Ok(OwnedIndex::for_body(request.body)),
                        |_| Ok(Vec::new()),
                    )
                    .unwrap();
            }
        }
        assert_eq!(queries.0[0].borrow().values.len(), 1);
    }

    #[test]
    fn failed_and_over_budget_index_queries_publish_no_index() {
        let root = node();
        let queries = Queries::default();
        assert!(
            queries
                .indexed(
                    Request {
                        body: &root,
                        node: &root,
                        selectors: &[]
                    },
                    || Ok(OwnedIndex::for_body(&root)),
                    |_| Err("source evidence missing".into())
                )
                .is_err()
        );
        assert!(queries.0[0].borrow().index.is_none());
        assert!(queries.0[0].borrow().values.is_empty());
        queries.0[0].borrow_mut().retained_steps = MAX_RETAINED_STEPS;
        queries
            .indexed(
                Request {
                    body: &root,
                    node: &root,
                    selectors: &[],
                },
                || Ok(OwnedIndex::for_body(&root)),
                |_| Ok(Vec::new()),
            )
            .unwrap();
        assert!(queries.0[0].borrow().index.is_none());
        assert!(queries.0[0].borrow().values.is_empty());
    }
}
