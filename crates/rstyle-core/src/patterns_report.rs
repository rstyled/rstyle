//! Output of `cargo rstyle patterns`: text and JSON.

use crate::candidates::{self, Candidate, Site, Suppressed};
use crate::cluster::Params;
use crate::facts::FnFacts;
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
    /// Patterns that look like candidates but are already abstracted.
    #[serde(default)]
    pub suppressed: Vec<Suppressed>,
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
}

/// Keeps groups at or above `min_score_milli`, then the first `top` (all when `None`).
pub fn build(groups: Vec<SigGroup>, min_score_milli: u32, top: Option<usize>) -> PatternsReport {
    let kept = groups
        .into_iter()
        .filter(|g| g.score_milli >= min_score_milli)
        .take(top.unwrap_or(usize::MAX))
        .collect();
    PatternsReport {
        signature_groups: kept,
        candidates: vec![],
        suppressed: vec![],
    }
}

/// Full pipeline over driver facts: prepare, layer 1, layer 2, then filter and rank.
pub fn run(facts: &[FnFacts], options: &Options) -> PatternsReport {
    // Lib and test targets compile the same functions: dedupe like the style path does.
    let facts = crate::facts::merge(facts.to_vec());
    let prepared = prep::hide_implements(prep::prepare(&facts), &options.hide_implements);
    let groups = sigmine::mine(&prepared);
    let mined = candidates::mine(&prepared, &groups, &options.params);
    let candidates = mined
        .candidates
        .into_iter()
        .filter(|c| c.score_milli >= options.min_score_milli)
        .take(options.top.unwrap_or(usize::MAX))
        .collect();

    PatternsReport {
        candidates,
        suppressed: mined.suppressed,
        ..build(groups, options.min_score_milli, options.top)
    }
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
        "#{} {}  score {}  [support {}, lift {}, holes {}, coverage {}]",
        index + 1,
        kind.as_ref()
            .and_then(|k| k.as_str())
            .unwrap_or("candidate"),
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
    let groups = section(
        format!("{} signature groups", report.signature_groups.len()),
        report.signature_groups.iter().map(group_text).collect(),
    );
    let found: Vec<String> = report
        .candidates
        .iter()
        .enumerate()
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
            "{} suppressed (already abstracted)\n{}",
            lines.len(),
            lines.join("\n")
        ));
    }
    parts.join("\n\n")
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
