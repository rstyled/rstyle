//! Changed-file membership borrows checked original paths without granting evidence.
use super::{Coverage, FunctionReport, Outcome, Report, changed_scope};
use std::path::Path;

#[derive(Clone, Copy)]
enum Family {
    Source,
    Guard,
}

pub(super) struct Scopes<'a> {
    pub sources: Option<changed_scope::Report<'a>>,
    pub guards: Option<changed_scope::Report<'a>>,
}

pub(super) fn evaluate<'a>(
    config: Option<&'a super::Config>,
    functions: &mut [FunctionReport<'_>],
    root: &Path,
    select_guards: bool,
) -> Scopes<'a> {
    let selection = config.and_then(super::Config::changed_files);
    Scopes {
        sources: family(
            selection.filter(|_| config.is_some_and(|config| config.source.is_some())),
            functions,
            root,
            Family::Source,
        ),
        guards: family(
            selection.filter(|_| select_guards),
            functions,
            root,
            Family::Guard,
        ),
    }
}

fn family<'a>(
    selection: Option<&'a changed_scope::Selection>,
    functions: &mut [FunctionReport<'_>],
    root: &Path,
    family: Family,
) -> Option<changed_scope::Report<'a>> {
    let selection = selection?;
    let owners = functions.iter_mut().map(|function| {
        let path = function
            .source_file
            .and_then(|original| checked_path(original, root));
        function.changed_file = path.map(|path| selection.contains(path));
        changed_scope::Owner {
            path,
            outside_scope: false,
            complete: match family {
                Family::Source => source_complete(function),
                Family::Guard => function.guard_clause.as_ref().is_some_and(|guard| {
                    guard.execution == super::Execution::Completed
                        && guard.coverage == Coverage::Complete
                }),
            },
            visible_findings: match family {
                Family::Source => function.source_visible_findings(),
                Family::Guard => usize::from(function.guard_finding()),
            },
        }
    });
    Some(changed_scope::evaluate(selection, owners))
}

pub(in crate::clones) fn selected_file(
    function: &crate::clones::Function,
    selection: &changed_scope::Selection,
    root: &Path,
) -> bool {
    super::source_file(function)
        .and_then(|original| checked_path(original, root))
        .is_some_and(|path| selection.contains(path))
}

fn source_complete(function: &FunctionReport<'_>) -> bool {
    function
        .assessments
        .iter()
        .all(|rule| rule.coverage == Coverage::Complete)
        && function.source_policy_complete()
}

impl FunctionReport<'_> {
    pub(super) fn guard_finding(&self) -> bool {
        self.guard_clause
            .as_ref()
            .is_some_and(|guard| guard.outcome == Outcome::Finding)
    }

    pub(super) fn source_policy_complete(&self) -> bool {
        let Some(exclusion) = &self.source_exclusion else {
            return false;
        };
        if !exclusion.complete() {
            return false;
        }
        if exclusion.excluded() {
            return true;
        }

        self.source_suppression
            .as_ref()
            .is_some_and(|suppression| suppression.complete())
    }
}

pub(in crate::clones) fn checked_path<'a>(original: &'a str, root: &Path) -> Option<&'a str> {
    let root_text = root.to_str()?;
    if original == "/" || !valid_absolute(original) || !valid_absolute(root_text) {
        return None;
    }
    match Path::new(original).strip_prefix(root) {
        Ok(relative) => relative.to_str().filter(|path| !path.is_empty()),
        // A checked absolute path cannot match a relative manifest entry.
        Err(_) => Some(original),
    }
}

fn valid_absolute(path: &str) -> bool {
    let Some(relative) = path.strip_prefix('/') else {
        return false;
    };
    !path.contains(['\\', ':'])
        && !path.chars().any(char::is_control)
        && (relative.is_empty()
            || relative
                .split('/')
                .all(|part| !matches!(part, "" | "." | "..")))
}

impl Report<'_> {
    pub(in crate::clones) fn finding_exit_count(&self) -> usize {
        let source = self.changed_scope.as_ref().map_or_else(
            || {
                self.functions
                    .iter()
                    .map(FunctionReport::source_visible_findings)
                    .sum()
            },
            |scope| scope.visible_findings,
        );
        let guards = self
            .functions
            .iter()
            .filter(|function| {
                (self.guard_changed_scope.is_none() || function.changed_file == Some(true))
                    && function.visible_guard_finding()
            })
            .count();
        source + guards
    }
}

#[cfg(test)]
mod tests {
    use super::checked_path;
    use std::path::Path;

    #[test]
    fn original_paths_preserve_workspace_boundaries_and_borrow_unicode() {
        let root = Path::new("/workspace");
        assert_eq!(
            checked_path("/workspace/src/café.rs", root),
            Some("src/café.rs")
        );
        assert_eq!(
            checked_path("/workspace-other/src/lib.rs", root),
            Some("/workspace-other/src/lib.rs")
        );
        assert_eq!(checked_path("/workspace", root), None);
        assert_eq!(checked_path("/", root), None);
        assert_eq!(
            checked_path("/src/lib.rs", Path::new("/")),
            Some("src/lib.rs")
        );
    }

    #[test]
    fn ambiguous_originals_and_roots_cannot_select_a_file() {
        for invalid in [
            "relative/lib.rs",
            "/workspace/./lib.rs",
            "/workspace/../lib.rs",
            "/workspace//lib.rs",
            "/workspace/lib.rs/",
            "/workspace/lib\\name.rs",
            "/workspace/lib:name.rs",
            "/workspace/lib\nname.rs",
        ] {
            assert_eq!(checked_path(invalid, Path::new("/workspace")), None);
        }
        for invalid in [
            "workspace",
            "/workspace/.",
            "/a/../workspace",
            "/workspace//",
        ] {
            assert_eq!(checked_path("/workspace/lib.rs", Path::new(invalid)), None);
        }
    }
}
