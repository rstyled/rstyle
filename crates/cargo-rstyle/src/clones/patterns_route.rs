//! Pattern listing and repository review share the compiler workflow.
mod session;
use super::{Args, capture_input};
use crate::{Cli, PatternsArgs, ignore};
use clap::{Args as _, FromArgMatches};
pub(crate) use session::run as ignore;

pub(crate) fn run(cli: Cli, patterns: PatternsArgs) -> Result<(String, u8), String> {
    compiler_args(patterns)?.run(cli, None)
}

struct Captured {
    config: super::Configuration,
    capture: super::capture::Capture,
}

fn prepare(cli: &mut Cli, args: Routed) -> Result<(Captured, Args), String> {
    let capture = capture_input(cli, &args.options)?;
    let mut options = ledger_options(cli, args, &capture.plan.root);
    let config = super::style_route::configure(cli, &mut options, &capture.plan.root)?;
    Ok((Captured { config, capture }, options))
}

fn ledger_options(cli: &Cli, args: Routed, root: &std::path::Path) -> Args {
    Args {
        pattern_ledger: (!args.no_ignore)
            .then(|| ignore::ledger_path(cli.ignore_file.as_deref(), root)),
        ..args.options
    }
}

struct Routed {
    options: Args,
    no_ignore: bool,
}

impl Routed {
    fn run(self, cli: Cli, review: Option<&crate::IgnoreArgs>) -> Result<(String, u8), String> {
        let ignore_file = review.and(cli.ignore_file.as_ref()).cloned();
        let review = review.map(|args| session::Review { args, ignore_file });
        Execution {
            cli,
            routed: self,
            review,
        }
        .run()
    }
}

struct Execution<'a> {
    cli: Cli,
    routed: Routed,
    review: Option<session::Review<'a>>,
}

impl Execution<'_> {
    fn run(mut self) -> Result<(String, u8), String> {
        let replay = self.routed.options.replay_run.is_some();
        super::with_capture_verification(replay, || {
            let (captured, options) = prepare(&mut self.cli, self.routed)?;
            session::publish(&self.cli, self.review, captured, options)
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
        style_inputs_only: true,
        mine: patterns.mine,
        show_ignored: patterns.show_ignored,
        pattern_display: patterns.display,
        fail_on_findings: patterns.fail_on_candidates,
        compiler_pattern_scope: patterns
            .route
            .input
            .compiler_pattern_scope
            .unwrap_or(super::CompilerSourceScope::Authored),
        replay_run: patterns.route.input.replay_run,
        review_file: patterns.route.review_file,
        review_status: patterns.route.review_status,
        locked: patterns.route.input.locked,
        dismiss_compiler_pattern: patterns.route.dismiss_compiler_pattern,
        review_reason: patterns.route.review_reason,
        ..Args::from_arg_matches(&matches).map_err(|error| error.to_string())?
    };

    Ok(Routed {
        options,
        no_ignore: patterns.no_ignore,
    })
}
