//! `cargo rstyle ignore`: dismiss, list and prune reviewed merge candidates.
//!
//! Everything here is pure except `load` and `save`, which are the only file access.

use rstyle_core::candidates::Candidate;
use rstyle_core::ledger::{self, Ledger, Opts};
use std::path::{Path, PathBuf};

const DEFAULT_FILE: &str = "rstyle-ignore.toml";
const MIN_PREFIX: usize = 4;

/// `--ignore-file`, or `rstyle-ignore.toml` in the workspace root.
pub fn ledger_path(explicit: Option<&Path>, root: &Path) -> PathBuf {
    explicit.map_or_else(|| root.join(DEFAULT_FILE), Path::to_path_buf)
}

/// The ledger at `path`; a missing file is an empty ledger.
// rstyle-allow(side_effect_density): IO boundary: reads the ledger file
pub fn load(path: &Path) -> Result<Ledger, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => ledger::parse(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Ledger::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

// rstyle-allow(side_effect_density): IO boundary: writes the ledger file
pub fn save(path: &Path, ledger: &Ledger) -> Result<(), String> {
    std::fs::write(path, ledger::render(ledger)).map_err(|e| format!("{}: {e}", path.display()))
}

/// The one candidate whose id starts with `prefix` (git style).
pub fn resolve<'a>(prefix: &str, candidates: &'a [Candidate]) -> Result<&'a Candidate, String> {
    if prefix.len() < MIN_PREFIX {
        return Err(format!(
            "candidate id `{prefix}` is too short (min {MIN_PREFIX})"
        ));
    }
    let found: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| c.id.starts_with(prefix))
        .collect();

    match found.as_slice() {
        [one] => Ok(one),
        [] => Err(format!(
            "no candidate with id `{prefix}`; run `cargo rstyle patterns` to list ids \
             (use the same clustering flags)"
        )),
        many => {
            let ids: Vec<&str> = many.iter().map(|c| c.id.as_str()).collect();
            Err(format!("`{prefix}` is ambiguous: {}", ids.join(", ")))
        }
    }
}

/// `ledger` with the candidate `prefix` dismissed, and a one-line confirmation.
pub fn dismiss(
    ledger: Ledger,
    candidates: &[Candidate],
    prefix: &str,
    opts: Opts,
) -> Result<(Ledger, String), String> {
    let c = resolve(prefix, candidates)?;
    let grows = if opts.allow_growth {
        ", allow_growth"
    } else {
        ""
    };
    let message = format!(
        "ignored {} {} ({} sites{grows})",
        c.kind.name(),
        c.id,
        c.sites.len()
    );

    Ok((ledger::add(ledger, c, opts), message))
}

/// Entries that match no current candidate: the new ledger and a line per removed entry.
pub fn prune(ledger: Ledger, candidates: &[Candidate], dry_run: bool) -> (Ledger, String) {
    let (kept, removed) = ledger::prune(ledger.clone(), candidates);
    let verb = if dry_run { "would remove" } else { "removed" };
    let mut lines: Vec<String> = removed
        .iter()
        .map(|(id, e)| {
            let hint = e
                .fns
                .iter()
                .chain(&e.definitions)
                .cloned()
                .collect::<Vec<_>>();
            let reason = e
                .reason
                .as_deref()
                .map(|r| format!("  \"{r}\""))
                .unwrap_or_default();

            format!("{verb} {id} {}  {}{reason}", e.kind.name(), hint.join(", "))
        })
        .collect();
    lines.push(format!("{} stale entries", removed.len()));

    (if dry_run { ledger } else { kept }, lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstyle_core::candidates::{Breakdown, CandidateKind, Site};

    fn candidate(id: &str, locator: &str) -> Candidate {
        let site = Site {
            path: "src/a.rs".into(),
            line: 1,
            id: locator.into(),
            name: locator.into(),
            locator: locator.into(),
            shape: Some("1".into()),
        };

        Candidate {
            id: id.into(),
            kind: CandidateKind::GenericFn,
            score_milli: 1000,
            breakdown: Breakdown {
                support: 1,
                lift_milli: 0,
                holes: 0,
                coverage_milli: 1000,
            },
            observation: String::new(),
            inference: String::new(),
            possible_refactor: String::new(),
            sites: vec![site],
            definitions: vec![],
            counter_evidence: vec![],
        }
    }

    fn pool() -> Vec<Candidate> {
        vec![
            candidate("a1b2c3d4e5f6", "k::a"),
            candidate("a1b2ffffffff", "k::b"),
            candidate("0123456789ab", "k::c"),
        ]
    }

    #[test]
    fn resolve_takes_a_unique_prefix() {
        assert_eq!(resolve("0123", &pool()).unwrap().id, "0123456789ab");
        assert_eq!(resolve("a1b2c", &pool()).unwrap().id, "a1b2c3d4e5f6");
    }

    #[test]
    fn resolve_rejects_unknown_ambiguous_and_short_prefixes() {
        assert!(
            resolve("dead", &pool())
                .unwrap_err()
                .contains("no candidate")
        );
        assert!(resolve("a1b2", &pool()).unwrap_err().contains("ambiguous"));
        assert!(resolve("a1", &pool()).unwrap_err().contains("too short"));
    }

    #[test]
    fn dismiss_adds_an_entry_for_the_resolved_candidate() {
        let (ledger, message) =
            dismiss(Ledger::default(), &pool(), "0123", Opts::default()).unwrap();

        assert!(ledger.entries.contains_key("0123456789ab"));
        assert!(message.contains("generic_fn 0123456789ab"));
        assert!(dismiss(ledger, &pool(), "zzzz", Opts::default()).is_err());
    }

    #[test]
    fn prune_dry_run_reports_without_changing_the_ledger() {
        let ledger = dismiss(Ledger::default(), &pool(), "0123", Opts::default())
            .unwrap()
            .0;
        let (same, report) = prune(ledger.clone(), &[], true);
        let (emptied, _) = prune(ledger.clone(), &[], false);

        assert_eq!(same, ledger);
        assert!(emptied.entries.is_empty());
        assert!(report.contains("would remove 0123456789ab generic_fn  k::c"));
        assert!(report.ends_with("1 stale entries"));
    }

    #[test]
    fn load_treats_a_missing_file_as_empty_and_save_round_trips() {
        let dir = std::env::temp_dir().join(format!("rstyle-ignore-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rstyle-ignore.toml");
        let ledger = dismiss(Ledger::default(), &pool(), "0123", Opts::default())
            .unwrap()
            .0;

        assert_eq!(load(&path).unwrap(), Ledger::default());
        save(&path, &ledger).unwrap();
        assert_eq!(load(&path).unwrap(), ledger);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
