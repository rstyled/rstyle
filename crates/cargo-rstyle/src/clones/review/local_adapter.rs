//! Durable review snapshots reuse completed local declaration witnesses.
use super::adapter::{boolean, relative, sorted, text};
use super::{CompilationTarget, Locator, Package, Versions, local};
use crate::clones::capture;
use rstyle_pdg::{HirFnFacts, consumer_facts::dispatch};
use std::path::Path;

pub(in crate::clones) struct Endpoint {
    member: local::Member,
    selections: [local::Selection; 2],
}

pub(in crate::clones) struct Input<'a> {
    pub facts: &'a HirFnFacts,
    pub unit: &'a capture::Unit,
    pub plan: &'a capture::Plan,
    pub index: &'a dispatch::Index,
}

pub(in crate::clones) fn dispatch(
    input: Input<'_>,
    candidate: &dispatch::Candidate,
) -> Result<local::Snapshot, String> {
    let Input {
        facts,
        unit,
        plan,
        index,
    } = input;
    let source = index.review_dispatch(facts, candidate)?;
    let identity = Identity::new(&facts.build, unit, plan)?;
    let member = identity.member(source.caller)?;
    let enumeration = identity.declaration(source.enumeration);
    let arms = source
        .arms
        .into_iter()
        .map(|(variant, selected)| local::DispatchArm {
            variant,
            selection: identity.selection(selected),
        })
        .collect();

    Ok(local::Snapshot {
        versions: versions("local-enum-dispatch"),
        members: vec![member],
        proposal: local::Proposal::EnumDispatch { enumeration, arms },
    })
}

/// Observe only the checked original locator and build context for status lookup.
pub(super) fn member_key(input: Input<'_>) -> Result<(Locator, local::BuildContext), String> {
    let identity = Identity::new(&input.facts.build, input.unit, input.plan)?;
    let (qualified_name, declaration_kind) = input.index.review_location(input.facts)?;
    Ok((
        Locator {
            package: identity.package,
            source: identity.source,
            qualified_name,
            declaration_kind: declaration_kind.into(),
            discriminator: "unique-declaration".into(),
        },
        context(&input.facts.build, input.unit)?,
    ))
}

/// Build once per wrapper endpoint, then borrow across proposal pairs.
pub(in crate::clones) fn capability_member(
    input: Input<'_>,
    wrapper: &dispatch::capabilities::Wrapper,
) -> Result<Endpoint, String> {
    let Input {
        facts,
        unit,
        plan,
        index,
    } = input;
    let source = index.review_capability(facts, wrapper)?;
    let identity = Identity::new(&facts.build, unit, plan)?;
    let member = identity.member(source.caller)?;
    let selections = source
        .selections
        .into_iter()
        .map(|selected| identity.selection(selected))
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "Review requires two ordered wrapper roles")?;

    Ok(Endpoint { member, selections })
}

pub(in crate::clones) fn capability(
    left: &Endpoint,
    right: &Endpoint,
) -> Result<local::Snapshot, String> {
    let (left, right) = if left.member.locator < right.member.locator {
        (left, right)
    } else if right.member.locator < left.member.locator {
        (right, left)
    } else {
        return Err("Review capability endpoints have ambiguous durable locators".into());
    };
    let roles = left
        .selections
        .iter()
        .zip(&right.selections)
        .map(|(left, right)| local::CapabilityRole {
            left: left.clone(),
            right: right.clone(),
        })
        .collect();
    Ok(local::Snapshot {
        versions: versions("ordered-local-capability"),
        members: vec![left.member.clone(), right.member.clone()],
        proposal: local::Proposal::OrderedCapability { roles },
    })
}

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

pub(super) struct Identity<'a> {
    build: &'a rstyle_pdg::BuildFacts,
    unit: &'a capture::Unit,
    package: Package,
    source: String,
}

impl<'a> Identity<'a> {
    pub(super) fn new(
        build: &'a rstyle_pdg::BuildFacts,
        unit: &'a capture::Unit,
        plan: &capture::Plan,
    ) -> Result<Self, String> {
        let package = unit
            .target
            .package_context
            .as_ref()
            .ok_or("Review package context is missing")?;
        if package.package_id != unit.target.package
            || package.manifest_path.parent() != Some(unit.target.manifest_dir.as_path())
            || unit.extraction_profile != plan.extraction_profile
        {
            return Err("Review package or extraction context is contradictory".into());
        }
        let path = build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            .ok_or("Review source file is unavailable")?;
        Ok(Self {
            build,
            unit,
            package: Package {
                identity: relative(&package.manifest_path, &plan.root)?,
                name: package.name.clone(),
                version: package.version.clone(),
            },
            source: relative(Path::new(path), &unit.target.manifest_dir)?,
        })
    }

    fn member(&self, declaration: dispatch::review::Declaration) -> Result<local::Member, String> {
        let declaration = self.declaration(declaration);
        Ok(local::Member {
            locator: declaration.locator,
            context: context(self.build, self.unit)?,
            source: declaration.source,
        })
    }

    fn declaration(&self, declaration: dispatch::review::Declaration) -> local::Declaration {
        local::Declaration {
            locator: self.locator(declaration.qualified_name, declaration.kind),
            source: declaration.source,
        }
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

    fn selection(&self, selected: dispatch::review::Selection) -> local::Selection {
        local::Selection {
            owner: self.declaration(selected.owner),
            method: self.declaration(selected.method),
            origin: selected
                .origin
                .map_or(local::Origin::Inherent, |(declaration, member)| {
                    local::Origin::LocalTrait {
                        declaration: Box::new(self.declaration(declaration)),
                        member,
                    }
                }),
        }
    }
}

pub(super) fn context(
    build: &rstyle_pdg::BuildFacts,
    unit: &capture::Unit,
) -> Result<local::BuildContext, String> {
    let width = (build.pointer_width.status == rstyle_pdg::FactStatus::Known)
        .then_some(build.pointer_width.value)
        .flatten()
        .and_then(|width| u16::try_from(width).ok())
        .filter(|width| *width != 0)
        .ok_or("Review target pointer width is unavailable")?;
    let extraction = unit.extraction_profile.policy();
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
        extraction_policy: format!("{}/{}", extraction.id, extraction.version),
    })
}
