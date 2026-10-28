//! Ranked inspection of all measured functions, independent of style thresholds.

use crate::{Cli, Format};
use clap::{Args as ClapArgs, ValueEnum};
use rstyle_core::complexity_report::{Order, Ranking};

#[derive(ClapArgs)]
pub struct Args {
    /// Primary score for descending ranking; ties use the other score, then source location.
    #[arg(long, value_enum, default_value_t = Sort::Cyclomatic)]
    sort: Sort,
    /// Maximum number of functions to display. Availability counts cover the full selection.
    #[arg(long, default_value_t = 20)]
    top: usize,
}

#[derive(Clone, Copy, ValueEnum)]
enum Sort {
    Cyclomatic,
    Cognitive,
}

pub fn run(cli: &Cli, args: &Args) -> Result<(String, u8), String> {
    let report = crate::analyze(cli)?;
    let order = match args.sort {
        Sort::Cyclomatic => Order::Cyclomatic,
        Sort::Cognitive => Order::Cognitive,
    };
    let ranking = Ranking::new(report.functions, order, args.top);
    let status = if ranking.unavailable > 0 { 2 } else { 0 };

    let output = match cli.format {
        Format::Text => ranking.text(),
        Format::Json => {
            serde_json::to_string_pretty(&ranking).map_err(|error| error.to_string())?
        }
    };
    Ok((output, status))
}
