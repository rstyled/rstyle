//! Advisory guard shapes over exact original source, without rewrite authority.

use super::{Coverage, Execution, FactAvailability, Outcome};
use rstyle_pdg::{ByteSpan, consumer_facts::Availability, source_evidence::original_span};
use serde::Serialize;
use syn::{Expr, Stmt, spanned::Spanned, visit::Visit};

#[derive(Debug, Serialize)]
pub(super) struct GuardAssessment {
    pub rule_id: &'static str,
    pub execution: Execution,
    pub outcome: Outcome,
    pub coverage: Coverage,
    pub availability: FactAvailability,
    pub opportunity: Option<GuardOpportunity>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub closure_opportunities: Vec<GuardOpportunity>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(in crate::clones) struct GuardOpportunity {
    pub span: ByteSpan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closure_span: Option<ByteSpan>,
    pub fallback_branch: Branch,
    pub continuing_statements: usize,
    pub advisory: &'static str,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::clones) enum Branch {
    Then,
    Else,
}

pub(super) fn evaluate(source: &Availability<&str>) -> GuardAssessment {
    match source {
        Availability::Known(source) => inspect(source),
        Availability::Unknown { reason } => unavailable(FactAvailability::Unknown, reason),
        Availability::Unsupported { reason } => unavailable(FactAvailability::Unsupported, reason),
        Availability::NotApplicable { reason } => {
            unavailable(FactAvailability::NotApplicable, reason)
        }
    }
}

fn unavailable(availability: FactAvailability, reason: &str) -> GuardAssessment {
    GuardAssessment {
        rule_id: "guard_clause",
        execution: Execution::Completed,
        outcome: Outcome::NotEvaluated,
        coverage: Coverage::Incomplete,
        availability,
        opportunity: None,
        closure_opportunities: vec![],
        reasons: vec![reason.into()],
    }
}

fn inspect(source: &str) -> GuardAssessment {
    if !rstyle_pdg::consumer_facts::within_source_budget(source) {
        let mut result = unavailable(
            FactAvailability::Known,
            "guard source syntax budget exhausted before parsing",
        );
        result.execution = Execution::BudgetExhausted;
        return result;
    }
    let function = match syn::parse_str::<syn::ItemFn>(source) {
        Ok(function) => function,
        Err(_) => return unsupported("original source is not a complete function"),
    };
    if function.sig.unsafety.is_some() || function.sig.asyncness.is_some() {
        return unsupported("unsafe or async functions are outside the guard contract");
    }
    let mut boundaries = Boundaries::for_source(source);
    boundaries.visit_item_fn(&function);
    if boundaries.opaque {
        let mut result =
            unsupported("opaque syntax or ambiguous control flow is outside the guard contract");
        result.closure_opportunities = boundaries.opportunities;
        if !result.closure_opportunities.is_empty() {
            result.outcome = Outcome::Finding;
        }
        return result;
    }

    let opportunity = candidate(&function.block, source);
    GuardAssessment {
        rule_id: "guard_clause",
        execution: Execution::Completed,
        outcome: if opportunity.is_some() {
            Outcome::Finding
        } else {
            Outcome::EvaluatedNoFinding
        },
        coverage: Coverage::Complete,
        availability: FactAvailability::Known,
        opportunity,
        closure_opportunities: vec![],
        reasons: vec![],
    }
}

fn unsupported(reason: &str) -> GuardAssessment {
    unavailable(FactAvailability::Unsupported, reason)
}

fn candidate(block: &syn::Block, source: &str) -> Option<GuardOpportunity> {
    let Stmt::Expr(Expr::If(branch), None) = block.stmts.last()? else {
        return None;
    };
    let Expr::Block(alternative) = &*branch.else_branch.as_ref()?.1 else {
        return None;
    };
    let mut control = Boundaries::default();
    control.visit_expr(&branch.cond);
    for statement in &block.stmts[..block.stmts.len() - 1] {
        control.visit_stmt(statement);
    }
    if control.terminal {
        return None;
    }

    let (fallback_branch, continuing) = if fallback(&branch.then_branch) {
        (Branch::Then, &alternative.block)
    } else if fallback(&alternative.block) {
        (Branch::Else, &branch.then_branch)
    } else {
        return None;
    };

    continuing_shape(continuing).then_some(GuardOpportunity {
        span: original_span(source, branch.span()),
        closure_span: None,
        fallback_branch,
        continuing_statements: continuing.stmts.len(),
        advisory: "consider an early scalar return and lifting the continuing branch; review lifetimes, drops, and condition inversion before editing",
    })
}

fn fallback(block: &syn::Block) -> bool {
    match block.stmts.as_slice() {
        [Stmt::Expr(Expr::Return(value), _)] => value.expr.as_deref().is_none_or(scalar),
        [Stmt::Expr(value, None)] => scalar(value),
        _ => false,
    }
}

fn scalar(expression: &Expr) -> bool {
    match expression {
        Expr::Lit(value) => matches!(
            value.lit,
            syn::Lit::Bool(_) | syn::Lit::Int(_) | syn::Lit::Char(_)
        ),
        Expr::Tuple(value) => value.elems.is_empty(),
        _ => false,
    }
}

fn continuing_shape(block: &syn::Block) -> bool {
    let mut boundaries = Boundaries::default();
    boundaries.visit_block(block);
    block.stmts.len() >= 2
        && matches!(block.stmts.last(), Some(Stmt::Expr(_, None)))
        && !boundaries.terminal
        && !boundaries.opaque
}

#[derive(Default)]
struct Boundaries<'source> {
    source: Option<&'source str>,
    opportunities: Vec<GuardOpportunity>,
    opaque: bool,
    terminal: bool,
}

impl<'source> Boundaries<'source> {
    fn for_source(source: &'source str) -> Self {
        Self {
            source: Some(source),
            ..Self::default()
        }
    }

    fn inspect_closure(&mut self, closure: &syn::ExprClosure) {
        let Some(source) = self.source else { return };
        if closure.asyncness.is_some()
            || closure.constness.is_some()
            || closure.movability.is_some()
            || closure.lifetimes.is_some()
            || !closure.attrs.is_empty()
        {
            return;
        }
        let Expr::Block(body) = &*closure.body else {
            return;
        };
        if body.label.is_some() || !body.attrs.is_empty() {
            return;
        }

        let mut local = Self::for_source(source);
        for input in &closure.inputs {
            local.visit_pat(input);
        }
        local.visit_return_type(&closure.output);
        local.visit_block(&body.block);
        if !local.opaque
            && let Some(mut opportunity) = candidate(&body.block, source)
        {
            opportunity.closure_span = Some(original_span(source, closure.span()));
            opportunity.advisory = "consider an early scalar return within this closure and lifting the continuing branch; review lifetimes, drops, and condition inversion before editing";
            self.opportunities.push(opportunity);
        }
        self.opportunities.extend(local.opportunities);
    }
}

impl<'ast> Visit<'ast> for Boundaries<'_> {
    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        self.opaque = true;
        self.inspect_closure(closure);
    }

    fn visit_expr(&mut self, expression: &'ast Expr) {
        self.terminal |= matches!(expression, Expr::Return(_));
        self.opaque |= opaque_expression(expression);
        syn::visit::visit_expr(self, expression);
    }

    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.opaque = true;
    }

    fn visit_item(&mut self, _: &'ast syn::Item) {
        self.opaque = true;
    }

    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {
        self.opaque = true;
    }
}

fn opaque_expression(expression: &Expr) -> bool {
    match expression {
        Expr::Async(_)
        | Expr::Await(_)
        | Expr::Break(_)
        | Expr::Closure(_)
        | Expr::Continue(_)
        | Expr::ForLoop(_)
        | Expr::Loop(_)
        | Expr::Match(_)
        | Expr::Try(_)
        | Expr::TryBlock(_)
        | Expr::Unsafe(_)
        | Expr::While(_)
        | Expr::Yield(_)
        | Expr::Let(_) => true,
        Expr::Block(block) => block.label.is_some(),
        Expr::If(branch) => branch
            .else_branch
            .as_ref()
            .is_some_and(|(_, alternative)| matches!(&**alternative, Expr::If(_))),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
