//! Route the opt-in patterns command through the existing compiler workflow.
use super::{Args, capture_input, run_captured};
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
    resolve_ledger(cli, args, &capture.plan.root)?;

    let config = super::style_route::configure(cli, &mut args.options, &capture.plan.root)?;
    Ok(Captured { config, capture })
}

struct Routed {
    options: Args,
    no_legacy_ignore: bool,
    preview_legacy_reviews: bool,
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

    let preview_legacy_reviews =
        patterns.route.preview_legacy_reviews || !patterns.route.import_legacy_pattern.is_empty();
    let options = Args {
        mine: patterns.mine,
        pattern_display: patterns.display,
        fail_on_findings: patterns.fail_on_candidates,
        compiler_pattern_scope: patterns.route.compiler_pattern_scope.unwrap_or_default(),
        replay_run: patterns.route.replay_run,
        review_file: patterns.route.review_file,
        review_status: patterns.route.review_status,
        locked: patterns.route.locked,
        import_legacy_pattern: patterns.route.import_legacy_pattern,
        dismiss_compiler_pattern: patterns.route.dismiss_compiler_pattern,
        legacy_current_members_only: patterns.route.legacy_current_members_only,
        review_reason: patterns.route.review_reason,
        ..Args::from_arg_matches(&matches).map_err(|error| error.to_string())?
    };

    Ok(Routed {
        options,
        no_legacy_ignore: patterns.no_ignore,
        preview_legacy_reviews,
    })
}

fn resolve_ledger(cli: &Cli, args: &mut Routed, root: &Path) -> Result<(), String> {
    if args.preview_legacy_reviews {
        args.options.legacy_pattern_ledger =
            Some(ignore::ledger_path(cli.ignore_file.as_deref(), root));
        Ok(())
    } else {
        check_ledger(cli, root, args.no_legacy_ignore)
    }
}

fn check_ledger(cli: &Cli, root: &Path, no_ignore: bool) -> Result<(), String> {
    if !no_ignore
        && !ignore::load(&ignore::ledger_path(cli.ignore_file.as_deref(), root))?
            .entries
            .is_empty()
    {
        return Err("Compiler patterns do not automatically migrate legacy dismissals; use --preview-legacy-reviews to inspect migration or --no-ignore to explicitly bypass the legacy ledger, and --review-file for scoped compiler reviews. The legacy ledger is unchanged.".into());
    }
    Ok(())
}
