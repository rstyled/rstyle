//! Exact source-local dismissal; loading a snapshot never grants current evidence.
mod model;
use super::{Locator, StaleReason};
pub(crate) use model::*;

type Error = super::ReviewError;

/// Minted only by the live source adapter after a completed positive consumer assessment.
pub(crate) struct LiveFinding<'a> {
    snapshot: &'a Snapshot,
}

impl<'a> LiveFinding<'a> {
    pub(super) fn checked(snapshot: &'a Snapshot) -> Result<Self, Error> {
        validate_snapshot(snapshot)?;
        Ok(Self { snapshot })
    }
}

#[derive(Debug)]
pub(crate) struct StaleEntry<'a> {
    pub entry: &'a Entry,
    pub reason: StaleReason,
}

#[derive(Debug)]
pub(crate) enum Lookup<'a> {
    HiddenExact(&'a Entry),
    VisibleNew,
    Stale(Vec<StaleEntry<'a>>),
}

pub(crate) struct ValidatedStore<'a> {
    entries: &'a [Entry],
}

impl<'a> ValidatedStore<'a> {
    pub(crate) fn new(entries: &'a [Entry]) -> Result<Self, Error> {
        validate_entries(entries)?;
        Ok(Self { entries })
    }

    pub(crate) fn lookup(&self, input: &LiveFinding<'_>) -> Lookup<'a> {
        let current = input.snapshot;
        if let Some(entry) = self.entries.iter().find(|entry| entry.finding == *current) {
            return Lookup::HiddenExact(entry);
        }

        let stale: Vec<_> = self
            .entries
            .iter()
            .filter_map(|entry| {
                stale_reason(&entry.finding, current).map(|reason| StaleEntry { entry, reason })
            })
            .collect();
        if stale.is_empty() {
            Lookup::VisibleNew
        } else {
            Lookup::Stale(stale)
        }
    }
}

pub(crate) fn checked(entries: &[Entry]) -> Result<ValidatedStore<'_>, Error> {
    ValidatedStore::new(entries)
}

pub(crate) fn record_dismissal(
    entries: &mut Vec<Entry>,
    current: &LiveFinding<'_>,
    reason: String,
) -> Result<(), Error> {
    validate_entries(entries)?;
    if reason.trim().is_empty() {
        return Err(Error::InvalidReason);
    }
    if entries
        .iter()
        .any(|entry| entry.finding == *current.snapshot)
    {
        return Err(Error::DuplicateReview);
    }
    if entries.len() >= 4096 {
        return Err(Error::ExpressionBudget);
    }

    entries.push(Entry {
        finding: current.snapshot.clone(),
        reason,
    });
    Ok(())
}

pub(crate) fn validate_entries(entries: &[Entry]) -> Result<(), Error> {
    if entries.len() > 4096 {
        return Err(Error::ExpressionBudget);
    }
    for (index, entry) in entries.iter().enumerate() {
        if entry.reason.trim().is_empty() {
            return Err(Error::InvalidReason);
        }
        validate_snapshot(&entry.finding)?;
        if entries[..index]
            .iter()
            .any(|old| old.finding == entry.finding)
        {
            return Err(Error::DuplicateReview);
        }
    }
    Ok(())
}

fn validate_snapshot(snapshot: &Snapshot) -> Result<(), Error> {
    let versions = &snapshot.versions;
    if versions.detector.is_empty()
        || [
            versions.detector_version,
            versions.identity_version,
            versions.locator_version,
            versions.context_version,
        ]
        .contains(&0)
    {
        return Err(Error::InvalidVersion);
    }

    let expected = match snapshot.proposal {
        Proposal::EnumDispatch { .. } => 1,
        Proposal::OrderedCapability { .. } => 2,
    };
    if snapshot.members.len() != expected
        || snapshot
            .members
            .windows(2)
            .any(|pair| pair[0].locator >= pair[1].locator)
    {
        return Err(Error::InvalidExpression);
    }
    for member in &snapshot.members {
        validate_locator(&member.locator)?;
        validate_context(&member.context)?;
        validate_source(&member.source)?;
    }
    if snapshot
        .members
        .windows(2)
        .any(|pair| pair[0].context != pair[1].context)
    {
        return Err(Error::InvalidContext);
    }
    validate_proposal(snapshot)
}

fn validate_proposal(snapshot: &Snapshot) -> Result<(), Error> {
    match &snapshot.proposal {
        Proposal::EnumDispatch { enumeration, arms } => {
            validate_declaration(enumeration, &snapshot.members[0].locator)?;
            if arms.len() != 2
                || arms[0].variant.is_empty()
                || arms[1].variant.is_empty()
                || arms[0].variant == arms[1].variant
            {
                return Err(Error::InvalidExpression);
            }
            for arm in arms {
                validate_selection(&arm.selection, &snapshot.members[0].locator)?;
            }
        }
        Proposal::OrderedCapability { roles } => {
            if roles.len() != 2 {
                return Err(Error::InvalidExpression);
            }
            for role in roles {
                validate_selection(&role.left, &snapshot.members[0].locator)?;
                validate_selection(&role.right, &snapshot.members[1].locator)?;
            }
            if roles[0].left.method.locator == roles[1].left.method.locator
                || roles[0].right.method.locator == roles[1].right.method.locator
            {
                return Err(Error::InvalidExpression);
            }
        }
    }
    Ok(())
}

fn validate_selection(selection: &Selection, caller: &Locator) -> Result<(), Error> {
    validate_declaration(&selection.owner, caller)?;
    validate_declaration(&selection.method, caller)?;
    if let Origin::LocalTrait {
        declaration,
        member,
    } = &selection.origin
    {
        validate_declaration(declaration, caller)?;
        if member.is_empty() {
            return Err(Error::InvalidExpression);
        }
    }
    Ok(())
}

fn validate_declaration(declaration: &Declaration, caller: &Locator) -> Result<(), Error> {
    validate_locator(&declaration.locator)?;
    validate_source(&declaration.source)?;
    if declaration.locator.package != caller.package || declaration.locator.source != caller.source
    {
        return Err(Error::InvalidLocator);
    }
    Ok(())
}

fn validate_source(source: &str) -> Result<(), Error> {
    if source.trim().is_empty() {
        return Err(Error::InvalidExpression);
    }
    if source.len() > 256 * 1024 {
        return Err(Error::ExpressionBudget);
    }
    Ok(())
}

fn validate_locator(locator: &Locator) -> Result<(), Error> {
    if !super::relative_source(&locator.package.identity)
        || locator.package.identity.rsplit('/').next() != Some("Cargo.toml")
        || locator.package.name.is_empty()
        || locator.package.version.is_empty()
        || !super::relative_source(&locator.source)
        || locator.qualified_name.is_empty()
        || locator.declaration_kind.is_empty()
        || locator.discriminator.is_empty()
    {
        return Err(Error::InvalidLocator);
    }
    Ok(())
}

fn validate_context(context: &BuildContext) -> Result<(), Error> {
    let target = &context.compilation_target;
    if [
        &context.compiler_commit,
        &context.edition,
        &context.data_layout,
        &context.target_triple,
        &context.opt_level,
        &context.panic_strategy,
        &context.extraction_policy,
        &target.name,
    ]
    .iter()
    .any(|value| value.is_empty())
        || context.pointer_width == 0
        || target.kind.is_empty()
        || target.kind.iter().any(String::is_empty)
        || !canonical(&target.kind)
        || !canonical(&context.features)
        || !canonical(&context.cfgs)
    {
        return Err(Error::InvalidContext);
    }
    Ok(())
}

fn canonical(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn stale_reason(previous: &Snapshot, current: &Snapshot) -> Option<StaleReason> {
    if std::mem::discriminant(&previous.proposal) != std::mem::discriminant(&current.proposal) {
        return None;
    }
    super::comparison::stale_reason(
        &previous.versions,
        &current.versions,
        previous
            .members
            .iter()
            .map(|member| (&member.locator, &member.context)),
        current
            .members
            .iter()
            .map(|member| (&member.locator, &member.context)),
    )
}

#[cfg(test)]
mod tests;
