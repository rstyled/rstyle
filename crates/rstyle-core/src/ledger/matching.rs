//! Applying, extending and pruning a ledger against the current candidates.

use super::{Entry, Ledger, site_key};
use crate::candidates::Candidate;
use serde::Serialize;
use std::collections::BTreeSet;

/// How an entry matched a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Matched {
    /// Same id: nothing about the candidate changed.
    Exact,
    /// An `allow_growth` entry whose members are all still there; `added` others joined.
    Growth { added: usize },
}

impl Matched {
    fn added(self) -> usize {
        match self {
            Matched::Exact => 0,
            Matched::Growth { added } => added,
        }
    }
}

/// A candidate the ledger hid, and the entry that did it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hidden {
    pub candidate: Candidate,
    pub entry_id: String,
    pub matched: Matched,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    pub kept: Vec<Candidate>,
    pub hidden: Vec<Hidden>,
}

/// What `cargo rstyle ignore` records beyond the candidate itself.
#[derive(Debug, Clone, Default)]
pub struct Opts {
    pub allow_growth: bool,
    pub reason: Option<String>,
}

pub(super) fn site_keys(c: &Candidate) -> BTreeSet<String> {
    c.sites.iter().map(site_key).collect()
}

pub(super) fn definition_keys(c: &Candidate) -> BTreeSet<String> {
    c.definitions.iter().map(|d| d.locator.clone()).collect()
}

/// How `entry` (stored under `id`) matches `c`: by id, or for `allow_growth` entries when
/// every recorded member is still in `c`. A candidate that lost a member never matches.
pub fn matches(id: &str, entry: &Entry, c: &Candidate) -> Option<Matched> {
    if entry.kind != c.kind {
        return None;
    }
    if c.id == id {
        return Some(Matched::Exact);
    }
    let sites: BTreeSet<String> = entry.sites.iter().cloned().collect();
    let definitions: BTreeSet<String> = entry.definitions.iter().cloned().collect();
    let (have_sites, have_definitions) = (site_keys(c), definition_keys(c));
    let grew = sites.is_subset(&have_sites) && definitions.is_subset(&have_definitions);
    let added =
        (have_sites.len() + have_definitions.len()).saturating_sub(sites.len() + definitions.len());

    (entry.allow_growth && grew).then_some(match added {
        0 => Matched::Exact,
        added => Matched::Growth { added },
    })
}

/// The entry that hides `c`, preferring the one it matches most exactly.
fn hiding_entry(ledger: &Ledger, c: &Candidate) -> Option<(String, Matched)> {
    ledger
        .entries
        .iter()
        .filter_map(|(id, e)| Some((id.clone(), matches(id, e, c)?)))
        .min_by_key(|(_, matched)| matched.added())
}

/// Splits `candidates` into those still worth reviewing and those the ledger hides.
pub fn apply(ledger: &Ledger, candidates: Vec<Candidate>) -> Applied {
    let (hidden, kept): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .map(|c| (hiding_entry(ledger, &c), c))
        .partition(|(entry, _)| entry.is_some());

    Applied {
        kept: kept.into_iter().map(|(_, c)| c).collect(),
        hidden: hidden
            .into_iter()
            .filter_map(|(entry, candidate)| {
                let (entry_id, matched) = entry?;
                Some(Hidden {
                    candidate,
                    entry_id,
                    matched,
                })
            })
            .collect(),
    }
}

fn entry_for(c: &Candidate, opts: Opts) -> Entry {
    let locators: BTreeSet<String> = c
        .sites
        .iter()
        .chain(&c.definitions)
        .map(|s| s.locator.clone())
        .collect();
    let (fns, sites, definitions) = match opts.allow_growth {
        true => (
            vec![],
            site_keys(c).into_iter().collect(),
            definition_keys(c).into_iter().collect(),
        ),
        false => (locators.into_iter().collect(), vec![], vec![]),
    };

    Entry {
        kind: c.kind,
        allow_growth: opts.allow_growth,
        reason: opts.reason,
        notes: vec![],
        fns,
        sites,
        definitions,
    }
}

/// `ledger` plus an entry dismissing `c` (replacing any entry for the same id).
pub fn add(mut ledger: Ledger, c: &Candidate, opts: Opts) -> Ledger {
    let notes = ledger.entries.get(&c.id).map(|e| e.notes.clone());
    let entry = Entry {
        notes: notes.unwrap_or_default(),
        ..entry_for(c, opts)
    };
    ledger.entries.insert(c.id.clone(), entry);

    ledger
}

/// Splits off the entries that match no current candidate. `candidates` must be the
/// unfiltered set, or entries hidden only by `--top` / `--min-score` would be dropped.
pub fn prune(ledger: Ledger, candidates: &[Candidate]) -> (Ledger, Vec<(String, Entry)>) {
    let (live, dead): (Vec<_>, Vec<_>) = ledger
        .entries
        .into_iter()
        .partition(|(id, e)| candidates.iter().any(|c| matches(id, e, c).is_some()));

    (
        Ledger {
            entries: live.into_iter().collect(),
        },
        dead,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidates::{Breakdown, CandidateKind, Site};
    use crate::ledger::candidate_id;

    fn site(locator: &str, shape: &str) -> Site {
        Site {
            path: "src/a.rs".into(),
            line: 1,
            id: locator.into(),
            name: locator.into(),
            locator: locator.into(),
            shape: Some(shape.into()),
        }
    }

    fn candidate(kind: CandidateKind, sites: &[(&str, &str)], defs: &[&str]) -> Candidate {
        let sites: Vec<Site> = sites.iter().map(|(l, s)| site(l, s)).collect();
        let definitions: Vec<Site> = defs.iter().map(|l| site(l, "0")).collect();

        Candidate {
            id: candidate_id(kind, &sites, &definitions),
            kind,
            score_milli: 1000,
            breakdown: Breakdown {
                support: sites.len(),
                lift_milli: 0,
                holes: 0,
                coverage_milli: 1000,
            },
            observation: String::new(),
            inference: String::new(),
            possible_refactor: String::new(),
            sites,
            definitions,
            counter_evidence: vec![],
        }
    }

    fn pair() -> Candidate {
        candidate(
            CandidateKind::TraitMethod,
            &[("k::a", "1"), ("k::b", "2")],
            &["k::A::m"],
        )
    }

    fn dismissed(c: &Candidate, allow_growth: bool) -> Ledger {
        add(
            Ledger::default(),
            c,
            Opts {
                allow_growth,
                reason: Some("reviewed".into()),
            },
        )
    }

    fn hidden_count(ledger: &Ledger, c: Candidate) -> usize {
        apply(ledger, vec![c]).hidden.len()
    }

    #[test]
    fn strict_entry_hides_only_the_identical_candidate() {
        let ledger = dismissed(&pair(), false);
        let kind = CandidateKind::GenericFn;
        let grown = candidate(
            CandidateKind::TraitMethod,
            &[("k::a", "1"), ("k::b", "2"), ("k::c", "3")],
            &["k::A::m"],
        );
        let shrunk = candidate(CandidateKind::TraitMethod, &[("k::a", "1")], &["k::A::m"]);
        let reshaped = candidate(
            CandidateKind::TraitMethod,
            &[("k::a", "9"), ("k::b", "2")],
            &["k::A::m"],
        );
        let more_defs = candidate(
            CandidateKind::TraitMethod,
            &[("k::a", "1"), ("k::b", "2")],
            &["k::A::m", "k::B::m"],
        );
        let other_kind = candidate(kind, &[("k::a", "1"), ("k::b", "2")], &["k::A::m"]);

        assert_eq!(hidden_count(&ledger, pair()), 1);
        for c in [grown, shrunk, reshaped, more_defs, other_kind] {
            assert_eq!(hidden_count(&ledger, c), 0);
        }
    }

    #[test]
    fn growth_entry_keeps_hiding_a_candidate_that_gains_members() {
        let ledger = dismissed(&pair(), true);
        let grown = candidate(
            CandidateKind::TraitMethod,
            &[("k::a", "1"), ("k::b", "2"), ("k::c", "3")],
            &["k::A::m", "k::B::m"],
        );
        let applied = apply(&ledger, vec![grown]);

        assert_eq!(applied.hidden[0].matched, Matched::Growth { added: 2 });
        assert_eq!(applied.hidden[0].entry_id, pair().id);
        assert_eq!(hidden_count(&ledger, pair()), 1);
    }

    #[test]
    fn growth_entry_resurfaces_when_a_member_is_lost_or_reshaped() {
        let ledger = dismissed(&pair(), true);
        let shrunk = candidate(CandidateKind::TraitMethod, &[("k::a", "1")], &["k::A::m"]);
        let reshaped = candidate(
            CandidateKind::TraitMethod,
            &[("k::a", "9"), ("k::b", "2"), ("k::c", "3")],
            &["k::A::m"],
        );
        let lost_definition = candidate(
            CandidateKind::TraitMethod,
            &[("k::a", "1"), ("k::b", "2"), ("k::c", "3")],
            &[],
        );

        for c in [shrunk, reshaped, lost_definition] {
            assert_eq!(hidden_count(&ledger, c), 0);
        }
    }

    #[test]
    fn apply_splits_kept_from_hidden() {
        let other = candidate(CandidateKind::GenericFn, &[("k::x", "5")], &[]);
        let applied = apply(&dismissed(&pair(), false), vec![other.clone(), pair()]);

        assert_eq!(applied.kept, vec![other]);
        assert_eq!(applied.hidden[0].matched, Matched::Exact);
    }

    #[test]
    fn add_records_hints_for_strict_and_members_for_growth() {
        let strict = &dismissed(&pair(), false).entries[&pair().id];
        let growth = &dismissed(&pair(), true).entries[&pair().id];

        assert_eq!(strict.fns, ["k::A::m", "k::a", "k::b"]);
        assert!(strict.sites.is_empty() && strict.definitions.is_empty());
        assert_eq!(growth.sites, ["k::a@1", "k::b@2"]);
        assert_eq!(growth.definitions, ["k::A::m"]);
        assert!(growth.fns.is_empty() && growth.allow_growth);
    }

    #[test]
    fn a_written_and_reread_ledger_still_hides() {
        for allow_growth in [false, true] {
            let text = crate::ledger::render(&dismissed(&pair(), allow_growth));
            let reread = crate::ledger::parse(&text).unwrap();

            assert_eq!(hidden_count(&reread, pair()), 1);
        }
    }

    #[test]
    fn dismissing_again_keeps_the_entrys_notes() {
        let mut ledger = dismissed(&pair(), false);
        ledger.entries.get_mut(&pair().id).unwrap().notes = vec!["see PR 12".into()];
        let again = add(ledger, &pair(), Opts::default());

        assert_eq!(again.entries[&pair().id].notes, ["see PR 12"]);
    }

    #[test]
    fn prune_drops_only_entries_that_match_nothing() {
        let gone = candidate(CandidateKind::GenericFn, &[("k::x", "5")], &[]);
        let ledger = add(dismissed(&pair(), true), &gone, Opts::default());
        let grown = candidate(
            CandidateKind::TraitMethod,
            &[("k::a", "1"), ("k::b", "2"), ("k::c", "3")],
            &["k::A::m"],
        );
        let (kept, removed) = prune(ledger, &[grown]);

        assert_eq!(kept.entries.keys().collect::<Vec<_>>(), [&pair().id]);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].0, gone.id);
    }
}
