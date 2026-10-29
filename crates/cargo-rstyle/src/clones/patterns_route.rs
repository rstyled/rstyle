//! Route the opt-in patterns command through the existing compiler workflow.
use super::{Args, capture_input, configuration, run_captured};
use crate::{Cli, PatternsArgs, ignore};
use clap::{Args as _, FromArgMatches};
use std::path::Path;

pub(crate) fn run(cli: Cli, patterns: PatternsArgs) -> Result<(String, u8), String> {
    if patterns.show_ignored {
        return Err("--show-ignored lists legacy dismissals; compiler reviews use --review-file and --review-status".into());
    }

    compiler_args(patterns)?.run(cli)
}

struct Captured {
    config: super::Configuration,
    capture: super::capture::Capture,
}

fn prepare(cli: &mut Cli, args: &mut Routed) -> Result<Captured, String> {
    let capture = capture_input(cli, &args.options)?;
    check_ledger(cli, &capture.plan.root, args.no_legacy_ignore)?;

    args.options.compiler_advisory_config = config_path(cli, &capture.plan.root);
    cli.ignore_file = None;
    let config = configuration(cli, &args.options)?;
    Ok(Captured { config, capture })
}

struct Routed {
    options: Args,
    no_legacy_ignore: bool,
}

impl Routed {
    fn run(mut self, mut cli: Cli) -> Result<(String, u8), String> {
        let replay = self.options.replay_run.is_some();
        super::with_capture_verification(replay, || {
            let Captured { config, capture } = prepare(&mut cli, &mut self)?;
            run_captured(&cli, &self.options, config, capture)
        })
    }
}

fn compiler_args(patterns: PatternsArgs) -> Result<Routed, String> {
    // Use the same clap defaults, including explicit nonzero budgets, as the clones entry.
    let matches = Args::augment_args(clap::Command::new("clones"))
        .try_get_matches_from([
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-patterns",
            "--compiler-advisory-only",
            "--report-detail",
            "compact",
        ])
        .map_err(|error| error.to_string())?;

    let options = Args {
        mine: patterns.mine,
        pattern_display: patterns.display,
        fail_on_findings: patterns.fail_on_candidates,
        replay_run: patterns.route.replay_run,
        review_file: patterns.route.review_file,
        review_status: patterns.route.review_status,
        locked: patterns.route.locked,
        ..Args::from_arg_matches(&matches).map_err(|error| error.to_string())?
    };

    Ok(Routed {
        options,
        no_legacy_ignore: patterns.no_ignore,
    })
}

fn check_ledger(cli: &Cli, root: &Path, no_ignore: bool) -> Result<(), String> {
    if !no_ignore
        && !ignore::load(&ignore::ledger_path(cli.ignore_file.as_deref(), root))?
            .entries
            .is_empty()
    {
        return Err("Compiler patterns do not automatically migrate legacy dismissals; use --no-ignore to explicitly bypass the legacy ledger, and --review-file for scoped compiler reviews. The legacy ledger is unchanged.".into());
    }
    Ok(())
}

fn config_path(cli: &mut Cli, root: &Path) -> Option<std::path::PathBuf> {
    cli.config.take().or_else(|| {
        let default = root.join("rstyle.toml");
        default.exists().then_some(default)
    })
}
