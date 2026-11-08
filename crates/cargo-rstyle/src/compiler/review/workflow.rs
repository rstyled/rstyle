//! Explicit review IO around compiler findings; lookup never writes.
use super::*;
use crate::compiler::report;
use std::path::Path;

pub(in crate::compiler) struct Inputs<'a> {
    pub compiler_advisory: Option<&'a crate::compiler::advisory::Report>,
}

pub(in crate::compiler) struct Reports {
    pub compiler: CompilerReviews,
    pub status: Option<status::Report>,
    pub pruning: Option<prune::Report>,
}

pub(in crate::compiler) struct CompilerReviews {
    patterns: super::compiler_patterns::Report,
    diagnostics: super::compiler_diagnostics::Report,
    baseline: Option<super::compiler_baseline::Report>,
    pattern_baseline: Option<super::compiler_pattern_baseline::Report>,
}

impl CompilerReviews {
    fn new(
        inputs: &Inputs<'_>,
        ctx: &report::Context<'_>,
        patterns: super::compiler_patterns::Report,
        diagnostics: super::compiler_diagnostics::Report,
    ) -> Result<Self, String> {
        let baseline = if ctx
            .args
            .compiler_baseline_requested(crate::compiler::CompilerBaseline::Diagnostics)
        {
            Some(super::compiler_baseline::evaluate(
                inputs
                    .compiler_advisory
                    .ok_or("Compiler baseline requires compiler advisory")?,
                &diagnostics,
                ctx,
            )?)
        } else {
            None
        };
        let pattern_baseline = if ctx
            .args
            .compiler_baseline_requested(crate::compiler::CompilerBaseline::Patterns)
        {
            Some(super::compiler_pattern_baseline::evaluate(
                inputs
                    .compiler_advisory
                    .ok_or("Compiler proposal baseline requires compiler advisory")?,
                &patterns,
                ctx,
            )?)
        } else {
            None
        };
        Ok(Self {
            patterns,
            diagnostics,
            baseline,
            pattern_baseline,
        })
    }

    pub(in crate::compiler) fn apply(
        self,
        advisory: Option<crate::compiler::advisory::Report>,
        ctx: &report::Context<'_>,
    ) -> Option<crate::compiler::advisory::Report> {
        advisory.map(|report| {
            report
                .with_reviews(self.patterns, self.diagnostics, self.baseline)
                .with_pattern_baseline(self.pattern_baseline)
                .select_changed(ctx)
        })
    }
}

impl Inputs<'_> {
    /// One load and atomic publication across all requested review families.
    /// A failed selector or snapshot leaves every retained entry untouched on disk.
    // rstyle-allow(side_effect_density): Review transaction boundary evaluates choices and publishes once.
    pub(in crate::compiler) fn apply(
        self,
        ctx: &report::Context<'_>,
        mut store: Option<ReviewStore>,
    ) -> Result<Reports, String> {
        let mut reports = evaluate(&self, ctx, store.as_mut())?;
        reports.pruning = pruning(store.as_mut(), reports.status.as_ref(), ctx)?;

        if (!ctx.args.dismiss_compiler_pattern.is_empty()
            || !ctx.args.dismiss_compiler_diagnostic.is_empty()
            || reports
                .pruning
                .as_ref()
                .is_some_and(|report| report.removed_entries > 0))
            && let (Some(path), Some(store)) = (&ctx.args.review_file, &store)
        {
            save(path, store)?;
        }
        Ok(reports)
    }
}

fn evaluate(
    inputs: &Inputs<'_>,
    ctx: &report::Context<'_>,
    mut store: Option<&mut ReviewStore>,
) -> Result<Reports, String> {
    let (patterns, pattern_records) =
        super::compiler_patterns::apply(inputs, ctx, store.as_deref_mut())?;
    let (diagnostics, diagnostic_records) = super::compiler_diagnostics::apply(inputs, ctx, store)?;
    let compiler = CompilerReviews::new(inputs, ctx, patterns, diagnostics)?;

    let status = ctx.args.review_status_requested().then(|| {
        status::Report::new(
            pattern_records
                .into_iter()
                .chain(diagnostic_records)
                .collect(),
            true,
        )
    });
    Ok(Reports {
        compiler,
        status,
        pruning: None,
    })
}

fn pruning(
    store: Option<&mut ReviewStore>,
    status: Option<&status::Report>,
    ctx: &report::Context<'_>,
) -> Result<Option<prune::Report>, String> {
    if !ctx.args.review_prune {
        return Ok(None);
    }
    if !ctx.capture.failures.is_empty() || !ctx.handoff.failures.is_empty() {
        return Err(
            "Review pruning requires a successful capture and assessment; no entries changed"
                .into(),
        );
    }

    let store = store.ok_or("Review pruning requires a review file")?;
    let status = status.ok_or("Review pruning requires retained-entry assessment")?;
    prune::apply(store, status, ctx.args.review_prune_dry_run).map(Some)
}

pub(super) fn unit<'a>(
    handoff: &crate::compiler::Handoff,
    capture: &'a crate::compiler::capture::Capture,
    record: &crate::compiler::Function,
) -> Result<&'a crate::compiler::capture::Unit, String> {
    let context = handoff
        .unit_contexts
        .iter()
        .find(|context| context.id == record.key.unit())
        .ok_or("Missing review compilation context")?;
    capture
        .units
        .get(context.observed_unit)
        .ok_or("Invalid review compilation context".into())
}

pub(in crate::compiler) fn load(path: Option<&Path>) -> Result<Option<ReviewStore>, String> {
    path.map(load_file).transpose()
}

fn load_file(path: &Path) -> Result<ReviewStore, String> {
    match baseline_io::read_bounded(path) {
        Ok(bytes) => {
            ReviewStore::from_json(std::str::from_utf8(&bytes).map_err(|error| error.to_string())?)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ReviewStore::default()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn save(path: &Path, store: &ReviewStore) -> Result<(), String> {
    let bytes = store.to_json()?;
    publication::write(path, bytes.as_bytes(), publication::Mode::Replace)
}
