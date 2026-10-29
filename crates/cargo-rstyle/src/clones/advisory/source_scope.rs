//! Shared explicit authored population for compiler consumers.
use serde::Serialize;

/// Outside the requested authored population, not measured-empty or unavailable authored input.
#[derive(Serialize)]
pub(super) struct SourceScope {
    kind: crate::clones::CompilerSourceScope,
    pub(super) outside_scope_functions: Vec<crate::clones::FunctionKey>,
}

impl SourceScope {
    pub(super) fn render(&self, unavailable: usize) -> String {
        format!(
            "Authored function scope: {} generated functions outside scope; {unavailable} applicable inputs unavailable.\n",
            self.outside_scope_functions.len(),
        )
    }

    pub(super) fn select(
        kind: crate::clones::CompilerSourceScope,
        generated_functions: impl IntoIterator<Item = crate::clones::FunctionKey>,
    ) -> Option<Self> {
        (kind == crate::clones::CompilerSourceScope::Authored).then(|| Self {
            kind,
            outside_scope_functions: generated_functions.into_iter().collect(),
        })
    }
}
