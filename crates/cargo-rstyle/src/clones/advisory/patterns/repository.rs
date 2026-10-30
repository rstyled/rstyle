//! Repository dismissals are a presentation mask over the original ranked proposals.
use rstyle_core::{
    ledger::{self, Ledger},
    patterns_report::{self, PatternsReport},
};

pub(super) fn apply(
    ledger: Option<&Ledger>,
    evaluation: &mut PatternsReport,
) -> Option<Vec<usize>> {
    let ledger = ledger?;
    let applied = ledger::apply(ledger, evaluation.candidates.iter());

    let mut kept = applied.kept.into_iter().peekable();
    let visible = evaluation
        .candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            kept.next_if(|kept| std::ptr::eq(*kept, candidate))
                .map(|_| index)
        })
        .collect();

    evaluation.ignored = applied
        .hidden
        .into_iter()
        .map(|hidden| patterns_report::ignored_entry(ledger, hidden))
        .collect();
    evaluation.ignored_count = evaluation.ignored.len();
    Some(visible)
}
