//! Compatibility matches suggest a migration; current scoped snapshots define imported reviews.
use super::super::{ReviewStore, exact_rules};
use super::observed;
use crate::clones::{identity::UnitId, report};
use rstyle_core::ledger::{self, Matched};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    ledger_path: PathBuf,
    scope_complete: bool,
    pub complete: bool,
    entries: Vec<Entry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    imported: Option<Imported>,
}

#[derive(Serialize)]
struct Entry {
    legacy_id: String,
    allow_growth: bool,
    reason: Option<String>,
    state: &'static str,
    matches: Vec<Match>,
}

#[derive(Serialize)]
struct Match {
    unit: UnitId,
    candidate_index: usize,
    candidate_id: String,
    matched: Matched,
    snapshot_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    unavailable_reason: Option<String>,
}

#[derive(Serialize)]
struct Imported {
    legacy_id: String,
    unit: UnitId,
    candidate_id: String,
    current_members_only: bool,
}

pub(super) fn apply(
    rows: &[observed::Row<'_>],
    ctx: &report::Context<'_>,
    store: Option<&mut ReviewStore>,
    scope_complete: bool,
) -> Result<Option<Report>, String> {
    let Some(path) = &ctx.args.legacy_pattern_ledger else {
        return Ok(None);
    };
    let ledger = crate::ignore::load_required(path)?;
    let entries = ledger
        .entries
        .iter()
        .map(|(id, entry)| preview_entry(id, entry, rows, scope_complete))
        .collect();
    let complete = scope_complete && rows.iter().all(|row| row.snapshot.is_ok());
    let imported = if ctx.args.import_legacy_pattern.is_empty() {
        None
    } else {
        Some(import(
            &ledger,
            rows,
            ctx,
            store.ok_or("Legacy review import requires a review file")?,
        )?)
    };

    Ok(Some(Report {
        schema: "rstyle.compiler-pattern-legacy-migration/1",
        ledger_path: path.clone(),
        scope_complete,
        complete,
        entries,
        imported,
    }))
}

fn preview_entry(
    id: &str,
    entry: &ledger::Entry,
    rows: &[observed::Row<'_>],
    scope_complete: bool,
) -> Entry {
    let matches: Vec<_> = rows
        .iter()
        .filter_map(|row| {
            ledger::matches(id, entry, row.candidate).map(|matched| preview_match(row, matched))
        })
        .collect();
    let state = match matches.as_slice() {
        [] if !scope_complete => "unavailable",
        [] => "no_current_match",
        _ if entry.allow_growth => "growth_review_required",
        [_] if !matches[0].snapshot_available => "unavailable",
        [_] => "matched",
        _ => "multiple_contexts",
    };
    Entry {
        legacy_id: id.into(),
        allow_growth: entry.allow_growth,
        reason: entry.reason.clone(),
        state,
        matches,
    }
}

fn preview_match(row: &observed::Row<'_>, matched: Matched) -> Match {
    let unavailable_reason = row.snapshot.as_ref().err().cloned();
    Match {
        unit: row.unit,
        candidate_index: row.index,
        candidate_id: row.id.into(),
        matched,
        snapshot_available: unavailable_reason.is_none(),
        unavailable_reason,
    }
}

fn import(
    ledger: &ledger::Ledger,
    rows: &[observed::Row<'_>],
    ctx: &report::Context<'_>,
    store: &mut ReviewStore,
) -> Result<Imported, String> {
    let [legacy_id, unit, candidate] = ctx.args.import_legacy_pattern.as_slice() else {
        return Err("Legacy import requires LEGACY_ID UNIT CANDIDATE_ID".into());
    };
    let entry = ledger
        .entries
        .get(legacy_id)
        .ok_or("Selected legacy dismissal is not in the ledger")?;
    let row = super::selected_candidate(rows, unit, candidate)?;
    if ledger::matches(legacy_id, entry, row.candidate).is_none() {
        return Err("Selected compiler proposal does not match the legacy dismissal".into());
    }
    if entry.allow_growth && !ctx.args.legacy_current_members_only {
        return Err("Legacy growth review requires --legacy-current-members-only to import the current exact membership; future growth is not migrated".into());
    }

    exact_rules::record_with_reason(
        &mut store.compiler_pattern_entries,
        &row.checked()?,
        ctx.args
            .review_reason
            .as_deref()
            .or(entry.reason.as_deref()),
    )?;
    store.schema_version = store.schema_version.max(8);
    Ok(Imported {
        legacy_id: legacy_id.clone(),
        unit: row.unit,
        candidate_id: row.id.into(),
        current_members_only: entry.allow_growth,
    })
}

impl Report {
    pub(super) fn render(&self) -> String {
        let mut lines = vec![format!(
            "Legacy proposal migration: {} entries, scope_complete={}, complete={}; original ledger unchanged",
            self.entries.len(),
            self.scope_complete,
            self.complete
        )];
        for entry in &self.entries {
            lines.push(format!(
                "Legacy {}: {} ({} matches)",
                entry.legacy_id,
                entry.state,
                entry.matches.len()
            ));
            for matched in &entry.matches {
                lines.push(format!(
                    "  unit {} candidate {}: {:?}, snapshot_available={}",
                    matched.unit.ordinal(),
                    matched.candidate_id,
                    matched.matched,
                    matched.snapshot_available
                ));
            }
        }
        if let Some(imported) = &self.imported {
            lines.push(format!(
                "Imported legacy {} as exact unit {} candidate {} (current_members_only={})",
                imported.legacy_id,
                imported.unit.ordinal(),
                imported.candidate_id,
                imported.current_members_only
            ));
        }
        lines.join("\n")
    }
}
