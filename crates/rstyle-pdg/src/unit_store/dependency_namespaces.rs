//! Source-only views of direct, independently checked dependency edges.
#[cfg(test)]
mod tests;

use super::*;
use crate::dependency_namespaces::capture::Import;

#[derive(Clone)]
pub(super) struct Association {
    pub dependency: usize,
    pub compiled_dependency: serde_json::Value,
}

/// This view is neither serialized nor constructed from a namespace observation.
/// Only the owning dependency closure can supply its exact producer.
pub struct CheckedDependencyNamespace<'a> {
    owner: &'a UnitStore,
    sources: &'a derived::Sources,
    capture: Arc<crate::dependency_namespaces::capture::UnitCapture>,
    import: usize,
}

impl CheckedDependencyNamespace<'_> {
    /// Both complete generated source and compiler export graphs are required.
    /// Capability activation and the exact consumer source join remain separate.
    pub(crate) fn generated_exports(
        &self,
    ) -> Result<crate::dependency_namespaces::proof::GeneratedExports, String> {
        let import = &self.capture.imports[self.import];

        if import.site.target_kind != "Mod"
            || import.exports.is_empty()
            || import.exports.iter().any(|export| {
                !matches!(
                    export.target_kind.as_str(),
                    "Struct" | "Ctor(Struct, Const)"
                )
            })
        {
            return Err(
                "Generated namespace requires a bounded unit-struct module footprint".into(),
            );
        }
        let producer = self
            .sources
            .dependency_namespace_capture(self.owner.functions())?
            .ok_or("Missing original producer namespace capture")?;
        let generated = self
            .sources
            .generated_capture(self.owner.functions())?
            .ok_or("Missing original generated producer capture")?;
        let facts = self
            .owner
            .functions()
            .first()
            .ok_or("Missing original producer source owner")?;
        let environment = self.sources.generated_environment(facts)?;
        let census = facts
            .build
            .frontend_options
            .data::<Arc<crate::nominal_boundary::census::Census>>()?;
        let modules = environment
            .generated_source_modules(&facts.build.cfgs)
            .ok_or("Generated namespace lacks its checked producer source policy")?;

        crate::dependency_namespaces::proof::verify_generated_namespace(
            crate::dependency_namespaces::proof::GeneratedProducer {
                census: Arc::clone(&census),
                modules,
                namespaces: &producer,
                generated: &generated,
            },
            import,
        )
        .map_err(|error| error.to_string())
    }

    /// Original immutable evidence only. No callee certification is performed.
    pub fn functions(&self) -> &[HirFnFacts] {
        self.owner.functions()
    }

    /// Independently prove only the anonymous export footprint of this exact
    /// captured import. Consumer lexical ownership is a separate obligation.
    pub fn anonymous_prelude(
        &self,
    ) -> Result<crate::dependency_namespaces::proof::AnonymousPrelude, String> {
        self.prelude(false)
    }

    pub(crate) fn anonymous_prelude_with_widening(
        &self,
    ) -> Result<crate::dependency_namespaces::proof::AnonymousPrelude, String> {
        self.prelude(self.capture.imports[self.import].exports.len() == 3)
    }

    fn prelude(
        &self,
        widening: bool,
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
        let verify = if widening {
            crate::dependency_namespaces::proof::verify_widening_anonymous_prelude
        } else {
            crate::dependency_namespaces::proof::verify_anonymous_prelude
        };

        verify(
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
        sources: &owner.sources,
        capture,
        import: index,
    }))
}
