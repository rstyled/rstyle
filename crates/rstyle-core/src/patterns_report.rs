//! Output of `cargo rstyle patterns`: text and JSON.

use crate::candidates::{self, Candidate, CandidateKind, Site, Suppressed};
use crate::cluster::Params;
use crate::facts::FnFacts;
use crate::ledger::{self, Hidden, Ledger, Matched};
use crate::prep;
use crate::sigmine::{self, Member, SigGroup};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PatternsReport {
    /// Signature groups (layer 1), best first.
    pub signature_groups: Vec<SigGroup>,
    /// Ranked abstraction candidates (layer 2), best first. Absent facts give none.
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    /// Patterns excluded by proposal policy, including already abstracted work.
    #[serde(default)]
    pub suppressed: Vec<Suppressed>,
    /// Candidates the ignore ledger hid, with the entry responsible.
    #[serde(default)]
    pub ignored: Vec<Ignored>,
    #[serde(default)]
    pub ignored_count: usize,
}

/// A candidate hidden by a ledger entry.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Ignored {
    pub id: String,
    pub kind: CandidateKind,
    pub entry_id: String,
    pub matched_by: Matched,
    pub reason: Option<String>,
    pub sites: Vec<Site>,
}

/// What to show and what to pretend.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub min_score_milli: u32,
    pub top: Option<usize>,
    /// Trait-hiding oracle: forget `implements` links whose trait method id contains one of these.
    pub hide_implements: Vec<String>,
    /// Clustering thresholds; the defaults are `Params::default()`.
    pub params: Params,
    /// Reviewed candidates to hide, applied before `min_score_milli` and `top`.
    pub ledger: Option<Ledger>,
}

/// Select ranked values by inclusive score, then limit, preserving their order.
pub fn select_scored<T>(
    items: impl IntoIterator<Item = T>,
    score: impl Fn(&T) -> u32,
    minimum: u32,
    top: Option<usize>,
) -> impl Iterator<Item = T> {
    items
        .into_iter()
        .filter(move |item| score(item) >= minimum)
        .take(top.unwrap_or(usize::MAX))
}

/// Keeps groups at or above `min_score_milli`, then the first `top` (all when `None`).
pub fn build(groups: Vec<SigGroup>, min_score_milli: u32, top: Option<usize>) -> PatternsReport {
    let kept = select_scored(groups, |g| g.score_milli, min_score_milli, top).collect();
    PatternsReport {
        signature_groups: kept,
        candidates: vec![],
        suppressed: vec![],
        ignored: vec![],
        ignored_count: 0,
    }
}

/// Layers 1 and 2 over driver facts, before any display filter or the ignore ledger.
fn mine_all(
    facts: Vec<FnFacts>,
    options: &Options,
) -> (Vec<SigGroup>, candidates::Mined, Vec<FnFacts>) {
    // Lib and test targets compile the same functions: dedupe like the style path does.
    let facts = crate::facts::merge(facts);
    let prepared = prep::hide_implements(prep::prepare_owned(facts), &options.hide_implements);
    let groups = sigmine::mine(&prepared);
    let mined = candidates::mine(&prepared, &groups, &options.params);

    (groups, mined, prepared)
}

/// Every candidate, unfiltered: what `ignore` resolves ids against and `--prune` compares to.
pub fn all_candidates(facts: &[FnFacts], options: &Options) -> Vec<Candidate> {
    mine_all(facts.to_vec(), options).1.candidates
}

fn ignored_entry(ledger: &Ledger, hidden: Hidden) -> Ignored {
    let c = hidden.candidate;

    Ignored {
        reason: ledger.entries[&hidden.entry_id].reason.clone(),
        id: c.id,
        kind: c.kind,
        entry_id: hidden.entry_id,
        matched_by: hidden.matched,
        sites: c.sites,
    }
}

/// Full pipeline over driver facts: layers 1 and 2, the ignore ledger, then filter and rank.
pub fn run(facts: &[FnFacts], options: &Options) -> PatternsReport {
    run_owned(facts.to_vec(), options)
}

/// Full proposal policy over owned compiler facts, preserving buffers through preparation.
pub fn run_owned(facts: Vec<FnFacts>, options: &Options) -> PatternsReport {
    run_owned_and_retain(facts, options).0
}

/// Return the exact prepared observations used for proposals without mining or copying again.
pub fn run_owned_and_retain(
    facts: Vec<FnFacts>,
    options: &Options,
) -> (PatternsReport, Vec<FnFacts>) {
    let (groups, mined, prepared) = mine_all(facts, options);
    let ledger = options.ledger.clone().unwrap_or_default();
    let applied = ledger::apply(&ledger, mined.candidates);
    let candidates = select_scored(
        applied.kept,
        |c| c.score_milli,
        options.min_score_milli,
        options.top,
    )
    .collect();
    let ignored: Vec<Ignored> = applied
        .hidden
        .into_iter()
        .map(|h| ignored_entry(&ledger, h))
        .collect();

    let report = PatternsReport {
        candidates,
        suppressed: mined.suppressed,
        ignored_count: ignored.len(),
        ignored,
        ..build(groups, options.min_score_milli, options.top)
    };
    (report, prepared)
}

fn site(m: &Member) -> String {
    format!("    {}:{}  {}", m.path, m.line, m.name)
}

fn group_text(group: &SigGroup) -> String {
    let mut lines = vec![format!(
        "{}  [{} types, score {:.2}{}]",
        group.key,
        group.self_types,
        f64::from(group.score_milli) / 1000.0,
        if group.same_name { ", same name" } else { "" }
    )];
    lines.extend(group.members.iter().map(site));
    if !group.traited.is_empty() {
        lines.push(format!("    ({} via trait impls)", group.traited.len()));
    }
    lines.join("\n")
}

fn site_line(s: &Site) -> String {
    format!("    {}:{}  {}", s.path, s.line, s.name)
}

fn milli(value: u32) -> String {
    format!("{:.2}", f64::from(value) / 1000.0)
}

fn headline(index: usize, c: &Candidate) -> String {
    let kind = serde_json::to_value(c.kind).ok();
    let b = &c.breakdown;
    format!(
        "#{} {}  id {}  score {}  [support {}, lift {}, holes {}, coverage {}]",
        index + 1,
        kind.as_ref()
            .and_then(|k| k.as_str())
            .unwrap_or("candidate"),
        c.id,
        milli(c.score_milli),
        b.support,
        milli(b.lift_milli),
        b.holes,
        milli(b.coverage_milli)
    )
}

fn candidate_text(index: usize, c: &Candidate) -> String {
    let counter = (!c.counter_evidence.is_empty()).then(|| "  counter evidence:".to_string());
    let lines = [
        headline(index, c),
        format!("  observation: {}", c.observation),
        format!("  inference: {}", c.inference),
        format!("  possible refactor: {}", c.possible_refactor),
        "  sites:".into(),
    ]
    .into_iter()
    .chain(c.sites.iter().map(site_line))
    .chain(["  definitions:".to_string()])
    .chain(c.definitions.iter().map(site_line))
    .chain(counter)
    .chain(c.counter_evidence.iter().map(|e| format!("    - {e}")));
    lines.collect::<Vec<_>>().join("\n")
}

fn suppressed_text(s: &Suppressed) -> String {
    let sites: Vec<String> = s
        .sites
        .iter()
        .map(|x| format!("{}:{}  {}", x.path, x.line, x.name))
        .collect();
    format!("  {} ({})", s.reason, sites.join(", "))
}

fn section(title: String, body: Vec<String>) -> String {
    std::iter::once(title)
        .chain(body)
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn text(report: &PatternsReport) -> String {
    text_selected(report, |_| true)
}

/// Present review visibility without copying the report or its candidate values.
pub fn text_selected(report: &PatternsReport, visible: impl Fn(usize) -> bool) -> String {
    text_selected_with_groups(report, visible, |_| true)
}

/// Present selected groups and candidates while borrowing the complete evaluation.
pub fn text_selected_with_groups(
    report: &PatternsReport,
    visible: impl Fn(usize) -> bool,
    group_visible: impl Fn(usize) -> bool,
) -> String {
    let groups: Vec<_> = report
        .signature_groups
        .iter()
        .enumerate()
        .filter(|(index, _)| group_visible(*index))
        .map(|(_, group)| group_text(group))
        .collect();
    let groups = section(format!("{} signature groups", groups.len()), groups);
    let found: Vec<String> = report
        .candidates
        .iter()
        .enumerate()
        .filter(|(i, _)| visible(*i))
        .map(|(i, c)| candidate_text(i, c))
        .collect();
    let mut parts = vec![
        groups,
        section(format!("{} candidates", found.len()), found),
    ];

    if !report.suppressed.is_empty() {
        let lines = report
            .suppressed
            .iter()
            .map(suppressed_text)
            .collect::<Vec<_>>();
        parts.push(format!(
            "{} suppressed by proposal policy\n{}",
            lines.len(),
            lines.join("\n")
        ));
    }
    if report.ignored_count > 0 {
        parts.push(format!(
            "{} candidates hidden by the ignore ledger (--show-ignored to list)",
            report.ignored_count
        ));
    }
    parts.join("\n\n")
}

fn how(m: Matched) -> String {
    match m {
        Matched::Exact => "unchanged".to_string(),
        Matched::Growth { added } => format!("grew by {added} members"),
    }
}

fn ignored_text(i: &Ignored) -> String {
    let reason = i.reason.as_ref().map(|r| format!("  \"{r}\""));
    let head = format!(
        "{} {}  ({}){}",
        i.kind.name(),
        i.id,
        how(i.matched_by),
        reason.unwrap_or_default()
    );

    std::iter::once(head)
        .chain(i.sites.iter().map(site_line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Candidates the ledger hid, for `--show-ignored`.
pub fn show_ignored(report: &PatternsReport) -> String {
    let body: Vec<String> = report.ignored.iter().map(ignored_text).collect();

    section(format!("{} ignored candidates", body.len()), body)
}

pub fn json(report: &PatternsReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidates::tests_support::corpus;

    fn group(key: &str, score: u32) -> SigGroup {
        SigGroup {
            key: key.into(),
            members: vec![],
            traited: vec![],
            self_types: 2,
            same_name: false,
            specificity_milli: score,
            score_milli: score,
        }
    }

    #[test]
    fn min_score_and_top_filter() {
        let groups = vec![group("a", 900), group("b", 500), group("c", 100)];
        assert_eq!(build(groups.clone(), 400, None).signature_groups.len(), 2);
        assert_eq!(build(groups, 0, Some(1)).signature_groups[0].key, "a");
    }

    #[test]
    fn scored_selection_is_inclusive_stable_and_accepts_zero_limit() {
        let values = [900, 500, 500, 100];
        assert_eq!(
            select_scored(values, |score| *score, 500, Some(2)).collect::<Vec<_>>(),
            [900, 500]
        );
        assert_eq!(select_scored(values, |score| *score, 500, None).count(), 3);
        assert_eq!(select_scored(values, |score| *score, 0, Some(0)).count(), 0);
        let report = build(vec![group("kept", 900), group("hidden", 500)], 0, None);
        let rendered = text_selected_with_groups(&report, |_| true, |index| index == 0);
        assert!(rendered.starts_with("1 signature groups"));
        assert!(rendered.contains("kept"));
        assert!(!rendered.contains("hidden"));
        assert_eq!(report.signature_groups.len(), 2);
    }

    #[test]
    fn text_lists_header_and_sites() {
        let mut g = group("fn(&Self) -> String", 1500);
        g.members.push(Member {
            path: "src/lib.rs".into(),
            line: 7,
            id: "k::Dog::bark".into(),
            name: "Dog::bark".into(),
            self_ty: "k::Dog".into(),
        });
        let out = text(&PatternsReport {
            signature_groups: vec![g],
            candidates: vec![],
            suppressed: vec![],
            ignored: vec![],
            ignored_count: 0,
        });
        assert!(out.starts_with("1 signature groups"));
        assert!(out.contains("score 1.50"));
        assert!(out.contains("src/lib.rs:7  Dog::bark"));
    }

    #[test]
    fn duplicate_target_facts_do_not_duplicate_sites() {
        let once = run(&corpus(), &Options::default());
        let doubled: Vec<FnFacts> = corpus().into_iter().chain(corpus()).collect();
        let twice = run(&doubled, &Options::default());
        assert_eq!(once.candidates.len(), 1);
        assert_eq!(twice.candidates, once.candidates);
        let c = &twice.candidates[0];
        let ids: std::collections::BTreeSet<_> =
            c.sites.iter().map(|s| (&s.path, s.line, &s.id)).collect();
        assert_eq!(ids.len(), c.sites.len());
    }

    #[test]
    fn owned_pipeline_retains_candidate_identity_and_review_policy() {
        let baseline = run(&corpus(), &Options::default());
        assert_eq!(run_owned(corpus(), &Options::default()), baseline);
        let options = dismissing(&baseline.candidates[0], false);
        let reviewed = run_owned(corpus(), &options);
        assert_eq!(reviewed.ignored_count, 1);
        assert_eq!(reviewed.ignored[0].id, baseline.candidates[0].id);
        assert!(reviewed.candidates.is_empty());
    }

    #[test]
    fn retained_pipeline_returns_original_graph_buffers_and_the_same_proposals() {
        let facts = corpus();
        let buffers: std::collections::BTreeMap<_, _> = facts
            .iter()
            .map(|facts| {
                let graph = facts
                    .pdg
                    .as_ref()
                    .map(|graph| (graph.nodes.as_ptr(), graph.edges.as_ptr()));
                (facts.id.clone(), (facts.source.as_ptr(), graph))
            })
            .collect();
        let (report, prepared) = run_owned_and_retain(facts, &Options::default());
        assert_eq!(report, run(&corpus(), &Options::default()));
        assert_eq!(prepared.len(), buffers.len());
        for facts in &prepared {
            assert_eq!(
                buffers[&facts.id],
                (
                    facts.source.as_ptr(),
                    facts
                        .pdg
                        .as_ref()
                        .map(|graph| (graph.nodes.as_ptr(), graph.edges.as_ptr()))
                )
            );
        }
    }

    #[test]
    fn candidates_carry_a_stable_id_and_site_shapes() {
        let once = run(&corpus(), &Options::default());
        let reversed: Vec<FnFacts> = corpus().into_iter().rev().collect();
        let again = run(&reversed, &Options::default());
        let c = &once.candidates[0];

        assert_eq!(c.id.len(), 12);
        assert_eq!(c.id, again.candidates[0].id);
        assert!(
            c.sites
                .iter()
                .all(|s| s.shape.is_some() && !s.locator.is_empty())
        );
    }

    fn dismissing(c: &Candidate, allow_growth: bool) -> Options {
        let opts = ledger::Opts {
            allow_growth,
            reason: Some("coincidence".into()),
        };

        Options {
            ledger: Some(ledger::add(Ledger::default(), c, opts)),
            ..Options::default()
        }
    }

    #[test]
    fn ledger_hides_a_dismissed_candidate_and_says_why() {
        let first = run(&corpus(), &Options::default());
        let c = &first.candidates[0];
        let report = run(&corpus(), &dismissing(c, false));

        assert!(report.candidates.is_empty());
        assert_eq!(report.ignored_count, 1);
        assert_eq!(report.ignored[0].id, c.id);
        assert_eq!(report.ignored[0].reason.as_deref(), Some("coincidence"));
        assert_eq!(report.ignored[0].matched_by, Matched::Exact);
    }

    #[test]
    fn ledger_hides_before_min_score_and_top() {
        let c = run(&corpus(), &Options::default()).candidates[0].clone();
        let options = Options {
            top: Some(1),
            ..dismissing(&c, true)
        };

        assert!(run(&corpus(), &options).candidates.is_empty());
    }

    #[test]
    fn all_candidates_ignores_the_ledger_and_display_filters() {
        let c = run(&corpus(), &Options::default()).candidates[0].clone();
        let options = Options {
            min_score_milli: u32::MAX,
            ..dismissing(&c, false)
        };

        assert_eq!(all_candidates(&corpus(), &options), vec![c]);
    }

    #[test]
    fn text_shows_ids_and_the_hidden_count() {
        let c = run(&corpus(), &Options::default()).candidates[0].clone();
        let shown = text(&run(&corpus(), &Options::default()));
        let hidden = run(&corpus(), &dismissing(&c, false));

        assert!(shown.contains(&format!("id {}", c.id)));
        assert!(!shown.contains("hidden by the ignore ledger"));
        assert!(text(&hidden).contains("1 candidates hidden by the ignore ledger"));
        assert!(show_ignored(&hidden).contains(&c.id));
        assert!(show_ignored(&hidden).contains("\"coincidence\""));
    }

    #[test]
    fn params_option_controls_clustering() {
        let strict = Options {
            params: Params {
                threshold_milli: 1001,
                ..Params::default()
            },
            ..Options::default()
        };
        assert!(run(&corpus(), &strict).candidates.is_empty());
        assert_eq!(run(&corpus(), &Options::default()).candidates.len(), 1);
    }
}
