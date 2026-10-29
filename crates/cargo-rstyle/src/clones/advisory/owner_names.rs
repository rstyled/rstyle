//! Reuse complete declaration counts without retaining more owner payloads.
use super::{Report, Unit};
use crate::clones::{
    identity::UnitId, report::Context, review::compiler_diagnostics::observed::Assessed,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// Locator packages are rooted at the manifest directory. Feature/cfg variants
// share a target here; check/test contexts remain distinct.
type Target<'a> = (&'a Path, &'a str, BTreeSet<&'a str>, bool);

#[derive(Default)]
pub(super) struct Census<'a> {
    targets: BTreeMap<UnitId, Target<'a>>,
    names: BTreeMap<Target<'a>, BTreeMap<&'a str, usize>>,
}

pub(super) fn counts<'a>(report: &'a Report, ctx: &'a Context<'_>) -> Census<'a> {
    let units: BTreeMap<_, _> = report.units.iter().map(|unit| (unit.unit, unit)).collect();
    let mut census = Census::default();
    for context in &ctx.handoff.unit_contexts {
        let Some(unit) = units.get(&context.id) else {
            continue;
        };
        let captured = &ctx.capture.units[context.observed_unit];
        let target = (
            unit.package_root.as_path(),
            captured.target.name.as_str(),
            captured.target.kind.iter().map(String::as_str).collect(),
            captured.test,
        );
        census.observe(target.clone(), &unit.diagnostic_owner_names);
        census.targets.insert(unit.unit, target);
    }
    census
}

impl<'a> Census<'a> {
    fn observe(&mut self, target: Target<'a>, observed: &'a BTreeMap<String, usize>) {
        let names = self.names.entry(target).or_default();
        for (name, count) in observed {
            *names.entry(name.as_str()).or_default() += count;
        }
    }

    fn unique(&self, target: &Target<'_>, name: &str) -> bool {
        self.names.get(target).and_then(|names| names.get(name)) == Some(&1)
    }
}

pub(super) fn unique(census: &Census<'_>, unit: &Unit, assessed: &Assessed) -> bool {
    let Some(name) = assessed.qualified_name() else {
        return false;
    };
    census
        .targets
        .get(&unit.unit)
        .is_some_and(|target| census.unique(target, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(package: &str, test: bool) -> Target<'_> {
        (Path::new(package), "library", BTreeSet::from(["lib"]), test)
    }

    #[test]
    fn full_census_rejects_variant_duplicates_and_preserves_disjoint_owners() {
        let first = BTreeMap::from([("crate::shared".into(), 1), ("crate::first".into(), 1)]);
        let second = BTreeMap::from([("crate::shared".into(), 1), ("crate::second".into(), 1)]);
        let mut census = Census::default();
        census.observe(target("package", false), &first);
        census.observe(target("package", false), &second);
        assert!(!census.unique(&target("package", false), "crate::shared"));
        assert!(census.unique(&target("package", false), "crate::first"));
        assert!(census.unique(&target("package", false), "crate::second"));
        assert!(!census.unique(&target("package", false), "crate::missing"));
    }

    #[test]
    fn packages_and_check_test_targets_keep_separate_populations() {
        let names = BTreeMap::from([("crate::shared".into(), 1)]);
        let ambiguous = BTreeMap::from([("crate::shared".into(), 2)]);
        let mut census = Census::default();
        census.observe(target("first", false), &names);
        census.observe(target("second", false), &ambiguous);
        census.observe(target("first", true), &ambiguous);
        assert!(census.unique(&target("first", false), "crate::shared"));
        assert!(!census.unique(&target("second", false), "crate::shared"));
        assert!(!census.unique(&target("first", true), "crate::shared"));
    }
}
