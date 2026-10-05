//! Resolving ledger entries back to the functions they were about.
//!
//! The file stores no `file:line`; it would go stale. Locations come from the current facts.

use super::{Entry, Ledger, matches};
use crate::candidates::{Candidate, CandidateKind};
use crate::facts::FnFacts;
use crate::{locator, wl};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberState {
    /// Found; the entry recorded no shape to compare (strict entries).
    Found,
    Unchanged,
    ShapeChanged,
    Gone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub locator: String,
    pub state: MemberState,
    /// Current `path:line`, when the function still exists.
    pub at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryStatus {
    pub id: String,
    pub kind: CandidateKind,
    pub reason: Option<String>,
    /// Some current candidate still matches (the entry is hiding something).
    pub active: bool,
    pub members: Vec<Member>,
    /// For a stale entry: members of the closest current candidate that the entry lacks.
    pub new_members: Vec<String>,
}

/// Where a function is now.
struct Current {
    at: String,
    shape: Option<String>,
}

fn current(facts: &[FnFacts]) -> BTreeMap<String, Current> {
    let names = locator::assign(facts);

    facts
        .iter()
        .map(|f| {
            let shape = f
                .pdg
                .as_ref()
                .map(|p| format!("{:016x}", wl::shape_hash(p)));
            let at = format!("{}:{}", f.path, f.line);

            (names[&f.id].clone(), Current { at, shape })
        })
        .collect()
}

/// `(locator, recorded shape)` of everything the entry remembers.
fn recorded(e: &Entry) -> Vec<(String, Option<String>)> {
    let sites = e.sites.iter().map(|s| match s.rsplit_once('@') {
        Some((locator, shape)) if shape != "-" => (locator.to_string(), Some(shape.to_string())),
        _ => (s.clone(), None),
    });
    let plain = e
        .fns
        .iter()
        .chain(&e.definitions)
        .map(|l| (l.clone(), None));

    sites.chain(plain).collect()
}

fn member(now: &BTreeMap<String, Current>, locator: String, shape: Option<String>) -> Member {
    let found = now.get(&locator);
    let state = match (found, shape) {
        (None, _) => MemberState::Gone,
        (Some(c), Some(old)) if c.shape.as_ref().is_some_and(|new| *new != old) => {
            MemberState::ShapeChanged
        }
        (Some(_), Some(_)) => MemberState::Unchanged,
        (Some(_), None) => MemberState::Found,
    };

    Member {
        at: found.map(|c| c.at.clone()),
        locator,
        state,
    }
}

fn locators_of(c: &Candidate) -> BTreeSet<String> {
    c.sites
        .iter()
        .chain(&c.definitions)
        .map(|s| s.locator.clone())
        .collect()
}

/// Members of the candidate that shares the most functions with the entry, minus the entry's.
fn new_members(known: &BTreeSet<String>, candidates: &[Candidate]) -> Vec<String> {
    let overlap = |c: &&Candidate| locators_of(c).intersection(known).count();
    let nearest = candidates
        .iter()
        .filter(|c| overlap(c) > 0)
        .max_by_key(overlap);

    nearest.map_or(vec![], |c| {
        locators_of(c).difference(known).cloned().collect()
    })
}

fn entry_status(
    (id, e): (&String, &Entry),
    candidates: &[Candidate],
    now: &BTreeMap<String, Current>,
) -> EntryStatus {
    let members: Vec<Member> = recorded(e)
        .into_iter()
        .map(|(locator, shape)| member(now, locator, shape))
        .collect();
    let active = candidates.iter().any(|c| matches(id, e, c).is_some());
    let known: BTreeSet<String> = members.iter().map(|m| m.locator.clone()).collect();

    EntryStatus {
        id: id.clone(),
        kind: e.kind,
        reason: e.reason.clone(),
        active,
        members,
        new_members: if active {
            vec![]
        } else {
            new_members(&known, candidates)
        },
    }
}

/// One status per entry, in id order. `candidates` must be the unfiltered current set.
pub fn status(ledger: &Ledger, candidates: &[Candidate], facts: &[FnFacts]) -> Vec<EntryStatus> {
    let now = current(facts);

    ledger
        .entries
        .iter()
        .map(|entry| entry_status(entry, candidates, &now))
        .collect()
}

fn member_text(m: &Member) -> String {
    let state = match m.state {
        MemberState::Found => "found",
        MemberState::Unchanged => "unchanged",
        MemberState::ShapeChanged => "shape changed",
        MemberState::Gone => "gone (renamed or deleted)",
    };

    format!(
        "  {}  {}  {state}",
        m.locator,
        m.at.as_deref().unwrap_or("-")
    )
}

fn stale_hint(s: &EntryStatus) -> Option<String> {
    let moved = s.members.iter().any(|m| m.state == MemberState::Gone);

    match (s.active, moved, s.new_members.is_empty()) {
        (true, _, _) | (_, true, _) | (_, _, false) => None,
        _ => Some("  no longer matches: a member's body changed".to_string()),
    }
}

fn entry_text(s: &EntryStatus) -> String {
    let state = if s.active { "active" } else { "stale" };
    let reason = s.reason.as_ref().map(|r| format!("  \"{r}\""));
    let head = format!(
        "{}  {}  {state}{}",
        s.id,
        s.kind.name(),
        reason.unwrap_or_default()
    );
    let added = s.new_members.iter().map(|l| format!("  + new member {l}"));

    std::iter::once(head)
        .chain(s.members.iter().map(member_text))
        .chain(added)
        .chain(stale_hint(s))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Human report for `cargo rstyle ignore --status`.
pub fn text(statuses: &[EntryStatus]) -> String {
    match statuses {
        [] => "no ignore entries".to_string(),
        _ => statuses
            .iter()
            .map(entry_text)
            .collect::<Vec<_>>()
            .join("\n\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidates::{Breakdown, Site};
    use crate::facts::{Pdg, sample};
    use crate::ledger::{Opts, add, candidate_id};
    use crate::pdgtest::{emit_loop, getter_call};

    fn fact(name: &str, line: usize, pdg: Pdg) -> FnFacts {
        FnFacts {
            path: "src/a.rs".into(),
            line,
            pdg: Some(pdg),
            ..sample(name)
        }
    }

    fn site_of(f: &FnFacts) -> Site {
        let shape = f
            .pdg
            .as_ref()
            .map(|p| format!("{:016x}", wl::shape_hash(p)));

        Site {
            path: f.path.clone(),
            line: f.line,
            id: f.id.clone(),
            name: f.name.clone(),
            locator: f.id.clone(),
            shape,
        }
    }

    fn candidate(fns: &[&FnFacts]) -> Candidate {
        let sites: Vec<Site> = fns.iter().map(|f| site_of(f)).collect();

        Candidate {
            id: candidate_id(CandidateKind::GenericFn, &sites, &[]),
            kind: CandidateKind::GenericFn,
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
            definitions: vec![],
            counter_evidence: vec![],
        }
    }

    fn two() -> (FnFacts, FnFacts) {
        (
            fact("a", 10, emit_loop("Dog", "bark")),
            fact("b", 20, emit_loop("Cat", "meow")),
        )
    }

    #[test]
    fn active_entry_resolves_live_locations() {
        let (a, b) = two();
        let dismissed = candidate(&[&a, &b]);
        let ledger = add(Ledger::default(), &dismissed, Opts::default());
        let moved = FnFacts {
            line: 55,
            ..a.clone()
        };
        let st = status(&ledger, &[dismissed], &[moved, b]);

        assert!(st[0].active);
        assert_eq!(st[0].members[0].at.as_deref(), Some("src/a.rs:55"));
        assert!(st[0].members.iter().all(|m| m.state == MemberState::Found));
    }

    #[test]
    fn growth_entry_reports_shape_changes_and_gone_members() {
        let (a, b) = two();
        let opts = Opts {
            allow_growth: true,
            reason: None,
        };
        let ledger = add(Ledger::default(), &candidate(&[&a, &b]), opts);
        let mut extra = emit_loop("Dog", "bark");
        extra.nodes.push(getter_call("Dog", "bark"));
        extra.edges.push(crate::pdgtest::data(2, 6, 0));
        let edited = fact("a", 10, extra);
        let st = status(&ledger, &[], &[edited]);
        let state_of = |locator: &str| {
            st[0]
                .members
                .iter()
                .find(|m| m.locator == locator)
                .unwrap()
                .state
        };

        assert!(!st[0].active);
        assert_eq!(state_of("krate::a"), MemberState::ShapeChanged);
        assert_eq!(state_of("krate::b"), MemberState::Gone);
    }

    #[test]
    fn stale_entry_names_the_members_that_joined() {
        let (a, b) = two();
        let c = fact("c", 30, emit_loop("Cow", "moo"));
        let ledger = add(Ledger::default(), &candidate(&[&a, &b]), Opts::default());
        let grown = candidate(&[&a, &b, &c]);
        let st = status(&ledger, &[grown], &[a, b, c]);

        assert!(!st[0].active);
        assert_eq!(st[0].new_members, ["krate::c"]);
    }

    #[test]
    fn text_marks_stale_entries_and_says_why() {
        let (a, b) = two();
        let ledger = add(Ledger::default(), &candidate(&[&a, &b]), Opts::default());
        let out = text(&status(&ledger, &[], &[a]));

        assert!(out.contains("stale"));
        assert!(out.contains("krate::b  -  gone (renamed or deleted)"));
        assert_eq!(text(&[]), "no ignore entries");
    }
}
