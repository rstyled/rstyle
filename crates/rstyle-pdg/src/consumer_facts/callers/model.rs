use super::super::mutations::{Coverage, Execution};
use crate::ByteSpan;
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub functions: usize,
    pub calls: usize,
    pub arguments: usize,
    pub uses: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            functions: 4096,
            calls: 16384,
            arguments: 65536,
            uses: 65536,
        }
    }
}

/// Ordinals borrow an original call retained by this session's immutable unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Witness {
    pub caller_index: usize,
    pub call_index: usize,
    pub target_index: usize,
}

#[derive(Debug, Serialize)]
pub struct Reason {
    pub function_index: Option<usize>,
    pub span: Option<ByteSpan>,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct Census {
    pub coverage: Coverage,
    pub execution: Execution,
    pub expected_functions: usize,
    pub scanned_functions: usize,
    pub call_count: usize,
    pub argument_count: usize,
    pub use_entries: usize,
    pub reasons: Vec<Reason>,
}

impl Census {
    pub(super) fn new(expected_functions: usize) -> Self {
        Self {
            coverage: Coverage::Complete,
            execution: Execution::Completed,
            expected_functions,
            scanned_functions: 0,
            call_count: 0,
            argument_count: 0,
            use_entries: 0,
            reasons: Vec::new(),
        }
    }

    pub(super) fn incomplete(
        &mut self,
        function_index: Option<usize>,
        span: Option<ByteSpan>,
        reason: &str,
    ) {
        self.coverage.record_incomplete(
            &mut self.reasons,
            Reason {
                function_index,
                span,
                reason: reason.into(),
            },
        );
    }

    pub(super) fn exhausted(&mut self, index: Option<usize>, reason: &str) {
        self.execution = Execution::BudgetExhausted;
        self.incomplete(index, None, reason);
    }
}
