//! Run-local routing through the exact frozen target table, never a digest.
use super::{Plan, Target};
use std::path::{Path, PathBuf};

pub(super) const SCHEMA: &str = "rstyle.capture-ledger/2";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TargetId(u32);

fn target_id(plan: &Plan, target: &Target) -> Result<TargetId, String> {
    if plan.ledger_schema != SCHEMA {
        return Err("Unsupported capture ledger schema".into());
    }

    let mut matches = plan
        .targets
        .iter()
        .enumerate()
        .filter(|(_, entry)| *entry == target);
    let (index, _) = matches
        .next()
        .ok_or("Capture target is absent from frozen plan")?;
    if matches.next().is_some() {
        return Err("Duplicate capture target descriptor in frozen plan".into());
    }

    Ok(TargetId(
        u32::try_from(index).map_err(|_| "Oversized capture target table")?,
    ))
}

pub(in crate::clones) fn path(
    plan: &Plan,
    root: &Path,
    target: &Target,
    test: bool,
) -> Result<PathBuf, String> {
    let id = target_id(plan, target)?;
    let mode = if test { "test" } else { "check" };
    Ok(root.join(format!("unit-t{}-{mode}.json", id.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(package: &str) -> Target {
        Target {
            package: package.into(),
            manifest_dir: format!("/{package}").into(),
            package_context: None,
            source: format!("/{package}/lib.rs").into(),
            name: "library".into(),
            kind: vec!["lib".into()],
            test: true,
        }
    }

    fn plan(targets: Vec<Target>) -> Plan {
        Plan {
            compiler_advisory: false,
            ledger_schema: SCHEMA.into(),
            extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile::R0,
            root: "/run".into(),
            driver: "/driver".into(),
            targets,
            compiler_identity: "compiler".into(),
            cargo_identity: "cargo".into(),
            primary_units: Vec::new(),
            dependency_units: Vec::new(),
        }
    }

    #[test]
    fn routing_uses_exact_plan_ordinals_and_distinct_test_modes() {
        let plan = plan(vec![target("a"), target("b")]);
        let root = Path::new("/run");
        assert_eq!(
            path(&plan, root, &target("a"), false).unwrap(),
            root.join("unit-t0-check.json")
        );
        assert_eq!(
            path(&plan, root, &target("a"), true).unwrap(),
            root.join("unit-t0-test.json")
        );
        assert_eq!(
            path(&plan, root, &target("b"), false).unwrap(),
            root.join("unit-t1-check.json")
        );
        let mut changed = target("a");
        changed.kind = vec!["bin".into()];
        assert!(path(&plan, root, &changed, false).is_err());
        changed = target("a");
        changed.manifest_dir = "/different".into();
        assert!(path(&plan, root, &changed, false).is_err());
    }

    #[test]
    fn duplicate_descriptors_and_old_routing_schemas_fail() {
        let mut plan = plan(vec![target("a"), target("a")]);
        assert!(target_id(&plan, &target("a")).is_err());
        plan.targets.pop();
        plan.ledger_schema = "rstyle.capture-ledger/1".into();
        assert!(target_id(&plan, &target("a")).is_err());
    }

    #[test]
    fn duplicate_ledger_publication_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let path = path(
            &plan(vec![target("a")]),
            directory.path(),
            &target("a"),
            false,
        )
        .unwrap();
        super::super::reserve_ledger(&path).unwrap();
        assert!(super::super::reserve_ledger(&path).is_err());
    }
}
