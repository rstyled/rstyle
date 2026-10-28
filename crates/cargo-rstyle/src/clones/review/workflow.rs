//! Explicit review IO around checked helper findings; lookup never writes.
use super::*;
use crate::clones::{Function, Handoff, analysis, capture, report};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    #[serde(flatten)]
    summary: summary::Summary,
    pairs: Vec<Pair>,
}

#[derive(Serialize)]
struct Pair {
    first: String,
    second: String,
    state: &'static str,
    reason: Option<String>,
}

impl Report {
    pub(in crate::clones) fn render(&self) -> String {
        if !self.summary.requested() {
            return String::new();
        }
        let mut output = format!(
            "\nHelper reviews: {:?} coverage; {} visible, {} hidden; {} retained entries.",
            self.summary.coverage,
            self.summary.visible_findings,
            self.summary.hidden_findings,
            self.summary.retained_entries
        );
        for pair in &self.pairs {
            output.push_str(&format!(
                "\nReview {} <> {}: {}. {}",
                pair.first,
                pair.second,
                pair.state,
                pair.reason.as_deref().unwrap_or("")
            ));
        }
        output
    }

    pub(in crate::clones) fn visible_or(&self, unfiltered: usize) -> usize {
        self.summary
            .visible_or(self.summary.visible_findings, unfiltered)
    }
}

impl summary::Decision for Pair {
    fn state(&self) -> &'static str {
        self.state
    }
}

fn apply_helpers(
    helpers: &analysis::helper_reports::Report<'_>,
    ctx: &report::Context<'_>,
    store: Option<&mut ReviewStore>,
) -> Result<(Report, Vec<status::Record>), String> {
    let Some(store) =
        store.filter(|_| ctx.args.helper_proposals || ctx.args.review_status_requested())
    else {
        return Ok((summarize(Vec::new(), 0, false, false), Vec::new()));
    };

    let policy = matching_policy(
        ctx.matching_config,
        crate::clones::matching_profile(ctx.args).identity(),
    );
    let snapshots = snapshots(helpers, ctx, &policy);
    if !ctx.args.dismiss_helper_pair.is_empty() {
        dismiss(store, &snapshots, ctx.args)?;
    }

    let checked_store = store.checked().map_err(|error| format!("{error:?}"))?;
    let pairs = snapshots
        .iter()
        .map(|(first, second, snapshot)| lookup(&checked_store, first, second, snapshot))
        .collect();
    let records = status_workflow::helpers(store, &snapshots, helpers, ctx);
    Ok((
        summarize(
            pairs,
            store.entries.len(),
            ctx.args.helper_proposals,
            helpers.scope_complete(),
        ),
        records,
    ))
}

pub(in crate::clones) struct Inputs<'a, 'b> {
    pub compiler_advisory: Option<&'a crate::clones::advisory::Report>,
    pub analysis: &'a analysis::Report<'b>,
    pub helpers: &'a analysis::helper_reports::Report<'b>,
    pub dispatch: &'a analysis::dispatch_reports::Report<'b>,
    pub capabilities: &'a analysis::capability_reports::Report<'b>,
    pub effects: &'a analysis::effect_reports::Report<'b>,
    pub mutation: &'a analysis::mutation_reports::Report<'b>,
    pub indices: &'a analysis::declaration_indices::Indices,
}

pub(in crate::clones) struct Reports {
    pub compiler: CompilerReviews,
    pub density: super::density_workflow::Report,
    pub mutations: super::mutation_workflow::Report,
    pub sources: super::source_workflow::Reports,
    pub helpers: Report,
    pub abstractions: super::local_workflow::Report,
    pub status: Option<status::Report>,
    pub pruning: Option<prune::Report>,
}

pub(in crate::clones) struct CompilerReviews {
    patterns: super::compiler_patterns::Report,
    diagnostics: super::compiler_diagnostics::Report,
    baseline: Option<super::compiler_baseline::Report>,
}

impl CompilerReviews {
    fn new(
        inputs: &Inputs<'_, '_>,
        ctx: &report::Context<'_>,
        patterns: super::compiler_patterns::Report,
        diagnostics: super::compiler_diagnostics::Report,
    ) -> Result<Self, String> {
        let baseline = if ctx.args.compiler_baseline_requested() {
            Some(super::compiler_baseline::evaluate(
                inputs
                    .compiler_advisory
                    .ok_or("Compiler baseline requires compiler advisory")?,
                &diagnostics,
                ctx,
            )?)
        } else {
            None
        };
        Ok(Self {
            patterns,
            diagnostics,
            baseline,
        })
    }

    pub(in crate::clones) fn apply(
        self,
        advisory: Option<crate::clones::advisory::Report>,
        ctx: &report::Context<'_>,
    ) -> Option<crate::clones::advisory::Report> {
        advisory.map(|report| {
            report
                .with_reviews(self.patterns, self.diagnostics, self.baseline)
                .select_changed(ctx)
        })
    }
}

impl Inputs<'_, '_> {
    /// One load and atomic publication across all requested review families.
    /// A failed selector or snapshot leaves every retained entry untouched on disk.
    pub(in crate::clones) fn apply(self, ctx: &report::Context<'_>) -> Result<Reports, String> {
        let mut store = ctx.args.review_file.as_deref().map(load).transpose()?;
        let mut reports = evaluate(&self, ctx, store.as_mut())?;
        reports.pruning = pruning(store.as_mut(), reports.status.as_ref(), ctx)?;

        if (!ctx.args.dismiss_helper_pair.is_empty()
            || ctx.args.dismiss_enum_dispatch.is_some()
            || !ctx.args.dismiss_capability_pair.is_empty()
            || ctx.args.dismiss_parameter.is_some()
            || ctx.args.dismiss_length.is_some()
            || ctx.args.dismiss_spacing.is_some()
            || ctx.args.dismiss_mutation_per_target.is_some()
            || ctx.args.dismiss_mutated_targets.is_some()
            || ctx.args.dismiss_density.is_some()
            || !ctx.args.dismiss_compiler_pattern.is_empty()
            || !ctx.args.dismiss_compiler_diagnostic.is_empty()
            || reports
                .pruning
                .as_ref()
                .is_some_and(|report| report.removed_entries > 0))
            && let (Some(path), Some(store)) = (&ctx.args.review_file, &store)
        {
            save(path, store)?;
        }
        Ok(reports)
    }
}

fn evaluate(
    inputs: &Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    mut store: Option<&mut ReviewStore>,
) -> Result<Reports, String> {
    let (helpers, helper_records) = apply_helpers(inputs.helpers, ctx, store.as_deref_mut())?;
    let (abstractions, source_records) =
        super::local_workflow::apply(inputs, ctx, store.as_deref_mut())?;
    let (sources, source_rule_records) =
        super::source_workflow::apply(inputs, ctx, store.as_deref_mut())?;
    let policy = evaluate_policy_reviews(inputs, ctx, store)?;
    let compiler = CompilerReviews::new(
        inputs,
        ctx,
        policy.compiler_patterns,
        policy.compiler_diagnostics,
    )?;
    let records = [
        helper_records,
        source_records,
        source_rule_records,
        policy.records,
    ]
    .into_iter()
    .flatten()
    .collect();
    let status = ctx
        .args
        .review_status_requested()
        .then(|| status::Report::new(records, true));

    Ok(Reports {
        compiler,
        density: policy.density,
        mutations: policy.mutations,
        sources,
        helpers,
        abstractions,
        status,
        pruning: None,
    })
}

struct PolicyReviews {
    mutations: super::mutation_workflow::Report,
    density: super::density_workflow::Report,
    compiler_patterns: super::compiler_patterns::Report,
    compiler_diagnostics: super::compiler_diagnostics::Report,
    records: Vec<status::Record>,
}

fn evaluate_policy_reviews(
    inputs: &Inputs<'_, '_>,
    ctx: &report::Context<'_>,
    mut store: Option<&mut ReviewStore>,
) -> Result<PolicyReviews, String> {
    let (mutations, mutation_records) =
        super::mutation_workflow::apply(inputs, ctx, store.as_deref_mut())?;
    let (density, density_records) =
        super::density_workflow::apply(inputs, ctx, store.as_deref_mut())?;
    let (compiler_patterns, compiler_records) =
        super::compiler_patterns::apply(inputs, ctx, store.as_deref_mut())?;
    let (compiler_diagnostics, diagnostic_records) =
        super::compiler_diagnostics::apply(inputs, ctx, store)?;
    let records = mutation_records
        .into_iter()
        .chain(density_records)
        .chain(compiler_records)
        .chain(diagnostic_records)
        .collect();

    Ok(PolicyReviews {
        mutations,
        density,
        compiler_patterns,
        compiler_diagnostics,
        records,
    })
}

fn pruning(
    store: Option<&mut ReviewStore>,
    status: Option<&status::Report>,
    ctx: &report::Context<'_>,
) -> Result<Option<prune::Report>, String> {
    if !ctx.args.review_prune {
        return Ok(None);
    }
    if !ctx.capture.failures.is_empty() || !ctx.handoff.failures.is_empty() {
        return Err(
            "Review pruning requires a successful capture and assessment; no entries changed"
                .into(),
        );
    }

    let store = store.ok_or("Review pruning requires a review file")?;
    let status = status.ok_or("Review pruning requires retained-entry assessment")?;
    prune::apply(store, status, ctx.args.review_prune_dry_run).map(Some)
}

pub(super) type Snapshot = (String, String, Result<FindingSnapshot, String>);

fn snapshots(
    helpers: &analysis::helper_reports::Report<'_>,
    ctx: &report::Context<'_>,
    policy: &MatchingPolicy,
) -> Vec<Snapshot> {
    let records: BTreeMap<_, _> = ctx
        .handoff
        .assessed
        .iter()
        .map(|record| (record.key.to_string(), record))
        .collect();
    let mut resolver = MemberResolver {
        records,
        members: BTreeMap::new(),
        ctx,
        policy,
    };
    helpers
        .finding_pairs()
        .map(|(first, second)| {
            let snapshot = resolver.pair_snapshot(first, second);
            (first.into(), second.into(), snapshot)
        })
        .collect()
}

struct MemberResolver<'a, 'b> {
    records: BTreeMap<String, &'a Function>,
    members: BTreeMap<crate::clones::FunctionKey, Result<MemberSnapshot, String>>,
    ctx: &'a report::Context<'b>,
    policy: &'a MatchingPolicy,
}

impl MemberResolver<'_, '_> {
    fn pair_snapshot(&mut self, first: &str, second: &str) -> Result<FindingSnapshot, String> {
        let mut snapshots = Vec::new();
        for key in [first, second] {
            let record = self
                .records
                .get(key)
                .ok_or("Missing reviewed helper member")?;
            let snapshot = self.members.entry(record.key).or_insert_with(|| {
                let unit = unit(self.ctx.handoff, self.ctx.capture, record)?;
                super::adapter::member(
                    &record.facts,
                    unit,
                    &self.ctx.capture.plan,
                    super::adapter::Policies {
                        matching: self.policy,
                        consumer: &consumer_policy(),
                    },
                )
            });
            snapshots.push(snapshot.clone()?);
        }
        snapshots.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(FindingSnapshot {
            versions: versions(),
            members: snapshots,
        })
    }
}

pub(super) fn unit<'a>(
    handoff: &Handoff,
    capture: &'a capture::Capture,
    record: &Function,
) -> Result<&'a capture::Unit, String> {
    let context = handoff
        .unit_contexts
        .iter()
        .find(|context| context.id == record.key.unit())
        .ok_or("Missing review compilation context")?;
    capture
        .units
        .get(context.observed_unit)
        .ok_or("Invalid review compilation context".into())
}

pub(super) fn consumer_policy() -> ConsumerPolicy {
    ConsumerPolicy {
        id: "scalar-expression-helper".into(),
        version: 1,
        min_builtin_operations: 2,
        max_literal_holes: 1,
    }
}

pub(super) fn versions() -> Versions {
    Versions {
        detector: "scalar-expression-helper".into(),
        detector_version: 1,
        identity_version: 1,
        locator_version: 1,
        context_version: 1,
    }
}

pub(super) fn matching_policy(
    config: &rstyle_ccgraph::CcgraphConfig,
    profile: &str,
) -> MatchingPolicy {
    let rational = |fraction: rstyle_ccgraph::Frac| {
        format!("{}/{}", fraction.numerator(), fraction.denominator())
    };
    MatchingPolicy {
        profile: profile.into(),
        minimum_size_ratio: rational(config.minimum_size_ratio),
        numerical_threshold: rational(config.numerical_threshold),
        string_threshold: rational(config.string_threshold),
        final_threshold: rational(config.final_threshold),
        scoring: match config.scoring {
            rstyle_ccgraph::Scoring::SmallerGraphOverlap => "smaller_graph_overlap",
            rstyle_ccgraph::Scoring::CrossProductCosine => "cross_product_cosine",
        }
        .into(),
    }
}

fn checked(snapshot: &FindingSnapshot) -> Result<LiveFinding<'_>, String> {
    let [left, right] = snapshot.members.as_slice() else {
        return Err("A helper review requires exactly two independently checked members".into());
    };
    LiveFinding::checked(snapshot, &[(&left.key, &right.key)]).map_err(|error| format!("{error:?}"))
}

fn dismiss(
    store: &mut ReviewStore,
    snapshots: &[Snapshot],
    args: &crate::clones::Args,
) -> Result<(), String> {
    let [first, second] = args.dismiss_helper_pair.as_slice() else {
        return Err("A dismissal requires exactly two saved-run endpoint handles".into());
    };
    let reason = args
        .review_reason
        .as_deref()
        .filter(|reason| !reason.trim().is_empty())
        .ok_or("A dismissal requires a nonempty review reason")?;

    let (_, _, snapshot) = snapshots
        .iter()
        .find(|(left, right, _)| {
            (left == first && right == second) || (left == second && right == first)
        })
        .ok_or("Selected saved-run endpoints do not form a helper finding")?;
    let current = checked(snapshot.as_ref().map_err(Clone::clone)?)?;
    let membership = if args.review_allow_growth {
        MembershipPolicy::AllowGrowth
    } else {
        MembershipPolicy::Exact
    };
    store
        .record_dismissal(&current, membership, reason.into())
        .map_err(|error| format!("{error:?}"))
}

fn lookup(
    store: &ValidatedStore<'_>,
    first: &str,
    second: &str,
    snapshot: &Result<FindingSnapshot, String>,
) -> Pair {
    let current = snapshot.as_ref().map_err(Clone::clone).and_then(checked);
    let input = match &current {
        Ok(current) => ReviewInput::Checked(current),
        Err(reason) => ReviewInput::Unavailable { reason },
    };
    let (state, reason) = match store.lookup(input) {
        Lookup::HiddenExact(entry) => ("hidden_exact", Some(entry.reason.clone())),
        Lookup::HiddenGrowth(entry) => ("hidden_growth", Some(entry.reason.clone())),
        Lookup::VisibleNew => ("visible_new", None),
        Lookup::Stale(entries) => (
            "stale",
            Some(
                entries
                    .iter()
                    .map(|stale| format!("{:?}: {}", stale.reason, stale.entry.reason))
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
        ),
        Lookup::Unavailable { reason } => ("unavailable", Some(reason)),
    };
    Pair {
        first: first.into(),
        second: second.into(),
        state,
        reason,
    }
}

fn summarize(
    pairs: Vec<Pair>,
    retained_entries: usize,
    requested: bool,
    scope_complete: bool,
) -> Report {
    let summary = summary::summarize(
        &pairs,
        retained_entries,
        summary::Scope {
            requested,
            complete: scope_complete,
        },
        summary::Contract {
            schema: "rstyle.finding-review/1",
            incomplete_scope: "Potential helper members remain unavailable; see helper_proposals coverage",
        },
    );
    Report { summary, pairs }
}

fn load(path: &Path) -> Result<ReviewStore, String> {
    match baseline_io::read_bounded(path) {
        Ok(bytes) => {
            ReviewStore::from_json(std::str::from_utf8(&bytes).map_err(|error| error.to_string())?)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ReviewStore::default()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn save(path: &Path, store: &ReviewStore) -> Result<(), String> {
    let bytes = store.to_json()?;
    publication::write(path, bytes.as_bytes(), publication::Mode::Replace)
}

impl crate::clones::analysis::Completion for Report {
    fn permits_success(&self) -> bool {
        self.summary.permits_success()
    }
}
