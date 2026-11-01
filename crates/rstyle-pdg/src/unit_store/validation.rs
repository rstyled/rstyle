//! Successful producer checks belong to one immutable graph owner and policy.
use super::*;
use crate::producer::{ValidatedIndexes, ValidatedPair};
use std::sync::Mutex;

#[derive(Default)]
pub(super) struct Cache(Mutex<BTreeMap<(GraphId, bool), Arc<ValidatedIndexes>>>);

impl Cache {
    fn get(&self, graph: GraphId) -> Option<Arc<ValidatedIndexes>> {
        self.0
            .lock()
            .expect("unit validation cache poisoned")
            .get(&(graph, crate::verification_policy::is_generated()))
            .cloned()
    }

    fn insert(&self, graph: GraphId, indexes: Arc<ValidatedIndexes>) {
        self.0
            .lock()
            .expect("unit validation cache poisoned")
            .insert((graph, crate::verification_policy::is_generated()), indexes);
    }
}

impl FunctionView<'_> {
    /// Verify this function's own immutable root, reusing successful checks
    /// across analyses of its exact owning store under the same policy.
    /// Independent or modified document pairs must use `producer::verify`.
    pub fn verified_pair(&self) -> Result<ValidatedPair<'_>, Vec<String>> {
        let tables = self.resolver.tables();
        let graph = tables.roots[self.id.0 as usize]
            .ok_or_else(|| vec!["Missing eligible unit graph root".into()])?;
        let pair = &tables.graphs[graph.0 as usize];
        let Some(cache) = self.resolver.validation_cache() else {
            return crate::producer::verify(pair, self);
        };
        if let Some(indexes) = cache.get(graph) {
            return Ok(ValidatedPair::with_indexes(pair.as_view(), self, indexes));
        }

        // No cache lock is held while verification recursively inspects helpers.
        let indexes = crate::producer::verify(pair, self)?.into_indexes();
        cache.insert(graph, Arc::clone(&indexes));
        Ok(ValidatedPair::with_indexes(pair.as_view(), self, indexes))
    }
}

#[cfg(test)]
mod tests;
