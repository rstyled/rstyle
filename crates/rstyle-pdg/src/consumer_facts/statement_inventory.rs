//! Complete original-source statement inventory for density denominators.
//!
//! Each lexical block statement (including its tail) counts once. A match arm
//! whose body is not a block contributes one additional statement. Nested block
//! statements also count, independently of reachability or execution frequency.
//! This is a source syntax metric, not a native HIR, PDG, or legacy statement
//! count. Original whole-file ownership and signature checks bind its input.
//! Opaque, expansion-sensitive, or compile-time expression boundaries make the
//! entire inventory unavailable; their blocks cannot enlarge this denominator.
use super::{Availability, InspectionExecution};
use crate::HirFnFacts;
use syn::visit::Visit;

#[cfg(test)]
mod tests;

/// Availability and execution of the complete original-source denominator.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct Inventory {
    pub statements: Availability<usize>,
    pub execution: InspectionExecution,
}

/// Count source statements after independent ownership and signature checks.
pub fn inspect(facts: &HirFnFacts) -> Result<Inventory, String> {
    let original = super::inspect(facts)?;
    let source = match original.source {
        Availability::Known(source) => source,
        Availability::Unknown { reason } => return Ok(unavailable(reason, original.execution)),
        Availability::Unsupported { reason } | Availability::NotApplicable { reason } => {
            return Ok(unsupported(reason));
        }
    };

    if original.execution == InspectionExecution::BudgetExhausted
        || !super::within_source_budget(source)
    {
        return Ok(unavailable(
            "Original statement inventory exceeds the source syntax budget".into(),
            InspectionExecution::BudgetExhausted,
        ));
    }
    if !matches!(original.signature, Availability::Known(_)) {
        return Ok(unavailable(
            "Original statement inventory requires a complete checked signature".into(),
            original.execution,
        ));
    }

    let function: syn::ItemFn = match syn::parse_str(source) {
        Ok(function) => function,
        Err(_) => {
            return Ok(unsupported(
                "Original function syntax is unavailable".into(),
            ));
        }
    };

    if !function.attrs.is_empty()
        || function.sig.asyncness.is_some()
        || function.sig.unsafety.is_some()
        || facts.body.expansion.is_some()
    {
        return Ok(unsupported(
            "Original function has an opaque syntax boundary".into(),
        ));
    }

    let mut counter = Counter::default();
    counter.visit_block(&function.block);
    Ok(match counter.reason {
        Some(reason) => unsupported(reason.into()),
        None => Inventory {
            statements: Availability::Known(counter.statements),
            execution: InspectionExecution::Completed,
        },
    })
}

fn unavailable(reason: String, execution: InspectionExecution) -> Inventory {
    Inventory {
        statements: Availability::Unknown { reason },
        execution,
    }
}

fn unsupported(reason: String) -> Inventory {
    Inventory {
        statements: Availability::Unsupported { reason },
        execution: InspectionExecution::Completed,
    }
}

#[derive(Default)]
struct Counter {
    statements: usize,
    reason: Option<&'static str>,
}

impl Counter {
    fn increment(&mut self) {
        match self.statements.checked_add(1) {
            Some(count) => self.statements = count,
            None => self.reason = Some("Original statement count overflowed"),
        }
    }
}

impl<'ast> Visit<'ast> for Counter {
    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        if self.reason.is_some() {
            return;
        }
        self.increment();
        syn::visit::visit_stmt(self, statement);
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if !matches!(arm.body.as_ref(), syn::Expr::Block(_)) {
            self.increment();
        }
        syn::visit::visit_arm(self, arm);
    }

    fn visit_item(&mut self, _: &'ast syn::Item) {
        self.reason = Some("Nested items have no complete source statement denominator");
    }

    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {
        self.reason = Some("Attributed syntax may change the original statement inventory");
    }

    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.reason = Some("Macro syntax has no complete original statement denominator");
    }

    fn visit_type_array(&mut self, _: &'ast syn::TypeArray) {
        self.reason = Some("Array type length has a compile-time expression boundary");
    }

    fn visit_generic_argument(&mut self, argument: &'ast syn::GenericArgument) {
        if matches!(
            argument,
            syn::GenericArgument::Const(_) | syn::GenericArgument::AssocConst(_)
        ) {
            self.reason = Some("Const generic argument has a compile-time expression boundary");
            return;
        }
        syn::visit::visit_generic_argument(self, argument);
    }

    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if self.reason.is_some() {
            return;
        }
        if matches!(
            expression,
            syn::Expr::Async(_)
                | syn::Expr::Await(_)
                | syn::Expr::Closure(_)
                | syn::Expr::Const(_)
                | syn::Expr::Repeat(_)
                | syn::Expr::Unsafe(_)
                | syn::Expr::Verbatim(_)
                | syn::Expr::Yield(_)
        ) {
            self.reason = Some("Opaque expression has no complete source statement denominator");
            return;
        }
        syn::visit::visit_expr(self, expression);
    }
}
