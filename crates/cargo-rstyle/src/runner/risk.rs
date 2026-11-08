//! `rstyle risk`: what changed between two saved runs. A saved run holds its documents, so this
//! needs no frontend or compiler, and reads every language the same way.

use super::{RunRecord, read_any_record, seen_saved_records};
use crate::cpg::load_config;
use crate::{Cli, Format};
use rstyle_core::config::Config;
use rstyle_core::cpg::records::Demand;
use rstyle_core::merge_risk::{self, Snapshot, SourceFiles};
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub(crate) struct RiskArgs {
    /// The saved run to compare from, as `--out` writes one.
    #[arg(long)]
    base: PathBuf,
    /// The saved run to compare to.
    #[arg(long)]
    head: PathBuf,
    /// Write the report as Markdown, for a pull request's description.
    #[arg(long)]
    markdown: bool,
    /// What the Markdown joins each file's path to for a link, such as
    /// `https://github.com/OWNER/REPO/blob/HEAD_SHA`.
    #[arg(long, requires = "markdown")]
    source_url: Option<String>,
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

// rstyle-allow(side_effect_density): IO boundary: reads both saved runs
pub(crate) fn risk(cli: &Cli, args: &RiskArgs) -> Result<(String, u8), String> {
    if args.markdown && matches!(cli.format, Format::Json) {
        return Err("--markdown and --format json each choose the output; pass one".to_owned());
    }
    let (base_run, head_run) = (read_any_record(&args.base)?, read_any_record(&args.head)?);
    same_language(base_run.as_ref(), head_run.as_ref())?;
    let config = base_policy(cli, &args.base)?;

    let base = snapshot(&args.base, base_run.as_ref(), &config)?;
    let head = snapshot(&args.head, head_run.as_ref(), &config)?;
    let report = merge_risk::compare(&base, &head, &config);
    let output = match cli.format {
        Format::Text if args.markdown => report.markdown(args.source_url.as_deref()),
        Format::Text => report.text(),
        Format::Json => serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?,
    };
    Ok((output, 0))
}
