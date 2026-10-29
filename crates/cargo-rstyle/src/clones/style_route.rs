//! Opt-in style routing through the existing authored compiler diagnostic workflow.
use super::{Args, compile_capture, configuration, prepare_capture, run_captured};
use crate::Cli;
use clap::{Args as _, FromArgMatches};

pub(crate) fn run(mut cli: Cli) -> Result<(String, u8), String> {
    let mut args = compiler_args()?;
    super::with_capture_verification(false, || {
        let Prepared { config, capture } = prepare(&mut cli, &mut args)?;
        let capture = compile_capture(capture, &cli, &args);
        run_captured(&cli, &args, config, capture)
    })
}

struct Prepared {
    config: super::Configuration,
    capture: super::capture::Capture,
}

fn prepare(cli: &mut Cli, args: &mut Args) -> Result<Prepared, String> {
    let capture = prepare_capture(cli, args)?;
    args.compiler_advisory_config = crate::config_path(cli.config.take(), &capture.plan.root);
    // The pattern ledger does not affect the legacy style command either.
    cli.ignore_file = None;

    let config = configuration(cli, args)?;
    Ok(Prepared { config, capture })
}

fn compiler_args() -> Result<Args, String> {
    let matches = Args::augment_args(clap::Command::new("style"))
        .try_get_matches_from([
            "style",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory-only",
            "--compiler-advisory",
            "--compiler-diagnostic-scope",
            "authored",
            "--compiler-plans",
            "--fail-on-findings",
            "--report-detail",
            "compact",
        ])
        .map_err(|error| error.to_string())?;
    Args::from_arg_matches(&matches).map_err(|error| error.to_string())
}
