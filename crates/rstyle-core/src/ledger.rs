//! The ignore ledger: a committed list of merge candidates a human already reviewed.
//!
//! It is a final filter on reported candidates. Analysis never sees it.

use crate::candidates::{CandidateKind, Site};
use crate::wl::fnv1a;

/// Hex digits of a candidate id.
const ID_LEN: usize = 12;

/// `locator@shape`: what identifies a site. Line numbers are deliberately absent, so a
/// function that moves within its file keeps its identity.
pub fn site_key(site: &Site) -> String {
    format!("{}@{}", site.locator, site.shape.as_deref().unwrap_or("-"))
}

fn joined(mut items: Vec<String>) -> String {
    items.sort();

    items.join("\n")
}

/// Fingerprint of a candidate: its kind, its sites (locator and shape) and its definitions
/// (locator only: editing a definition's body does not change what the candidate proposes).
pub fn candidate_id(kind: CandidateKind, sites: &[Site], definitions: &[Site]) -> String {
    let site_keys = joined(sites.iter().map(site_key).collect());
    let definition_keys = joined(definitions.iter().map(|d| d.locator.clone()).collect());
    let text = format!("{}|{site_keys}|{definition_keys}", kind.name());

    format!("{:016x}", fnv1a(text.as_bytes()))[..ID_LEN].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(locator: &str, shape: &str, line: usize) -> Site {
        Site {
            path: "src/a.rs".into(),
            line,
            id: locator.into(),
            name: locator.into(),
            locator: locator.into(),
            shape: Some(shape.into()),
        }
    }

    fn id_of(sites: &[Site], definitions: &[Site]) -> String {
        candidate_id(CandidateKind::TraitMethod, sites, definitions)
    }

    #[test]
    fn id_ignores_line_numbers_and_member_order() {
        let a = [site("k::a", "1", 45), site("k::b", "2", 90)];
        let moved = [site("k::b", "2", 120), site("k::a", "1", 50)];

        assert_eq!(id_of(&a, &[]), id_of(&moved, &[]));
    }

    #[test]
    fn id_changes_with_site_shape_membership_and_kind() {
        let base = [site("k::a", "1", 1), site("k::b", "2", 2)];
        let reshaped = [site("k::a", "9", 1), site("k::b", "2", 2)];
        let grown = [base[0].clone(), base[1].clone(), site("k::c", "3", 3)];

        assert_ne!(id_of(&base, &[]), id_of(&reshaped, &[]));
        assert_ne!(id_of(&base, &[]), id_of(&grown, &[]));
        assert_ne!(
            id_of(&base, &[]),
            candidate_id(CandidateKind::GenericFn, &base, &[])
        );
    }

    #[test]
    fn definitions_count_by_locator_not_shape() {
        let sites = [site("k::a", "1", 1)];
        let edited = [site("k::Dog::name", "7", 5)];
        let rewritten = [site("k::Dog::name", "8", 5)];
        let added = [edited[0].clone(), site("k::Cat::name", "7", 9)];

        assert_eq!(id_of(&sites, &edited), id_of(&sites, &rewritten));
        assert_ne!(id_of(&sites, &edited), id_of(&sites, &added));
        assert_ne!(id_of(&sites, &edited), id_of(&sites, &[]));
    }

    #[test]
    fn id_is_pinned() {
        // Golden: a change here invalidates every committed rstyle-ignore.toml.
        let sites = [site("k::a", "00000000000000a1", 1)];

        assert_eq!(id_of(&sites, &[]), "0dafcd1ad065");
    }
}
