//! Repository review commands consume the same compiler proposal pipeline as listing.
use super::*;
use crate::{IgnoreArgs, PatternDisplayArgs, PatternRouteArgs, Session};

pub(crate) fn run(cli: Cli, mut args: IgnoreArgs) -> Result<(String, u8), String> {
    let routed = route(&mut args)?;
    routed.run(cli, Some(&args))
}

fn route(args: &mut IgnoreArgs) -> Result<Routed, String> {
    let listing = PatternsArgs {
        route: PatternRouteArgs {
            input: std::mem::take(&mut args.route),
            ..Default::default()
        },
        display: PatternDisplayArgs::default(),
        no_ignore: true,
        show_ignored: false,
        fail_on_candidates: false,
        mine: std::mem::take(&mut args.mine),
    };
    let mut routed = compiler_args(listing)?;
    routed.options.pattern_session = args.status;
    Ok(routed)
}

pub(super) struct Review<'a> {
    pub args: &'a IgnoreArgs,
    pub ignore_file: Option<std::path::PathBuf>,
}

pub(super) fn publish(
    cli: &Cli,
    review: Option<Review<'_>>,
    captured: Captured,
    options: Args,
) -> Result<(String, u8), String> {
    let Some(review) = review else {
        return super::super::run_captured(cli, &options, captured.config, captured.capture);
    };
    let evaluated = evaluate(cli, &review, captured, options)?;
    finish(evaluated, review.args)
}

pub(super) struct Evaluated {
    session: Session,
    code: u8,
    workspace_all_targets: bool,
}

pub(super) fn evaluate(
    cli: &Cli,
    review: &Review<'_>,
    captured: Captured,
    options: Args,
) -> Result<Evaluated, String> {
    let Captured { config, capture } = captured;
    let args = review.args;

    let workspace_all_targets = capture.plan.workspace_all_targets == Some(true);
    let path = ignore::ledger_path(review.ignore_file.as_deref(), &capture.plan.root);
    let ledger = ignore::load(&path)?;
    let (session, code) = super::super::run_captured_with(
        cli,
        &options,
        super::super::CapturedRun { config, capture },
        |report, _| {
            let advisory = report
                .compiler_advisory
                .ok_or("Compiler proposals were not evaluated")?;
            let (candidates, status) =
                advisory.into_repository_session(&ledger, args.id.as_deref(), args.status)?;
            Ok(Session {
                path,
                ledger,
                candidates,
                status,
            })
        },
    )?;
    Ok(Evaluated {
        session,
        code,
        workspace_all_targets,
    })
}

pub(super) fn finish(evaluated: Evaluated, args: &IgnoreArgs) -> Result<(String, u8), String> {
    let Evaluated {
        session,
        code,
        workspace_all_targets,
    } = evaluated;
    if code != 0 {
        return if args.status {
            Ok((
                "Compiler proposal inputs incomplete; repository member status is unavailable"
                    .into(),
                code,
            ))
        } else {
            Err("Repository review writes require complete compiler proposal inputs".into())
        };
    }
    if args.prune && !workspace_all_targets {
        return Err("Pruning requires a complete workspace capture made with --all-targets; package selection cannot establish that unseen entries are stale".into());
    }
    Ok((crate::apply_ignore(session, args)?, code))
}
