//! Selection filters density presentation after reviews, preserving whole-scope evidence.
use super::{Report, density};
use crate::clones::analysis::{changed_file_reports::checked_path, changed_scope};
use std::path::Path;

impl<'a> Report<'a> {
    pub(in crate::clones) fn select_changed(
        &mut self,
        selection: Option<&'a changed_scope::Selection>,
        root: &Path,
    ) {
        let Some(selection) = selection.filter(|_| self.density_requested) else {
            return;
        };
        let owners = self.functions.iter_mut().map(|function| {
            let path = function
                .source_file
                .and_then(|original| checked_path(original, root));
            function.changed_file = path.map(|path| selection.contains(path));
            changed_scope::Owner {
                path,
                complete: function
                    .density
                    .as_ref()
                    .is_some_and(|density| density.complete),
                visible_findings: function
                    .density
                    .as_ref()
                    .map_or(0, density::Report::presented_findings),
            }
        });
        self.changed_scope = Some(changed_scope::evaluate(selection, owners));
    }

    pub(in crate::clones) fn finding_exit_count(&self) -> usize {
        self.changed_scope
            .as_ref()
            .map_or(self.visible_findings, |scope| scope.visible_findings)
    }
}
