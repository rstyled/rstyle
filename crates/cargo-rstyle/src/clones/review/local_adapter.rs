//! Durable review snapshots reuse completed local declaration witnesses.
use super::{CompilationTarget, Locator, Package, Versions, local};
use crate::clones::build::StringFact;
use crate::clones::capture;
use std::path::{Component, Path};

pub(super) fn versions(detector: &str) -> Versions {
    Versions {
        detector: detector.into(),
        detector_version: if detector == "compiler-patterns" {
            rstyle_core::candidates::POLICY_VERSION
        } else {
            1
        },
        identity_version: if detector == "compiler-patterns" {
            2
        } else {
            1
        },
        locator_version: 1,
        context_version: 1,
    }
}

pub(super) struct Identity {
    package: Package,
    source: String,
}

impl Identity {
    /// The identity of a function viewed through any of its inputs.
    pub(super) fn viewed(
        view: &crate::clones::FactView<'_>,
        unit: &capture::Unit,
        plan: &capture::Plan,
    ) -> Result<Self, String> {
        Self::located(view.source_file, unit, plan)
    }

    fn located(
        path: Option<&str>,
        unit: &capture::Unit,
        plan: &capture::Plan,
    ) -> Result<Self, String> {
        let package = unit
            .target
            .package_context
            .as_ref()
            .ok_or("Review package context is missing")?;
        if package.package_id != unit.target.package
            || package.manifest_path.parent() != Some(unit.target.manifest_dir.as_path())
        {
            return Err("Review package or extraction context is contradictory".into());
        }
        let path = path.ok_or("Review source file is unavailable")?;
        Ok(Self {
            package: Package {
                identity: relative(&package.manifest_path, &plan.root)?,
                name: package.name.clone(),
                version: package.version.clone(),
            },
            source: relative(Path::new(path), &unit.target.manifest_dir)?,
        })
    }

    pub(super) fn locator(&self, qualified_name: String, kind: &str) -> Locator {
        Locator {
            package: self.package.clone(),
            source: self.source.clone(),
            qualified_name,
            declaration_kind: kind.into(),
            discriminator: "unique-declaration".into(),
        }
    }
}

pub(super) fn context(
    build: &crate::clones::build::BuildFacts,
    unit: &capture::Unit,
) -> Result<local::BuildContext, String> {
    let width = (build.pointer_width.status == crate::clones::build::FactStatus::Known)
        .then_some(build.pointer_width.value)
        .flatten()
        .and_then(|width| u16::try_from(width).ok())
        .filter(|width| *width != 0)
        .ok_or("Review target pointer width is unavailable")?;
    Ok(local::BuildContext {
        compiler_commit: text(&build.rustc_commit, "compiler commit")?,
        edition: text(&build.edition, "edition")?,
        data_layout: text(&build.data_layout, "data layout")?,
        compilation_target: CompilationTarget {
            name: unit.target.name.clone(),
            kind: sorted(&unit.target.kind),
            test: unit.test,
        },
        target_triple: text(&build.target_triple, "target triple")?,
        pointer_width: width,
        features: sorted(&build.features),
        cfgs: sorted(&build.cfgs),
        opt_level: text(&build.opt_level, "optimization level")?,
        debug_assertions: boolean(&build.debug_assertions, "debug assertions")?,
        overflow_checks: boolean(&build.overflow_checks, "overflow checks")?,
        panic_strategy: text(&build.panic_strategy, "panic strategy")?,
        extraction_policy: "compat.cpg/1".into(),
    })
}

fn relative(path: &Path, root: &Path) -> Result<String, String> {
    let path = rstyle_capture::absolute_path(path)?;
    let root = rstyle_capture::absolute_path(root)?;

    let suffix = path
        .strip_prefix(&root)
        .map_err(|_| "Review locator lies outside its declared root")?;
    let components = suffix
        .components()
        .map(|component| match component {
            Component::Normal(value) => value.to_str().ok_or("Review locator is not UTF-8"),
            _ => Err("Review locator contains an ambiguous path component"),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if components.is_empty() {
        return Err("Review locator is empty".into());
    }
    Ok(components.join("/"))
}

fn text(fact: &StringFact, label: &str) -> Result<String, String> {
    match fact.known_value() {
        Some(value) if !value.is_empty() => Ok(value.into()),
        _ => Err(format!("Review {label} is unavailable")),
    }
}

fn boolean(fact: &crate::clones::build::BoolFact, label: &str) -> Result<bool, String> {
    (fact.status == crate::clones::build::FactStatus::Known)
        .then_some(fact.value)
        .flatten()
        .ok_or_else(|| format!("Review {label} is unavailable"))
}

fn sorted(values: &[String]) -> Vec<String> {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    values
}
