//! Explicit review IO around checked helper findings; lookup never writes.
use super::*;
use crate::clones::{Function, Handoff, analysis, capture, report};
use serde::Serialize;
use std::{collections::BTreeMap, io::Write, path::Path};

#[derive(Serialize)]
pub(in crate::clones) struct Report {
    schema: &'static str,
    execution: analysis::Execution,
    coverage: analysis::Coverage,
    findings: usize,
    hidden_findings: usize,
    pub visible_findings: usize,
    retained_entries: usize,
    pairs: Vec<Pair>,
    reasons: Vec<&'static str>,
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
        if self.execution == analysis::Execution::NotRequested {
            return String::new();
        }
        let mut output = format!(
            "\nHelper reviews: {:?} coverage; {} visible, {} hidden; {} retained entries.",
            self.coverage, self.visible_findings, self.hidden_findings, self.retained_entries
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

    pub(in crate::clones) fn permits_success(&self) -> bool {
        self.execution == analysis::Execution::NotRequested
            || self.coverage == analysis::Coverage::Complete
    }

    pub(in crate::clones) fn visible_or(&self, unfiltered: usize) -> usize {
        if self.execution == analysis::Execution::NotRequested {
            unfiltered
        } else {
            self.visible_findings
        }
    }
}

pub(in crate::clones) fn apply(
    helpers: &analysis::helper_reports::Report<'_>,
    ctx: &report::Context<'_>,
) -> Result<Report, String> {
    let Some(path) = &ctx.args.review_file else {
        return Ok(summarize(Vec::new(), 0, false, false));
    };

    let mut store = load(path)?;
    let policy = matching_policy(
        ctx.matching_config,
        crate::clones::matching_profile(ctx.args).identity(),
    );
    let snapshots = snapshots(helpers, ctx, &policy);
    if !ctx.args.dismiss_helper_pair.is_empty() {
        dismiss(&mut store, &snapshots, ctx.args)?;
        save(path, &store)?;
    }

    let checked_store = store.checked().map_err(|error| format!("{error:?}"))?;
    let pairs = snapshots
        .iter()
        .map(|(first, second, snapshot)| lookup(&checked_store, first, second, snapshot))
        .collect();
    Ok(summarize(
        pairs,
        store.entries.len(),
        true,
        helpers.scope_complete(),
    ))
}

type Snapshot = (String, String, Result<FindingSnapshot, String>);

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

fn unit<'a>(
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

fn consumer_policy() -> ConsumerPolicy {
    ConsumerPolicy {
        id: "scalar-expression-helper".into(),
        version: 1,
        min_builtin_operations: 2,
        max_literal_holes: 1,
    }
}

fn versions() -> Versions {
    Versions {
        detector: "scalar-expression-helper".into(),
        detector_version: 1,
        identity_version: 1,
        locator_version: 1,
        context_version: 1,
    }
}

fn matching_policy(config: &rstyle_ccgraph::CcgraphConfig, profile: &str) -> MatchingPolicy {
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
    let hidden_findings = pairs
        .iter()
        .filter(|pair| matches!(pair.state, "hidden_exact" | "hidden_growth"))
        .count();
    let complete = scope_complete && pairs.iter().all(|pair| pair.state != "unavailable");
    Report {
        schema: "rstyle.finding-review/1",
        execution: if requested {
            analysis::Execution::Completed
        } else {
            analysis::Execution::NotRequested
        },
        coverage: if requested && complete {
            analysis::Coverage::Complete
        } else {
            analysis::Coverage::Incomplete
        },
        findings: pairs.len(),
        hidden_findings,
        visible_findings: pairs.len() - hidden_findings,
        retained_entries,
        reasons: if requested && !scope_complete {
            vec!["Potential helper members remain unavailable; see helper_proposals coverage"]
        } else {
            Vec::new()
        },
        pairs,
    }
}

fn load(path: &Path) -> Result<ReviewStore, String> {
    match std::fs::read_to_string(path) {
        Ok(bytes) => ReviewStore::from_json(&bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ReviewStore::default()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn save(path: &Path, store: &ReviewStore) -> Result<(), String> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    let bytes = store.to_json()?;
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|error| error.to_string())?;
    file.write_all(bytes.as_bytes())
        .map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    file.persist(path).map_err(|error| error.to_string())?;
    Ok(())
}
