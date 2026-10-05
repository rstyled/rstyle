//! The ignore ledger: a committed list of merge candidates a human already reviewed.
//!
//! It is a final filter on reported candidates. Analysis never sees it.

mod matching;

pub use matching::{Applied, Hidden, Matched, Opts, add, apply, matches, prune};

use crate::candidates::{CandidateKind, Site};
use crate::wl::fnv1a;
use serde::Deserialize;
use std::collections::BTreeMap;

/// File format version; bump when the id or site-key scheme changes.
pub const VERSION: u32 = 1;

const HEADER: &str = "# rstyle ignore ledger: merge candidates a human reviewed and dismissed.\n\
# Commit it. Delete it (or an entry) to re-review. Rewritten by `cargo rstyle ignore`,\n\
# which does not keep comments; use `reason` for notes.";

/// One dismissed candidate. Strict entries (the default) match by id alone and keep `fns` as a
/// human hint; `allow_growth` entries keep the members the subset check needs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub kind: CandidateKind,
    #[serde(default)]
    pub allow_growth: bool,
    #[serde(default)]
    pub reason: Option<String>,
    /// Locators of the dismissed functions. A hint, never matched on.
    #[serde(default)]
    pub fns: Vec<String>,
    /// `locator@shape` of each site (`allow_growth` entries).
    #[serde(default)]
    pub sites: Vec<String>,
    /// Locator of each definition (`allow_growth` entries).
    #[serde(default)]
    pub definitions: Vec<String>,
}

/// Entries keyed by candidate id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger {
    pub entries: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: u32,
    #[serde(default)]
    ignore: BTreeMap<String, Entry>,
}

fn valid_id(id: &str) -> bool {
    id.len() == ID_LEN && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn check(id: &str, entry: &Entry) -> Result<(), String> {
    match () {
        () if !valid_id(id) => Err(format!("`{id}` is not a candidate id (12 hex digits)")),
        () if entry.allow_growth && entry.sites.is_empty() => Err(format!(
            "[ignore.{id}] sets allow_growth but lists no sites"
        )),
        () => Ok(()),
    }
}

/// Parses `rstyle-ignore.toml`. Unknown keys, a duplicate id, a malformed id and an
/// `allow_growth` entry without `sites` are errors.
pub fn parse(text: &str) -> Result<Ledger, String> {
    let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
    if file.version != VERSION {
        return Err(format!(
            "unsupported ledger version {} (this rstyle reads {VERSION})",
            file.version
        ));
    }
    file.ignore.iter().try_for_each(|(id, e)| check(id, e))?;

    Ok(Ledger {
        entries: file.ignore,
    })
}

fn quoted(text: &str) -> String {
    // A JSON string is a valid TOML basic string.
    serde_json::to_string(text).unwrap_or_default()
}

fn list(key: &str, items: &[String]) -> Option<String> {
    let mut sorted = items.to_vec();
    sorted.sort();
    sorted.dedup();
    let lines: Vec<String> = sorted.iter().map(|i| format!("  {},", quoted(i))).collect();

    (!lines.is_empty()).then(|| format!("{key} = [\n{}\n]", lines.join("\n")))
}

fn entry_text(id: &str, e: &Entry) -> String {
    let fields = [
        Some(format!("kind = {}", quoted(e.kind.name()))),
        e.allow_growth.then(|| "allow_growth = true".to_string()),
        e.reason.as_ref().map(|r| format!("reason = {}", quoted(r))),
        list("fns", &e.fns),
        list("sites", &e.sites),
        list("definitions", &e.definitions),
    ];
    let body: Vec<String> = fields.into_iter().flatten().collect();

    format!("[ignore.{id}]\n{}", body.join("\n"))
}

/// Deterministic text: entries by kind then id, members sorted, defaults omitted.
pub fn render(ledger: &Ledger) -> String {
    let mut ordered: Vec<(&String, &Entry)> = ledger.entries.iter().collect();
    ordered.sort_by_key(|(id, e)| (e.kind.name(), id.as_str()));
    let entries = ordered.into_iter().map(|(id, e)| entry_text(id, e));
    let head = format!("{HEADER}\n\nversion = {VERSION}");

    std::iter::once(head)
        .chain(entries)
        .collect::<Vec<_>>()
        .join("\n\n")
        + "\n"
}

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

    const SAMPLE: &str = r#"
version = 1

[ignore.7e90bb214c8d]
kind = "trait_method"
allow_growth = true
reason = "emitters stay separate"
sites = [
  "k::emit_json@d83c41a9f0e26b75",
  "k::emit_text@2a6f9d03e5c81b47",
]
definitions = ["k::Json::name", "k::Text::name"]

[ignore.a1b2c3d4e5f6]
kind = "generic_fn"
fns = ["k::a", "k::b"]
"#;

    fn err(text: &str) -> String {
        parse(text).unwrap_err()
    }

    #[test]
    fn parses_strict_and_growth_entries() {
        let ledger = parse(SAMPLE).unwrap();
        let strict = &ledger.entries["a1b2c3d4e5f6"];
        let growth = &ledger.entries["7e90bb214c8d"];

        assert!(!strict.allow_growth && strict.sites.is_empty());
        assert_eq!(strict.fns, ["k::a", "k::b"]);
        assert!(growth.allow_growth && growth.sites.len() == 2);
        assert_eq!(growth.reason.as_deref(), Some("emitters stay separate"));
    }

    #[test]
    fn render_round_trips_and_is_byte_stable() {
        let ledger = parse(SAMPLE).unwrap();
        let text = render(&ledger);

        assert_eq!(parse(&text).unwrap(), ledger);
        assert_eq!(render(&parse(&text).unwrap()), text);
    }

    #[test]
    fn render_omits_defaults_and_orders_by_kind_then_id() {
        let text = render(&parse(SAMPLE).unwrap());
        let after = text.split("[ignore.a1b2c3d4e5f6]").nth(1).unwrap();
        let strict = after.split("[ignore.7e90bb214c8d]").next().unwrap();

        assert!(!strict.contains("allow_growth") && !strict.contains("sites"));
        assert!(text.find("generic_fn").unwrap() < text.find("trait_method").unwrap());
    }

    #[test]
    fn an_empty_ledger_is_just_a_version() {
        let text = render(&Ledger::default());

        assert!(text.contains("version = 1") && !text.contains("[ignore"));
        assert_eq!(parse(&text).unwrap(), Ledger::default());
    }

    #[test]
    fn rejects_bad_files() {
        let entry = "[ignore.a1b2c3d4e5f6]\nkind = \"generic_fn\"\n";

        assert!(err("version = 2").contains("unsupported ledger version 2"));
        assert!(err(&format!("version = 1\n{entry}{entry}")).contains("duplicate"));
        assert!(
            err("version = 1\n[ignore.zz]\nkind = \"generic_fn\"").contains("not a candidate id")
        );
        assert!(err(&format!("version = 1\n{entry}typo = 1")).contains("typo"));
        assert!(
            err(&format!("version = 1\n{entry}allow_growth = true")).contains("lists no sites")
        );
    }

    #[test]
    fn strings_with_quotes_survive() {
        let mut ledger = parse(SAMPLE).unwrap();
        ledger.entries.get_mut("a1b2c3d4e5f6").unwrap().reason = Some("say \"no\" \\ ok".into());

        assert_eq!(parse(&render(&ledger)).unwrap(), ledger);
    }

    #[test]
    fn id_is_pinned() {
        // Golden: a change here invalidates every committed rstyle-ignore.toml.
        let sites = [site("k::a", "00000000000000a1", 1)];

        assert_eq!(id_of(&sites, &[]), "0dafcd1ad065");
    }
}
