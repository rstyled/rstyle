//! Source-only views of direct, independently checked dependency edges.
#[cfg(test)]
mod tests;

use super::*;
use crate::dependency_namespaces::capture::Import;

pub(super) struct Association {
    pub dependency: usize,
    pub compiled_dependency: serde_json::Value,
}

/// This view is neither serialized nor constructed from a namespace observation.
/// Only the owning dependency closure can supply its exact producer.
pub struct CheckedDependencyNamespace<'a> {
    owner: &'a UnitStore,
    capture: Arc<crate::dependency_namespaces::capture::UnitCapture>,
    import: usize,
}

impl CheckedDependencyNamespace<'_> {
    /// Original immutable evidence only. No callee certification is performed.
    pub fn functions(&self) -> &[HirFnFacts] {
        self.owner.functions()
    }

    /// Independently prove only the anonymous export footprint of this exact
    /// captured import. Consumer lexical ownership is a separate obligation.
    pub fn anonymous_prelude(
        &self,
    ) -> Result<crate::dependency_namespaces::proof::AnonymousPrelude, String> {
        let producer = crate::dependency_namespaces::unit_capture(self.owner.functions())?
            .ok_or("Missing original producer namespace capture")?;
        let facts = self
            .owner
            .functions()
            .first()
            .ok_or("Missing original producer source owner")?;
        let census = facts
            .build
            .frontend_options
            .data::<Arc<crate::nominal_boundary::census::Census>>()?;
        crate::dependency_namespaces::proof::verify_anonymous_prelude(
            Arc::clone(&census),
            &producer,
            &self.capture.imports[self.import],
        )
        .map_err(|error| error.to_string())
    }
}

pub(super) fn selected<'a>(
    view: &FunctionView<'a>,
    import: &Import,
) -> Result<Option<CheckedDependencyNamespace<'a>>, String> {
    let Some(capture) = view.dependency_namespace_capture()? else {
        return Ok(None);
    };
    let index = capture
        .imports
        .iter()
        .position(|observed| observed == import)
        .ok_or("Namespace request is absent from the exact owning capture")?;

    let tables = view.resolver.tables();
    let selected = tables
        .namespace_associations
        .iter()
        .find(|edge| edge.compiled_dependency == import.compiled_dependency);
    let Some(edge) = selected else {
        return Ok(None);
    };
    let owner = view
        .resolver
        .dependencies()
        .get(edge.dependency)
        .ok_or("Missing direct namespace dependency owner")?;
    Ok(Some(CheckedDependencyNamespace {
        owner: &owner.store,
        capture,
        import: index,
    }))
}
