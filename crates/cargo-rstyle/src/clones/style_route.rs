//! Default style routing through the existing authored compiler diagnostic workflow.
use super::{Args, CapturedRun, compile_capture, prepare_capture, run_captured_with};
use crate::Cli;
use clap::{Args as _, FromArgMatches};

pub(crate) fn run(mut cli: Cli) -> Result<(String, u8), String> {
    let mut args = apply_inputs(&mut cli, compiler_args()?);
    let replay = args.replay_run.is_some();

    super::with_capture_verification(replay, || {
        let Prepared { config, capture } = prepare(&mut cli, &mut args)?;
        let capture = if replay {
            capture
        } else {
            compile_capture(capture, &cli, &args)
        };
        run_captured_with(&cli, &args, CapturedRun { config, capture }, |report, _| {
            report.publish_style(cli.format)
        })
    })
}

fn apply_inputs(cli: &mut Cli, args: Args) -> Args {
    let inputs = std::mem::take(&mut cli.style_inputs);
    let baseline = inputs.baseline;
    let review = inputs.review;
    Args {
        replay_run: inputs.replay_run,
        changed_files: inputs.changed_files,
        compiler_baseline: baseline.baseline,
        write_compiler_baseline: baseline.write_baseline,
        fail_on_new_compiler: baseline.fail_on_new,
        fail_on_findings: !baseline.fail_on_new,
        review_file: review.review_file,
        review_status: review.review_status,
        dismiss_compiler_diagnostic: review.dismiss_compiler_diagnostic,
        review_reason: review.review_reason,
        ..args
    }
}

struct Prepared {
    config: super::Configuration,
    capture: super::capture::Capture,
}

fn prepare(cli: &mut Cli, args: &mut Args) -> Result<Prepared, String> {
    let capture = if args.replay_run.is_some() {
        super::capture_input(cli, args)?
    } else {
        prepare_capture(cli, args)?
    };
    let config = configure(cli, args, &capture.plan.root)?;
    Ok(Prepared { config, capture })
}

/// Both root routes resolve core policy before clearing their legacy-only CLI inputs.
pub(super) fn configure(
    cli: &mut Cli,
    args: &mut Args,
    root: &std::path::Path,
) -> Result<super::Configuration, String> {
    args.compiler_advisory_config = crate::config_path(cli.config.take(), root);
    cli.ignore_file = None;
    super::configuration(cli, args)
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
    let mut args = Args::from_arg_matches(&matches).map_err(|error| error.to_string())?;
    args.style_inputs_only = true;
    Ok(args)
}
