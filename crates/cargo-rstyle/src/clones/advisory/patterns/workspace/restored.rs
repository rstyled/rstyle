//! Restore original display and review identities after scoped discovery.
use super::identities::{restore_facts, restore_id};
use rstyle_core::{
    candidates::{Candidate, Site, Suppressed},
    facts::FnFacts,
    patterns_report::PatternsReport,
    sigmine::{Member, SigGroup},
};
use std::collections::BTreeMap;

pub(in crate::clones::advisory::patterns) fn restore_evaluation(
    evaluation: PatternsReport,
    mut prepared: Vec<FnFacts>,
) -> (PatternsReport, Vec<FnFacts>) {
    prepared.iter_mut().for_each(restore_facts);
    let restorer = Restorer {
        names: rstyle_core::locator::assign(&prepared),
        facts: prepared
            .iter()
            .map(|facts| (facts.id.as_str(), facts))
            .collect(),
    };

    let evaluation = PatternsReport {
        candidates: evaluation
            .candidates
            .into_iter()
            .map(|candidate| restorer.candidate(candidate))
            .collect(),
        suppressed: evaluation
            .suppressed
            .into_iter()
            .map(|suppressed| restorer.suppressed(suppressed))
            .collect(),
        signature_groups: evaluation
            .signature_groups
            .into_iter()
            .map(|group| restorer.group(group))
            .collect(),
        ..evaluation
    };
    (evaluation, prepared)
}

struct Restorer<'a> {
    names: BTreeMap<String, String>,
    facts: BTreeMap<&'a str, &'a FnFacts>,
}

impl Restorer<'_> {
    fn site(&self, site: Site) -> Site {
        let mut id = site.id;
        restore_id(&mut id);
        let locator = self.names[&id].clone();
        let shape = self.facts[id.as_str()]
            .pdg
            .as_ref()
            .map(|graph| format!("{:016x}", rstyle_core::wl::shape_hash(graph)));

        Site {
            id,
            locator,
            shape,
            ..site
        }
    }

    fn candidate(&self, candidate: Candidate) -> Candidate {
        let sites: Vec<_> = candidate
            .sites
            .into_iter()
            .map(|site| self.site(site))
            .collect();
        let definitions: Vec<_> = candidate
            .definitions
            .into_iter()
            .map(|site| self.site(site))
            .collect();
        let id = rstyle_core::ledger::candidate_id(candidate.kind, &sites, &definitions);

        Candidate {
            id,
            sites,
            definitions,
            ..candidate
        }
    }

    fn suppressed(&self, suppressed: Suppressed) -> Suppressed {
        Suppressed {
            reason: restore_reason(suppressed.reason),
            sites: suppressed
                .sites
                .into_iter()
                .map(|site| self.site(site))
                .collect(),
        }
    }

    fn member(&self, member: Member) -> Member {
        let mut id = member.id;
        restore_id(&mut id);
        let self_ty = self.facts[id.as_str()]
            .sig
            .as_ref()
            .and_then(|sig| sig.owner())
            .unwrap_or(member.self_ty);

        Member {
            id,
            self_ty,
            ..member
        }
    }

    fn group(&self, group: SigGroup) -> SigGroup {
        SigGroup {
            members: group
                .members
                .into_iter()
                .map(|member| self.member(member))
                .collect(),
            traited: group
                .traited
                .into_iter()
                .map(|member| self.member(member))
                .collect(),
            ..group
        }
    }
}

fn restore_reason(reason: String) -> String {
    let Some((prefix, method)) = reason.split_once(" impls are of trait method ") else {
        return reason;
    };
    let mut method = method.to_owned();
    restore_id(&mut method);

    format!("{prefix} impls are of trait method {method}")
}
