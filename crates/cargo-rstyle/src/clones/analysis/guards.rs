//! Advisory guard shapes over exact original source, without rewrite authority.

use super::{Coverage, Execution, FactAvailability, Outcome};
use rstyle_pdg::consumer_facts::Availability;
use serde::Serialize;
use syn::{Expr, Stmt, visit::Visit};

const MAX_SOURCE_BYTES: usize = 256 * 1024;

#[derive(Debug, Serialize)]
pub(super) struct GuardAssessment {
    pub rule_id: &'static str,
    pub execution: Execution,
    pub outcome: Outcome,
    pub coverage: Coverage,
    pub availability: FactAvailability,
    pub opportunity: Option<GuardOpportunity>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct GuardOpportunity {
    pub fallback_branch: Branch,
    pub continuing_statements: usize,
    pub advisory: &'static str,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Branch {
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
        reasons: vec![reason.into()],
    }
}

fn inspect(source: &str) -> GuardAssessment {
    if source.len() > MAX_SOURCE_BYTES {
        let mut result = unavailable(
            FactAvailability::Known,
            "guard source byte budget exhausted before parsing",
        );
        result.execution = Execution::BudgetExhausted;
        return result;
    }
    let function = match syn::parse_str::<syn::ItemFn>(source) {
        Ok(function) => function,
        Err(_) => return unsupported("original source is not a complete function"),
    };
    let mut boundaries = Boundaries::default();
    boundaries.visit_item_fn(&function);
    if boundaries.opaque || function.sig.unsafety.is_some() || function.sig.asyncness.is_some() {
        return unsupported(
            "opaque syntax or ambiguous control flow is outside the guard contract",
        );
    }

    let opportunity = candidate(&function.block);
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
        reasons: vec![],
    }
}

fn unsupported(reason: &str) -> GuardAssessment {
    unavailable(FactAvailability::Unsupported, reason)
}

fn candidate(block: &syn::Block) -> Option<GuardOpportunity> {
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
struct Boundaries {
    opaque: bool,
    terminal: bool,
}

impl<'ast> Visit<'ast> for Boundaries {
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
