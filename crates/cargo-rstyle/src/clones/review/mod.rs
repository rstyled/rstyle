//! Pure exact review lifecycle. Loading bytes never grants current pair validity.
//! No hashes: full versioned member values are the only equality authority.
mod adapter;
pub(super) mod baseline_common;
mod baseline_io;
mod comparison;
pub(super) mod compiler_baseline;
pub(super) mod compiler_diagnostics;
mod compiler_owners;
pub(super) mod compiler_pattern_baseline;
pub(super) mod compiler_patterns;
pub(super) mod density_baseline;
pub(super) mod density_rules;
pub(super) mod density_workflow;
mod exact_rules;
pub(super) mod guard_rules;
pub(super) mod guard_workflow;
pub(super) mod length_rules;
pub(super) mod local;
mod local_adapter;
pub(super) mod local_workflow;
mod model;
pub(super) mod mutation_rules;
pub(super) mod mutation_workflow;
mod numeric_workflow;
mod outcome;
mod owner_observations;
mod owner_population;
mod positive_workflow;
pub(super) mod prune;
mod publication;
pub(super) mod source_baseline;
pub(super) mod source_rules;
pub(super) mod source_workflow;
pub(super) mod spacing_rules;
pub(super) mod status;
mod status_workflow;
mod summary;
pub(super) mod workflow;
pub(crate) use model::*;
use std::collections::BTreeSet;

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewError {
    InvalidSchema,
    InvalidReason,
    InvalidVersion,
    InvalidContext,
    InvalidLocator,
    AmbiguousLocator,
    NoncanonicalMembers,
    InvalidPairRoster,
    IncompletePairRoster,
    DuplicateReview,
    InvalidSignature,
    InvalidExpression,
    InvalidLiteral,
    ExpressionBudget,
}

/// Only the owning live adapter (a child module) may mint this capability.
/// The adapter must supply independently checked proposal pairs, never graph
/// connectivity. No serialized shape can construct this value.
pub(crate) struct LiveFinding<'a> {
    snapshot: &'a FindingSnapshot,
}

impl<'a> LiveFinding<'a> {
    fn checked(
        snapshot: &'a FindingSnapshot,
        checked_pairs: &[(&MemberKey, &MemberKey)],
    ) -> Result<Self, ReviewError> {
        validate_snapshot(snapshot)?;
        validate_pairs(snapshot, checked_pairs)?;
        Ok(Self { snapshot })
    }
}

pub(crate) enum ReviewInput<'a> {
    Checked(&'a LiveFinding<'a>),
    Unavailable { reason: &'a str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaleReason {
    Versions,
    Context,
    Body,
    Membership,
}

#[derive(Debug)]
pub(crate) struct StaleEntry<'a> {
    pub entry: &'a ReviewEntry,
    pub reason: StaleReason,
}

#[derive(Debug)]
pub(crate) enum Lookup<'a> {
    HiddenExact(&'a ReviewEntry),
    HiddenGrowth(&'a ReviewEntry),
    VisibleNew,
    Stale(Vec<StaleEntry<'a>>),
    Unavailable { reason: String },
}

impl Default for ReviewStore {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            entries: vec![],
            source_entries: vec![],
            parameter_entries: vec![],
            length_entries: vec![],
            spacing_entries: vec![],
            mutation_entries: vec![],
            density_entries: vec![],
            compiler_pattern_entries: vec![],
            compiler_diagnostic_entries: vec![],
            guard_entries: vec![],
        }
    }
}

impl ReviewStore {
    pub(super) fn retained_entries(&self) -> usize {
        self.entries.len()
            + self.source_entries.len()
            + self.parameter_entries.len()
            + self.length_entries.len()
            + self.spacing_entries.len()
            + self.mutation_entries.len()
            + self.density_entries.len()
            + self.compiler_pattern_entries.len()
            + self.compiler_diagnostic_entries.len()
            + self.guard_entries.len()
    }

    pub(crate) fn from_json(bytes: &str) -> Result<Self, String> {
        baseline_io::parse(bytes.as_bytes())
    }

    pub(crate) fn to_json(&self) -> Result<String, String> {
        String::from_utf8(baseline_io::encode(self)?).map_err(|error| error.to_string())
    }

    pub(crate) fn record_dismissal(
        &mut self,
        current: &LiveFinding<'_>,
        membership: MembershipPolicy,
        reason: String,
    ) -> Result<(), ReviewError> {
        self.validate()?;
        if reason.trim().is_empty() {
            return Err(ReviewError::InvalidReason);
        }

        if self
            .entries
            .iter()
            .any(|entry| entry.finding == *current.snapshot)
        {
            return Err(ReviewError::DuplicateReview);
        }
        self.entries.push(ReviewEntry {
            finding: current.snapshot.clone(),
            membership,
            reason,
        });
        Ok(())
    }

    pub(crate) fn checked(&self) -> Result<ValidatedStore<'_>, ReviewError> {
        self.validate()?;
        Ok(ValidatedStore { store: self })
    }

    #[cfg(test)]
    pub(crate) fn lookup(&self, input: ReviewInput<'_>) -> Lookup<'_> {
        match self.checked() {
            Ok(store) => store.lookup(input),
            Err(error) => Lookup::Unavailable {
                reason: format!("{error:?}"),
            },
        }
    }

    fn validate(&self) -> Result<(), ReviewError> {
        if !matches!(self.schema_version, 1..=10)
            || (self.schema_version == 1 && !self.source_entries.is_empty())
            || (self.schema_version < 3 && !self.parameter_entries.is_empty())
            || (self.schema_version < 4 && !self.length_entries.is_empty())
            || (self.schema_version < 5 && !self.spacing_entries.is_empty())
            || (self.schema_version < 6 && !self.mutation_entries.is_empty())
            || (self.schema_version < 7 && !self.density_entries.is_empty())
            || (self.schema_version < 8 && !self.compiler_pattern_entries.is_empty())
            || (self.schema_version < 9 && !self.compiler_diagnostic_entries.is_empty())
            || (self.schema_version < 10 && !self.guard_entries.is_empty())
        {
            return Err(ReviewError::InvalidSchema);
        }

        local::validate_entries(&self.source_entries)?;
        source_rules::validate_entries(&self.parameter_entries)?;
        length_rules::validate_entries(&self.length_entries)?;
        spacing_rules::validate_entries(&self.spacing_entries)?;
        mutation_rules::validate_entries(&self.mutation_entries)?;
        density_rules::validate_entries(&self.density_entries)?;
        compiler_patterns::validate_entries(&self.compiler_pattern_entries)?;
        compiler_diagnostics::validate_entries(&self.compiler_diagnostic_entries)?;
        guard_rules::validate_entries(&self.guard_entries)?;
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.reason.trim().is_empty() {
                return Err(ReviewError::InvalidReason);
            }
            validate_snapshot(&entry.finding)?;
            if self.entries[..index]
                .iter()
                .any(|previous| previous.finding == entry.finding)
            {
                return Err(ReviewError::DuplicateReview);
            }
        }
        Ok(())
    }
}

impl baseline_io::Artifact for ReviewStore {
    const LABEL: &'static str = "Review store";

    fn validate(&self) -> Result<(), String> {
        ReviewStore::validate(self).map_err(|error| format!("{error:?}"))
    }
}

/// Completed validation borrows the store immutably across all finding lookups.
pub(crate) struct ValidatedStore<'a> {
    store: &'a ReviewStore,
}

impl<'a> ValidatedStore<'a> {
    pub(crate) fn lookup(&self, input: ReviewInput<'_>) -> Lookup<'a> {
        let current = match input {
            ReviewInput::Checked(current) => current,
            ReviewInput::Unavailable { reason } => {
                return Lookup::Unavailable {
                    reason: reason.into(),
                };
            }
        };

        if let Some(entry) = self
            .store
            .entries
            .iter()
            .find(|entry| entry.finding == *current.snapshot)
        {
            return Lookup::HiddenExact(entry);
        }

        let mut stale = Vec::new();
        for entry in &self.store.entries {
            if permits_growth(entry, current.snapshot) {
                return Lookup::HiddenGrowth(entry);
            }
            if let Some(reason) = stale_reason(&entry.finding, current.snapshot) {
                stale.push(StaleEntry { entry, reason });
            }
        }

        if stale.is_empty() {
            Lookup::VisibleNew
        } else {
            Lookup::Stale(stale)
        }
    }
}

fn validate_snapshot(snapshot: &FindingSnapshot) -> Result<(), ReviewError> {
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
        return Err(ReviewError::InvalidVersion);
    }
    if !(2..=64).contains(&snapshot.members.len()) {
        return Err(ReviewError::NoncanonicalMembers);
    }

    let mut locators = BTreeSet::new();
    for member in &snapshot.members {
        validate_context(&member.key.context)?;
        validate_member_expression(member)?;
        let locator = &member.key.locator;
        if !relative_source(&locator.package.identity)
            || locator.package.identity.rsplit('/').next() != Some("Cargo.toml")
            || locator.package.name.is_empty()
            || locator.package.version.is_empty()
            || locator.qualified_name.is_empty()
            || locator.declaration_kind.is_empty()
            || locator.discriminator.is_empty()
            || !relative_source(&locator.source)
        {
            return Err(ReviewError::InvalidLocator);
        }
        if !locators.insert(locator) {
            return Err(ReviewError::AmbiguousLocator);
        }
    }

    if snapshot
        .members
        .windows(2)
        .any(|pair| pair[0].key >= pair[1].key)
    {
        return Err(ReviewError::NoncanonicalMembers);
    }
    Ok(())
}

fn validate_context(context: &Context) -> Result<(), ReviewError> {
    if context.compiler_commit.is_empty()
        || context.edition.is_empty()
        || context.data_layout.is_empty()
        || context.opt_level.is_empty()
        || context.panic_strategy.is_empty()
        || context.compilation_target.name.is_empty()
        || context.compilation_target.kind.is_empty()
        || context.compilation_target.kind.iter().any(String::is_empty)
        || context
            .compilation_target
            .kind
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || context.target_triple.is_empty()
        || context.pointer_width == 0
        || context.extraction_policy.is_empty()
        || !valid_matching_policy(&context.matching_policy)
        || context.consumer_policy.id.is_empty()
        || context.consumer_policy.version == 0
        || context.features.windows(2).any(|pair| pair[0] >= pair[1])
        || context.cfgs.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(ReviewError::InvalidContext);
    }
    Ok(())
}

fn valid_matching_policy(policy: &MatchingPolicy) -> bool {
    [
        &policy.profile,
        &policy.minimum_size_ratio,
        &policy.numerical_threshold,
        &policy.string_threshold,
        &policy.final_threshold,
    ]
    .iter()
    .all(|value| !value.is_empty())
        && matches!(
            policy.scoring.as_str(),
            "smaller_graph_overlap" | "cross_product_cosine"
        )
}

fn validate_member_expression(member: &MemberSnapshot) -> Result<(), ReviewError> {
    if member.signature.parameters.len() > 64 {
        return Err(ReviewError::InvalidSignature);
    }

    let (result, _) = validate_expression(
        &member.expression,
        ExpressionScope {
            signature: &member.signature,
            depth: 1,
        },
        512,
    )?;
    if result != member.signature.result {
        return Err(ReviewError::InvalidSignature);
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ExpressionScope<'a> {
    signature: &'a Signature,
    depth: usize,
}

fn validate_expression(
    expression: &Expression,
    scope: ExpressionScope<'_>,
    remaining: usize,
) -> Result<(ScalarType, usize), ReviewError> {
    if scope.depth > 64 || remaining == 0 {
        return Err(ReviewError::ExpressionBudget);
    }

    match expression {
        Expression::Formal { position, ty } => {
            if scope.signature.parameters.get(*position as usize) != Some(ty) {
                return Err(ReviewError::InvalidExpression);
            }
            Ok((*ty, 1))
        }
        Expression::Literal {
            ty,
            value,
            spelling,
        } => {
            if !valid_literal(value, spelling) {
                return Err(ReviewError::InvalidLiteral);
            }
            Ok((*ty, 1))
        }
        Expression::Unary { ty, operand, .. } => {
            let consumed = validate_operand(operand, *ty, scope, remaining - 1)?;
            Ok((*ty, 1 + consumed))
        }
        Expression::Binary {
            ty, left, right, ..
        } => {
            let first = validate_operand(left, *ty, scope, remaining - 1)?;
            let second = validate_operand(right, *ty, scope, remaining - 1 - first)?;
            Ok((*ty, 1 + first + second))
        }
    }
}

fn validate_operand(
    expression: &Expression,
    expected: ScalarType,
    scope: ExpressionScope<'_>,
    remaining: usize,
) -> Result<usize, ReviewError> {
    let (actual, consumed) = validate_expression(
        expression,
        ExpressionScope {
            depth: scope.depth + 1,
            ..scope
        },
        remaining,
    )?;
    if actual != expected {
        return Err(ReviewError::InvalidExpression);
    }
    Ok(consumed)
}

fn valid_literal(value: &str, spelling: &str) -> bool {
    if value.len() > 10
        || value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || value.len() > 1 && value.starts_with('0')
    {
        return false;
    }

    let Ok(semantic) = value.parse::<u32>() else {
        return false;
    };

    let token = spelling.strip_suffix("u32").unwrap_or(spelling);
    let (radix, digits) = if let Some(digits) = token.strip_prefix("0x") {
        (16, digits)
    } else if let Some(digits) = token.strip_prefix("0o") {
        (8, digits)
    } else if let Some(digits) = token.strip_prefix("0b") {
        (2, digits)
    } else {
        if !token.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            return false;
        }
        (10, token)
    };
    spelling_digits(digits, radix) == Some(semantic)
}

fn spelling_digits(digits: &str, radix: u32) -> Option<u32> {
    let mut parsed = 0_u32;
    let mut seen = false;
    for ch in digits.chars() {
        if ch == '_' {
            continue;
        }
        if !ch.is_ascii() {
            return None;
        }
        let digit = ch.to_digit(radix)?;
        parsed = parsed.checked_mul(radix)?.checked_add(digit)?;
        seen = true;
    }
    seen.then_some(parsed)
}

fn relative_source(source: &str) -> bool {
    !source.is_empty()
        && !source.starts_with('/')
        && !source.contains('\\')
        && !source.contains(':')
        && source
            .split('/')
            .all(|part| !matches!(part, "" | "." | ".."))
}

fn validate_pairs(
    snapshot: &FindingSnapshot,
    pairs: &[(&MemberKey, &MemberKey)],
) -> Result<(), ReviewError> {
    let members: Vec<_> = snapshot.members.iter().map(|member| &member.key).collect();
    let mut roster = BTreeSet::new();
    for (left, right) in pairs {
        let left = members
            .iter()
            .position(|key| key == left)
            .ok_or(ReviewError::InvalidPairRoster)?;
        let right = members
            .iter()
            .position(|key| key == right)
            .ok_or(ReviewError::InvalidPairRoster)?;
        if left == right || !roster.insert((left.min(right), left.max(right))) {
            return Err(ReviewError::InvalidPairRoster);
        }
    }

    for left in 0..members.len() {
        for right in left + 1..members.len() {
            if !roster.contains(&(left, right)) {
                return Err(ReviewError::IncompletePairRoster);
            }
        }
    }
    Ok(())
}

fn permits_growth(entry: &ReviewEntry, current: &FindingSnapshot) -> bool {
    entry.membership == MembershipPolicy::AllowGrowth
        && entry.finding.versions == current.versions
        && entry
            .finding
            .members
            .iter()
            .all(|member| current.members.contains(member))
        && current.members.len() > entry.finding.members.len()
        && current
            .members
            .iter()
            .all(|member| member.key.context == entry.finding.members[0].key.context)
}

fn stale_reason(previous: &FindingSnapshot, current: &FindingSnapshot) -> Option<StaleReason> {
    comparison::stale_reason(
        &previous.versions,
        &current.versions,
        previous
            .members
            .iter()
            .map(|member| (&member.key.locator, &member.key.context)),
        current
            .members
            .iter()
            .map(|member| (&member.key.locator, &member.key.context)),
    )
}

#[cfg(test)]
mod tests;
