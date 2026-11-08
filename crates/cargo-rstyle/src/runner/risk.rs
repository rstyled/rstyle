//! `rstyle risk`: what changed between two saved runs. A saved run holds its documents, so this
//! needs no frontend or compiler, and reads every language the same way. The runs' commits say
//! what changed that no document holds, and `--check` gates a merge on the rating.

use super::{RunRecord, read_any_record, revision, seen_saved_records};
use crate::cpg::load_config;
use crate::{Cli, Format};
use rstyle_core::config::Config;
use rstyle_core::cpg::records::Demand;
use rstyle_core::merge_risk::rating::Level;
use rstyle_core::merge_risk::{self, Commits, Snapshot, SourceFiles};
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub(crate) struct RiskArgs {
    /// The saved run to compare from, as `--out` writes one.
    #[arg(long)]
    base: PathBuf,
    /// The saved run to compare to.
    #[arg(long)]
    head: PathBuf,
    /// Exit 1 unless the change rates low, for CI. The report is still written. Run it from a
    /// clone holding both runs' commits.
    #[arg(long)]
    check: bool,
}

const DEMAND: Demand = Demand {
    graphs: true,
    bounds: false,
    flow: false,
    defects: false,
};

fn same_language(base: Option<&RunRecord>, head: Option<&RunRecord>) -> Result<(), String> {
    match (base, head) {
        (Some(base), Some(head)) if base.language != head.language => Err(format!(
            "--base is a {} run and --head is a {} run, so nothing in them corresponds",
            base.language, head.language
        )),
        _ => Ok(()),
    }
}

/// `--config`, or the policy the base run kept. Its checkout can't say: by now it may well hold the
/// head, and a change can't relax the policy it's judged by.
// rstyle-allow(side_effect_density): IO boundary: reads the base's policy
fn base_policy(cli: &Cli, base: &Path) -> Result<Config, String> {
    load_config(crate::config_path(cli.config.clone(), base).as_deref())
}

// rstyle-allow(side_effect_density): IO boundary: reads a saved run
fn snapshot(run: &Path, record: Option<&RunRecord>, config: &Config) -> Result<Snapshot, String> {
    let files = SourceFiles::default();
    let records = seen_saved_records((run, record), DEMAND, &|document| files.see(document))?;
    Snapshot::of(records, files, config)
}

/// What git, from the current directory, says changed between the runs' commits.
// rstyle-allow(side_effect_density): process boundary: asks git
fn commits(
    runs: (Option<&RunRecord>, Option<&RunRecord>),
    read: (&Snapshot, &Snapshot),
) -> Commits {
    let revisions = (
        runs.0.and_then(|run| run.revision.as_ref()),
        runs.1.and_then(|run| run.revision.as_ref()),
    );
    match std::env::current_dir() {
        Ok(directory) => revision::commits(&directory, revisions, read),
        Err(error) => Commits::Unavailable(format!("the current directory: {error}")),
    }
}

// rstyle-allow(side_effect_density): IO boundary: reads both saved runs
pub(crate) fn risk(cli: &Cli, args: &RiskArgs) -> Result<(String, u8), String> {
    let (base_run, head_run) = (read_any_record(&args.base)?, read_any_record(&args.head)?);
    same_language(base_run.as_ref(), head_run.as_ref())?;
    let config = base_policy(cli, &args.base)?;

    let base = snapshot(&args.base, base_run.as_ref(), &config)?;
    let head = snapshot(&args.head, head_run.as_ref(), &config)?;
    let commits = commits((base_run.as_ref(), head_run.as_ref()), (&base, &head));
    let report = merge_risk::compare(&base, &head, &config, &commits)?;
    let output = match cli.format {
        Format::Text => report.text(),
        Format::Json => serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?,
    };
    // Only `low` passes: what can't be seen, or what nobody rated, needs a person.
    let gated = args.check && report.rating.risk != Level::Low;
    Ok((output, u8::from(gated)))
}
