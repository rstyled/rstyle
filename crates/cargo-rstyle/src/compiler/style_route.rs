//! Default style routing through the existing authored compiler diagnostic workflow.
use super::{Args, CapturedRun, compile_capture, run_captured_with};
use crate::Cli;
use clap::{Args as _, FromArgMatches};

pub(crate) fn run(mut cli: Cli) -> Result<(String, u8), String> {
    let mut args = apply_inputs(&mut cli, compiler_args()?);
    let replay = args.replay_run.is_some();

    let Prepared { config, capture } = prepare(&mut cli, &mut args)?;
    let capture = if replay {
        capture
    } else {
        compile_capture(capture, &cli, &args)
    };
    run_captured_with(&cli, &args, CapturedRun { config, capture }, |report| {
        Ok((report.publish_style(cli.format)?, report.style_complete()?))
    })
    .map(summary_exit)
}

/// Findings decide 0 or 1 only over a complete summary. Without one, such as a
/// capture with no completed compiler evaluation, the style check could not
/// evaluate the code and exits 2 rather than passing.
fn summary_exit(((text, complete), code): ((String, bool), u8)) -> (String, u8) {
    (text, if complete { code } else { 2 })
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
        super::capture::prepare(cli, args)?
    };
    let config = configure(cli, args, &capture.plan.root)?;
    Ok(Prepared { config, capture })
}

/// Both root routes resolve core policy before consuming their root-only CLI inputs.
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
            "--compiler-advisory",
            "--compiler-diagnostic-scope",
            "authored",
            "--compiler-plans",
            "--fail-on-findings",
        ])
        .map_err(|error| error.to_string())?;
    Args::from_arg_matches(&matches).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::summary_exit;
    use crate::compiler::advisory::style::Projection;

    #[test]
    fn findings_decide_the_exit_only_over_a_complete_summary() {
        for code in [0, 1, 2] {
            assert_eq!(summary_exit((("out".into(), true), code)).1, code);
            assert_eq!(
                summary_exit((("out".into(), false), code)),
                ("out".into(), 2)
            );
        }
    }

    #[test]
    fn capture_without_units_is_incomplete() {
        assert!(!Projection::new(None).unwrap().complete());
    }
}
