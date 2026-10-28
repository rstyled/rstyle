use super::*;

impl Report<'_> {
    pub(in crate::clones) fn render(&self) -> String {
        if self.scope.execution == Execution::NotRequested {
            return String::new();
        }
        let mut output = format!(
            "\nMutation analysis: {:?} policy coverage; {} visible findings ({} observed, {} suppressed, {} excluded); {} evaluated, {} excluded, {} unavailable policy owners. Raw evidence: {} complete, {} unavailable functions.",
            self.scope.coverage,
            self.visible_findings,
            self.findings,
            self.suppressed_findings,
            self.excluded_findings,
            self.scope.policy_evaluated_functions,
            self.scope.excluded_functions,
            self.scope.policy_unavailable_functions,
            self.scope.evaluated_functions,
            self.scope.unavailable_functions,
        );
        for function in &self.functions {
            function.render(&mut output);
        }
        output
    }
}

impl FunctionReport<'_> {
    fn render(&self, output: &mut String) {
        for rule in self
            .policy
            .rule_decisions
            .iter()
            .filter(|r| r.outcome == Outcome::Finding)
        {
            output.push_str(&format!(
                "\n{} {} at {} [bytes {}..{}]: at least {} sites/roots (limit {}).",
                rule.rule_id,
                self.location.name,
                self.location.source_file.unwrap_or("<source unavailable>"),
                self.location.fn_span.start,
                self.location.fn_span.end,
                rule.actual.unwrap_or(0),
                rule.limit
            ));
        }
        if let Some(invalid) = &self.policy.invalid_suppression {
            output.push_str(&format!(
                "\ninvalid_suppression {} at {} [bytes {}..{}]: {}.",
                self.location.name,
                self.location.source_file.unwrap_or("<source unavailable>"),
                self.location.fn_span.start,
                self.location.fn_span.end,
                invalid.reasons.join("; "),
            ));
        }
    }
}
