mod branches;

use super::{Assessment, Execution, Operation, Provenance, Root, WriteSite, native};
use crate::{ByteSpan, HirFnFacts, HirNode};
use std::collections::{BTreeMap, BTreeSet};
use syn::{Expr, Pat, Type, spanned::Spanned, visit::Visit};

pub(super) use crate::consumer_facts::syntax_budget::within_budget;

#[cfg(test)]
pub(super) fn inspect(
    facts: &HirFnFacts,
    source: &str,
    result: Assessment,
) -> Result<Assessment, String> {
    Ok(
        observe_with_signature(facts, source, result, super::Request::default())?
            .0
            .direct,
    )
}

pub(super) fn observe_with_signature(
    facts: &HirFnFacts,
    source: &str,
    result: Assessment,
    request: super::Request,
) -> Result<(super::Observations, bool), String> {
    let function = match parsed(source) {
        Ok(function) => function,
        Err(reason) => {
            return Ok((
                super::without_calls(Effects::unsupported(whole(source), &reason).merge(result)),
                false,
            ));
        }
    };
    super::super::check_signature(facts, &function)?;
    let Some(index) = native::Index::new(facts)? else {
        return Ok((
            super::without_calls(
                Effects::exhausted(whole(source), "Native inventory budget exhausted")
                    .merge(result),
            ),
            true,
        ));
    };

    let mut poison = Poison::default();
    poison.visit_block(&function.block);
    let mut scan = Scan {
        facts,
        source,
        index,
        result,
        scopes: vec![BTreeMap::new()],
        poison: poison.names,
        opaque_references: poison.opaque_references,
        declarations: 0,
        calls: request.calls.then(Vec::new),
        uses: request.uses.then(super::uses::Scan::new),
    };

    scan.signature(&function)?;
    scan.block(&function.block)?;
    scan.finish(&function).map(|observed| (observed, true))
}

fn whole(source: &str) -> ByteSpan {
    ByteSpan {
        start: 0,
        end: source.len() as u32,
    }
}

fn parsed(source: &str) -> Result<syn::ItemFn, String> {
    let function: syn::ItemFn =
        syn::parse_str(source).map_err(|_| "Original function syntax unavailable")?;
    if !function.attrs.is_empty() {
        return Err("Function attributes may transform the entire body".into());
    }

    Ok(function)
}

// Derive observations without editing scan state, then apply one ordered batch.
struct Effects {
    sites: Vec<WriteSite>,
    boundaries: Vec<super::Boundary>,
    invalidate: bool,
    execution: Execution,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            sites: vec![],
            boundaries: vec![],
            invalidate: false,
            execution: Execution::Completed,
        }
    }
}

impl Effects {
    fn unsupported(span: ByteSpan, reason: &str) -> Self {
        Self::boundary(span, reason, super::BoundaryAvailability::Unsupported)
    }

    fn unknown(span: ByteSpan, reason: &str) -> Self {
        Self::boundary(span, reason, super::BoundaryAvailability::Unknown)
    }

    fn boundary(span: ByteSpan, reason: &str, availability: super::BoundaryAvailability) -> Self {
        Self {
            boundaries: vec![super::Boundary {
                span,
                reason: reason.into(),
                availability,
            }],
            ..Self::default()
        }
    }

    fn opaque(span: ByteSpan, reason: &str) -> Self {
        Self {
            invalidate: true,
            ..Self::unsupported(span, reason)
        }
    }

    fn exhausted(span: ByteSpan, reason: &str) -> Self {
        Self {
            execution: Execution::BudgetExhausted,
            ..Self::unknown(span, reason)
        }
    }

    fn merge(self, current: Assessment) -> Assessment {
        let Assessment {
            availability,
            coverage,
            execution,
            mut sites,
            mut boundaries,
        } = current;
        let coverage = if coverage == super::Coverage::Complete && self.boundaries.is_empty() {
            super::Coverage::Complete
        } else {
            super::Coverage::Incomplete
        };
        let execution = if execution == Execution::BudgetExhausted {
            execution
        } else {
            self.execution
        };

        sites.extend(self.sites);
        boundaries.extend(self.boundaries);
        Assessment {
            availability,
            coverage,
            execution,
            sites,
            boundaries,
        }
    }
}

#[derive(Clone, Copy)]
enum NativeEffect {
    Value,
    Operation,
}

impl NativeEffect {
    fn missing(self) -> &'static str {
        match self {
            Self::Value => "Missing original native value evidence",
            Self::Operation => "Missing original native operation evidence",
        }
    }

    fn inspect(self, node: &HirNode, span: ByteSpan) -> Effects {
        match self {
            Self::Value => value_effects(node, span),
            Self::Operation => primitive_effects(node, span),
        }
    }
}

struct ExpressionPlan<'a> {
    effects: Effects,
    children: Vec<&'a Expr>,
    block: Option<&'a syn::Block>,
}

impl<'a> ExpressionPlan<'a> {
    fn effects(effects: Effects) -> Self {
        Self {
            effects,
            children: vec![],
            block: None,
        }
    }

    fn children(children: Vec<&'a Expr>) -> Self {
        Self {
            children,
            ..Self::effects(Effects::default())
        }
    }
}

struct DeclarationPlan {
    effects: Effects,
    bindings: BTreeMap<String, Option<Binding>>,
}

struct BindingDecision {
    binding: Option<Binding>,
    effects: Effects,
}

struct LocalPlan<'a> {
    effects: Effects,
    initializer: Option<&'a Expr>,
    pattern: Option<&'a Pat>,
}

#[derive(Clone)]
struct Binding {
    root: Root,
    mutable: bool,
}

impl Binding {
    fn new(node: &HirNode, context: DeclarationContext, mutable: bool) -> Self {
        Self {
            root: Root {
                declaration_span: node.span,
                scope_span: context.scope,
                native: node.hir.clone(),
                formal: context.formal,
                provenance: context.provenance,
            },
            mutable,
        }
    }
}

#[derive(Clone, Copy)]
struct DeclarationContext {
    scope: ByteSpan,
    formal: Option<u32>,
    provenance: Provenance,
}

struct Assignment<'a> {
    expression: &'a Expr,
    left: &'a Expr,
    right: &'a Expr,
    operation: Operation,
}

impl<'a> Assignment<'a> {
    fn plain(expression: &'a Expr, assign: &'a syn::ExprAssign) -> Self {
        Self {
            expression,
            left: &assign.left,
            right: &assign.right,
            operation: Operation::Assignment,
        }
    }

    fn compound(expression: &'a Expr, binary: &'a syn::ExprBinary) -> Self {
        Self {
            expression,
            left: &binary.left,
            right: &binary.right,
            operation: Operation::CompoundAssignment,
        }
    }
}

struct Scan<'a> {
    facts: &'a HirFnFacts,
    source: &'a str,
    index: native::Index<'a>,
    result: Assessment,
    scopes: Vec<BTreeMap<String, Option<Binding>>>,
    poison: BTreeSet<String>,
    opaque_references: bool,
    declarations: usize,
    calls: Option<Vec<super::Call>>,
    uses: Option<super::uses::Scan>,
}

impl Scan<'_> {
    fn finish(self, function: &syn::ItemFn) -> Result<super::Observations, String> {
        let signature = if self.calls.is_some() {
            super::calls::signature(self.facts, function)?
        } else {
            super::Availability::Unknown {
                reason: "Call observations were not requested".into(),
            }
        };
        let uses = self
            .uses
            .map_or_else(super::UseInventory::unrequested, |uses| {
                uses.finish(self.facts.signature.params.len(), &signature)
            });
        Ok(super::Observations {
            uses,
            direct: self.result,
            calls: self.calls.unwrap_or_default(),
            signature,
        })
    }

    fn span(&self, value: &impl Spanned) -> ByteSpan {
        crate::source_evidence::original_span(self.source, value.span())
    }

    fn boundary(&mut self, value: &impl Spanned, reason: &str) {
        let boundary = super::Boundary {
            span: self.span(value),
            availability: super::BoundaryAvailability::Unsupported,
            reason: reason.into(),
        };
        self.result
            .coverage
            .record_incomplete(&mut self.result.boundaries, boundary);
    }

    fn signature(&mut self, function: &syn::ItemFn) -> Result<(), String> {
        if !function.attrs.is_empty()
            || function.sig.receiver().is_some()
            || !function.sig.generics.params.is_empty()
            || function.sig.unsafety.is_some()
            || function.sig.asyncness.is_some()
            || function.sig.variadic.is_some()
        {
            self.boundary(
                &function.sig,
                "Unsupported function signature or expansion attributes",
            );
        }
        let scope = ByteSpan {
            start: 0,
            end: self.source.len() as u32,
        };
        for (ordinal, argument) in function.sig.inputs.iter().enumerate() {
            let syn::FnArg::Typed(argument) = argument else {
                continue;
            };
            self.formal(ordinal, argument, scope)?;
        }
        Ok(())
    }

    fn formal(
        &mut self,
        ordinal: usize,
        argument: &syn::PatType,
        scope: ByteSpan,
    ) -> Result<(), String> {
        let borrowed = matches!(&*argument.ty, Type::Reference(ty) if ty.mutability.is_some() && crate::source_evidence::plain_type_path(&ty.elem, "u32"));
        let provenance = if borrowed {
            Provenance::FormalBorrowed
        } else {
            Provenance::FormalOwned
        };
        self.declare(
            &argument.pat,
            DeclarationContext {
                scope,
                formal: Some(ordinal as u32),
                provenance,
            },
        )?;

        if self.uses.is_some() {
            self.use_formal(&argument.pat, &argument.ty)?;
        }

        let observed = &self.facts.signature.params[ordinal].ty;
        if !borrowed
            && !matches!(observed.normalized.as_deref(), Some("u32" | "bool"))
            && !benign_formal(&argument.ty)
        {
            self.boundary(
                argument,
                "Unsupported formal type or nontrivial drop boundary",
            );
        }
        Ok(())
    }

    fn declare(&mut self, pattern: &Pat, context: DeclarationContext) -> Result<(), String> {
        let plan = if self.declarations >= 4096 {
            DeclarationPlan {
                effects: Effects::exhausted(self.span(pattern), "Root inventory budget exhausted"),
                bindings: BTreeMap::new(),
            }
        } else {
            self.declaration(pattern, context)?
        };

        if !plan.effects.boundaries.is_empty()
            && !matches!(pattern, Pat::Wild(wild) if wild.attrs.is_empty())
        {
            self.use_boundary(
                self.span(pattern),
                "Original declaration is outside checked formal-use bindings",
            );
        }
        self.install_declaration(plan);
        Ok(())
    }

    fn install_declaration(&mut self, plan: DeclarationPlan) {
        self.declarations += 1;
        self.apply(plan.effects);
        self.scopes
            .last_mut()
            .expect("root scope exists")
            .extend(plan.bindings);
    }

    fn declaration(
        &self,
        pattern: &Pat,
        context: DeclarationContext,
    ) -> Result<DeclarationPlan, String> {
        let pattern = if let Pat::Type(typed) = pattern {
            &*typed.pat
        } else {
            pattern
        };
        let Pat::Ident(ident) = pattern else {
            return Ok(self.unsupported_pattern(pattern));
        };
        if ident.subpat.is_some() {
            return Ok(self.unsupported_pattern(pattern));
        }

        let decision = self.binding(ident, context)?;
        Ok(DeclarationPlan {
            effects: decision.effects,
            bindings: BTreeMap::from([(ident.ident.to_string(), decision.binding)]),
        })
    }

    fn unsupported_pattern(&self, pattern: &Pat) -> DeclarationPlan {
        let mut names = PatternNames::default();
        names.visit_pat(pattern);
        let effects = Effects {
            invalidate: names.opaque,
            ..Effects::unsupported(self.span(pattern), "Unsupported declaration pattern")
        };

        DeclarationPlan {
            effects,
            bindings: names.names.into_iter().map(|name| (name, None)).collect(),
        }
    }

    fn binding(
        &self,
        ident: &syn::PatIdent,
        context: DeclarationContext,
    ) -> Result<BindingDecision, String> {
        if ident.by_ref.is_some() || ident.subpat.is_some() || !ident.attrs.is_empty() {
            return Ok(BindingDecision {
                binding: None,
                effects: Effects::unsupported(self.span(ident), "Unsupported binding mode"),
            });
        }
        let declaration_span = self.span(&ident.ident);
        let Some(node) = self.index.node("pat_binding", declaration_span)? else {
            return Ok(BindingDecision {
                binding: None,
                effects: Effects::unknown(
                    self.span(ident),
                    "Original binding lacks native correspondence",
                ),
            });
        };
        if node.ty.is_none() {
            return Ok(BindingDecision {
                binding: None,
                effects: Effects::unknown(self.span(ident), "Native binding type was not captured"),
            });
        }

        self.check_binding(node, ident, context)?;
        Ok(self.binding_decision(node, ident, context))
    }

    fn binding_decision(
        &self,
        node: &HirNode,
        ident: &syn::PatIdent,
        context: DeclarationContext,
    ) -> BindingDecision {
        let DeclarationContext {
            formal, provenance, ..
        } = context;
        if provenance != Provenance::FormalBorrowed && !native::scalar(node) {
            let effects = if formal.is_none() && !benign_node(node) {
                Effects::unsupported(
                    self.span(ident),
                    "Unsupported local type or nontrivial drop boundary",
                )
            } else {
                Effects::default()
            };
            return BindingDecision {
                binding: None,
                effects,
            };
        }
        if provenance == Provenance::FormalBorrowed
            && (self.opaque_references || self.poison.contains(&ident.ident.to_string()))
        {
            return BindingDecision {
                binding: None,
                effects: Effects::unsupported(
                    self.span(ident),
                    "Reference reassignment or mutable reference-binding escape",
                ),
            };
        }

        BindingDecision {
            binding: Some(Binding::new(node, context, ident.mutability.is_some())),
            effects: Effects::default(),
        }
    }

    fn check_binding(
        &self,
        node: &HirNode,
        ident: &syn::PatIdent,
        context: DeclarationContext,
    ) -> Result<(), String> {
        let DeclarationContext { scope, formal, .. } = context;
        let declaration = self.span(&ident.ident);
        let mutable = ident.mutability.is_some();

        let binding = self.index.binding(node)?;
        if let Some(ordinal) = formal {
            let parameter = self
                .facts
                .signature
                .params
                .get(ordinal as usize)
                .ok_or("Mutation formal ordinal is outside its signature")?;
            if parameter.symbol != binding.symbol
                || parameter.pat.hir != node.hir
                || parameter.ty.normalized != binding.ty.normalized
            {
                return Err("Mutation formal type or identity contradicts its binding".into());
            }
        }

        let kind = if formal.is_some() { "fn" } else { "block" };
        let expected_scope = format!("scope:source:{kind}:{}:{}", scope.start, scope.end);
        if binding.decl_span != declaration
            || binding.scope != expected_scope
            || binding.is_param != formal.is_some()
            || binding.is_mut != mutable
            || node.binding_mut != Some(mutable)
            || node.param_index != formal
            || node.binding.as_ref() != Some(&binding.symbol)
            || node.ty.as_ref().map(|ty| &ty.normalized) != Some(&binding.ty.normalized)
            || crate::source_evidence::byte_range(self.source, declaration)? != binding.name
        {
            return Err("Mutation binding identity contradicts its original declaration".into());
        }
        Ok(())
    }

    fn block(&mut self, block: &syn::Block) -> Result<(), String> {
        let scope = self.span(block);
        self.scopes.push(BTreeMap::new());

        let result = self.statements(&block.stmts, scope);
        self.scopes.pop();
        result
    }

    fn statements(&mut self, statements: &[syn::Stmt], scope: ByteSpan) -> Result<(), String> {
        for statement in statements {
            if self.result.execution == Execution::BudgetExhausted {
                break;
            }
            match statement {
                syn::Stmt::Local(local) => self.local(local, scope)?,
                syn::Stmt::Expr(expression, _) => self.expression(expression)?,
                _ => self.opaque_statement(statement),
            }
        }
        Ok(())
    }

    fn opaque_statement(&mut self, statement: &syn::Stmt) {
        self.use_boundary(
            self.span(statement),
            "Opaque original item or macro statement may hide formal uses",
        );
        self.apply(Effects::opaque(
            self.span(statement),
            "Opaque original item or macro statement",
        ));
    }

    fn local(&mut self, local: &syn::Local, scope: ByteSpan) -> Result<(), String> {
        if !local.attrs.is_empty()
            || local
                .init
                .as_ref()
                .is_some_and(|init| init.diverge.is_some())
        {
            self.use_boundary(
                self.span(local),
                "Local attributes or let-else may hide formal uses",
            );
        }
        let plan = self.local_plan(local);
        if let Some(expression) = plan.initializer {
            self.expression(expression)?;
        }
        self.install_local(plan, scope)
    }

    fn install_local(&mut self, plan: LocalPlan<'_>, scope: ByteSpan) -> Result<(), String> {
        self.apply(plan.effects);

        if let Some(pattern) = plan.pattern {
            self.declare(
                pattern,
                DeclarationContext {
                    scope,
                    formal: None,
                    provenance: Provenance::LocalOwned,
                },
            )?;
        }
        Ok(())
    }

    fn local_plan<'a>(&self, local: &'a syn::Local) -> LocalPlan<'a> {
        if !local.attrs.is_empty() {
            return LocalPlan {
                effects: Effects::opaque(self.span(local), "Local declaration attributes"),
                initializer: None,
                pattern: None,
            };
        }
        let effects = if local
            .init
            .as_ref()
            .is_some_and(|init| init.diverge.is_some())
        {
            Effects::unsupported(self.span(local), "Let-else control boundary")
        } else {
            Effects::default()
        };

        LocalPlan {
            effects,
            initializer: local.init.as_ref().map(|init| &*init.expr),
            pattern: Some(&local.pat),
        }
    }

    fn invalidate_roots(&mut self) {
        self.scopes
            .iter_mut()
            .flat_map(BTreeMap::values_mut)
            .for_each(|binding| *binding = None);
    }

    fn apply(&mut self, effects: Effects) {
        if effects.execution == Execution::BudgetExhausted
            && let Some(uses) = &mut self.uses
        {
            uses.exhausted(whole(self.source));
        }
        let invalidate = effects.invalidate;
        self.merge_effects(effects);

        if invalidate {
            self.invalidate_roots();
        }
    }

    fn merge_effects(&mut self, effects: Effects) {
        let previous = std::mem::replace(&mut self.result, Assessment::empty());
        self.result = effects.merge(previous);
    }

    fn expression(&mut self, expression: &Expr) -> Result<(), String> {
        if self.calls.is_some() && self.result.execution == Execution::BudgetExhausted {
            return Ok(());
        }
        if (self.calls.is_some() || self.uses.is_none())
            && let Expr::If(branch) = expression
            && branch.attrs.is_empty()
            && plain_condition(&branch.cond)
        {
            return self.conditional_expression(branch);
        }
        if self.calls.is_some()
            && let Expr::Call(call) = expression
            && call.attrs.is_empty()
        {
            return self.observe_call(call);
        }

        self.regular_expression(expression)
    }

    fn regular_expression(&mut self, expression: &Expr) -> Result<(), String> {
        if self.uses.is_some() {
            self.expression_use(expression, UseMode::Expression)?;
        }

        let plan = self.expression_plan(expression)?;
        self.evaluate_plan(plan)
    }

    fn conditional_calls(&mut self, branch: &syn::ExprIf) -> Result<(), String> {
        let span = self.span(branch);
        self.use_boundary(
            span,
            "Conditional control flow leaves formal-use coverage incomplete",
        );
        self.apply(Effects::opaque(
            span,
            "Conditional control flow is not effect-certified",
        ));
        self.conditional_branches(branch)
    }

    fn conditional_branches(&mut self, branch: &syn::ExprIf) -> Result<(), String> {
        self.conditional_piece(ConditionalPiece::Expression(&branch.cond))?;
        self.conditional_piece(ConditionalPiece::Block(&branch.then_branch))?;
        if let Some((_, alternative)) = &branch.else_branch {
            self.conditional_piece(ConditionalPiece::Expression(alternative))?;
        }
        Ok(())
    }

    fn conditional_piece(&mut self, piece: ConditionalPiece<'_>) -> Result<(), String> {
        if self.result.execution == Execution::BudgetExhausted {
            return Ok(());
        }

        self.invalidate_roots();
        let result = self.evaluate_conditional_piece(piece);
        self.invalidate_roots();
        result
    }

    fn evaluate_conditional_piece(&mut self, piece: ConditionalPiece<'_>) -> Result<(), String> {
        match piece {
            ConditionalPiece::Expression(expression) => self.expression(expression),
            ConditionalPiece::Block(block) => self.block(block),
        }
    }

    fn evaluate_plan(&mut self, plan: ExpressionPlan<'_>) -> Result<(), String> {
        self.apply(plan.effects);
        if let Some(block) = plan.block {
            self.block(block)?;
        }
        for child in plan.children {
            self.expression(child)?;
        }
        Ok(())
    }

    fn observe_call(&mut self, call: &syn::ExprCall) -> Result<(), String> {
        let boundary_index = self.result.boundaries.len();
        if self.calls.as_ref().is_some_and(|calls| calls.len() >= 4096) {
            self.apply(Effects::exhausted(
                self.span(call),
                "Call-site inventory budget exhausted",
            ));
            return Ok(());
        }

        let site = super::calls::Site {
            facts: self.facts,
            source: self.source,
            index: &self.index,
            call,
            boundary_index,
        };
        let observation = super::calls::observe(site, |expression, node| {
            self.call_argument(expression, node)
        })?;
        self.apply(Effects::unknown(self.span(call), "Unknown call boundary"));

        self.retain_call(call, observation)
    }

    fn retain_call(
        &mut self,
        call: &syn::ExprCall,
        observation: Option<super::Call>,
    ) -> Result<(), String> {
        if let Some(observation) = observation {
            if self.uses.is_some() {
                self.call_uses(call, &observation)?;
            }
            self.calls
                .as_mut()
                .expect("call observations requested")
                .push(observation);
        } else {
            self.use_boundary(
                self.span(call),
                "Original call observation is unavailable for formal-use roles",
            );
        }
        Ok(())
    }

    fn call_argument(
        &self,
        expression: &Expr,
        node: &HirNode,
    ) -> Result<super::calls::ArgumentEvidence, String> {
        use super::{ArgumentValue, Availability, calls::ArgumentEvidence};
        let expression = transparent(expression);
        if matches!(expression, Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(_), attrs, .. }) if attrs.is_empty())
            && node.kind == "expr_lit"
            && native::scalar(node)
        {
            return Ok(ArgumentEvidence::unrooted(Availability::Known(
                ArgumentValue::OwnedU32,
            )));
        }
        let Expr::Path(path) = expression else {
            return Ok(ArgumentEvidence::unrooted(Availability::Unsupported {
                reason: "Call argument evaluation is outside direct scalar reads".into(),
            }));
        };

        let binding = plain_path(path)
            .and_then(|name| self.scopes.iter().rev().find_map(|scope| scope.get(&name)))
            .and_then(Option::as_ref);
        let Some(binding) = binding else {
            return Ok(ArgumentEvidence::unrooted(Availability::Unknown {
                reason: "Call argument has no checked lexical scalar root".into(),
            }));
        };
        if !check_call_argument(self.source, path, node, binding)? {
            return Ok(ArgumentEvidence::unrooted(Availability::Unknown {
                reason: "Call argument native type or binding resolution was not captured".into(),
            }));
        }

        self.argument_evidence(binding)
    }

    fn argument_evidence(
        &self,
        binding: &Binding,
    ) -> Result<super::calls::ArgumentEvidence, String> {
        let value = self.typed_call_argument(binding)?;
        let unchanged_formal = if !binding.mutable
            && binding.root.provenance == Provenance::FormalOwned
            && matches!(
                value,
                super::Availability::Known(super::ArgumentValue::OwnedU32)
            ) {
            binding.root.formal
        } else {
            None
        };

        Ok(super::calls::ArgumentEvidence {
            value,
            formal: binding.root.formal,
            unchanged_formal,
        })
    }

    fn typed_call_argument(
        &self,
        binding: &Binding,
    ) -> Result<super::Availability<super::ArgumentValue>, String> {
        use super::{ArgumentValue, Availability};
        if binding.root.provenance != Provenance::FormalBorrowed {
            return Ok(Availability::Known(ArgumentValue::OwnedU32));
        }

        if let Some(formal) = binding.root.formal
            && binding.root.provenance == Provenance::FormalBorrowed
            && super::super::formal_types::mutable_u32(self.facts, formal, &binding.root.native)?
        {
            return Ok(Availability::Known(ArgumentValue::MutableFormal(
                binding.root.clone(),
            )));
        }

        Ok(Availability::Unknown {
            reason: "Call argument scalar reference type is unavailable".into(),
        })
    }

    fn expression_plan<'a>(&self, expression: &'a Expr) -> Result<ExpressionPlan<'a>, String> {
        if !crate::conditional_source::expression_attributes(expression).is_empty() {
            return Ok(ExpressionPlan::effects(Effects::opaque(
                self.span(expression),
                "Expression attributes may introduce lexical bindings",
            )));
        }

        Ok(match expression {
            Expr::Assign(assign) => self.assignment(Assignment::plain(expression, assign))?,
            Expr::Binary(binary) if compound(&binary.op) => {
                self.assignment(Assignment::compound(expression, binary))?
            }
            Expr::Binary(binary) => ExpressionPlan {
                effects: self.native_effect(expression, "expr_binary", NativeEffect::Operation)?,
                ..ExpressionPlan::children(vec![&binary.left, &binary.right])
            },
            Expr::Unary(unary) => ExpressionPlan {
                effects: self.native_effect(expression, "expr_unary", NativeEffect::Operation)?,
                ..ExpressionPlan::children(vec![&unary.expr])
            },
            Expr::Lit(_) => ExpressionPlan::effects(self.native_effect(
                expression,
                "expr_lit",
                NativeEffect::Value,
            )?),
            Expr::Path(_) => ExpressionPlan::effects(self.native_effect(
                expression,
                "expr_path",
                NativeEffect::Value,
            )?),
            Expr::Paren(paren) => ExpressionPlan::children(vec![&paren.expr]),
            Expr::Group(group) => ExpressionPlan::children(vec![&group.expr]),
            Expr::Block(block) if block.label.is_none() && block.attrs.is_empty() => {
                ExpressionPlan {
                    block: Some(&block.block),
                    ..ExpressionPlan::children(vec![])
                }
            }
            _ => ExpressionPlan::effects(self.opaque_expression(expression)),
        })
    }

    fn opaque_expression(&self, expression: &Expr) -> Effects {
        let span = self.span(expression);
        match expression {
            Expr::Call(_) | Expr::MethodCall(_) => Effects::unknown(span, "Unknown call boundary"),
            Expr::Closure(_) => Effects::unsupported(span, "Deferred closure body boundary"),
            Expr::Macro(_) => Effects::opaque(span, "Opaque macro may introduce lexical bindings"),
            Expr::Reference(_) => Effects::unsupported(span, "Alias or reborrow boundary"),
            _ => Effects::unsupported(
                span,
                "Unsupported control, projection, expansion or effect boundary",
            ),
        }
    }

    fn native_effect(
        &self,
        expression: &Expr,
        kind: &str,
        effect: NativeEffect,
    ) -> Result<Effects, String> {
        let span = self.span(expression);
        let Some(node) = self.index.node(kind, span)? else {
            return Ok(Effects::unknown(span, effect.missing()));
        };
        Ok(effect.inspect(node, span))
    }

    fn assignment<'a>(&self, site: Assignment<'a>) -> Result<ExpressionPlan<'a>, String> {
        let Assignment {
            expression,
            right,
            operation,
            ..
        } = site;
        if self.result.sites.len() >= 4096 {
            return Ok(ExpressionPlan::effects(Effects::exhausted(
                self.span(expression),
                "Write-site budget exhausted",
            )));
        }
        let kind = if operation == Operation::Assignment {
            "expr_assign"
        } else {
            "expr_assign_op"
        };
        let Some(node) = self.index.node(kind, self.span(expression))? else {
            return Ok(ExpressionPlan::effects(Effects::unknown(
                self.span(expression),
                "Assignment lacks original native correspondence",
            )));
        };

        self.check_assignment(node, &site)?;
        let effects = self.assignment_effects(&site, node)?;
        Ok(ExpressionPlan {
            effects,
            ..ExpressionPlan::children(vec![right])
        })
    }

    fn assignment_effects(&self, site: &Assignment<'_>, node: &HirNode) -> Result<Effects, String> {
        let Assignment {
            expression,
            left,
            operation,
            ..
        } = *site;
        if node.children[0].ty.is_none()
            || (operation == Operation::CompoundAssignment
                && (node.dispatch.is_none() || node.children[1].ty.is_none()))
        {
            return Ok(Effects::unknown(
                self.span(left),
                "Native assignment target type or compound dispatch was not captured",
            ));
        }

        let root = self.target(left)?;
        let builtin = operation == Operation::Assignment || compound_builtin(node, expression);
        let effects = if let Some(root) = root.filter(|_| builtin) {
            Effects {
                sites: vec![WriteSite {
                    span: self.span(expression),
                    caller_visible: root.provenance == Provenance::FormalBorrowed,
                    root,
                    operation,
                }],
                ..Effects::default()
            }
        } else {
            Effects::unsupported(
                self.span(left),
                "Assignment target or builtin compound operation is unproven",
            )
        };
        Ok(effects)
    }

    fn check_assignment(&self, node: &HirNode, site: &Assignment<'_>) -> Result<(), String> {
        if node.children.len() != 2
            || node.children[0].span != self.span(site.left)
            || node.children[1].span != self.span(site.right)
        {
            return Err(
                "Mutation assignment native operand roster contradicts original syntax".into(),
            );
        }
        Ok(())
    }

    fn pointee_path<'a>(
        &self,
        unary: &'a syn::ExprUnary,
    ) -> Result<Option<&'a syn::ExprPath>, String> {
        let Expr::Path(path) = &*unary.expr else {
            return Ok(None);
        };
        let Some(node) = self.index.node("expr_unary", self.span(unary))? else {
            return Ok(None);
        };
        if node.children.len() != 1 || node.children[0].span != self.span(path) {
            return Err("Mutation dereference operand roster contradicts original syntax".into());
        }

        let known = native::scalar(node)
            && node
                .dispatch
                .as_ref()
                .is_some_and(|dispatch| dispatch.kind == "builtin" && dispatch.op == "*");
        Ok(known.then_some(path))
    }

    fn target(&self, expression: &Expr) -> Result<Option<Root>, String> {
        let (path, borrowed) = match expression {
            Expr::Path(path) => (path, false),
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                let Some(path) = self.pointee_path(unary)? else {
                    return Ok(None);
                };
                (path, true)
            }
            _ => return Ok(None),
        };
        let Some(name) = plain_path(path) else {
            return Ok(None);
        };

        let Some(binding) = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&name))
            .and_then(Option::as_ref)
        else {
            return Ok(None);
        };
        if borrowed != (binding.root.provenance == Provenance::FormalBorrowed)
            || (!borrowed && !binding.mutable)
        {
            return Ok(None);
        }

        let Some(node) = self.index.node("expr_path", self.span(path))? else {
            return Ok(None);
        };
        if !node.res.as_ref().is_some_and(|res| {
            res.kind == "local" && res.target.as_ref() == Some(&binding.root.native)
        }) {
            return Err("Mutation target resolution contradicts its lexical declaration".into());
        }

        if !borrowed && !native::scalar(node) {
            return Ok(None);
        }
        Ok(Some(binding.root.clone()))
    }
}

impl Scan<'_> {
    fn use_boundary(&mut self, span: ByteSpan, reason: &str) {
        if let Some(uses) = &mut self.uses {
            uses.boundary(span, super::BoundaryAvailability::Unknown, reason);
        }
    }

    fn use_formal(&mut self, pattern: &Pat, ty: &Type) -> Result<(), String> {
        match checked_use_formal(&self.scopes, pattern, ty) {
            UseCheck::Known(binding) => self
                .uses
                .as_mut()
                .unwrap()
                .formal(&binding.root, binding.mutable),
            UseCheck::Unavailable(reason) => {
                self.use_boundary(self.span(pattern), reason);
                Ok(())
            }
        }
    }

    fn path_use(&mut self, path: &syn::ExprPath, role: super::UseRole) -> Result<(), String> {
        let context = UseContext {
            source: self.source,
            index: &self.index,
            scopes: &self.scopes,
        };
        match context.path(path)? {
            UseCheck::Known((node, binding)) => {
                self.uses
                    .as_mut()
                    .unwrap()
                    .occurrence(node, &binding.root, role)
            }
            UseCheck::Unavailable(reason) => {
                self.use_boundary(self.span(path), reason);
                Ok(())
            }
        }
    }

    fn expression_use(&mut self, expression: &Expr, mode: UseMode) -> Result<(), String> {
        let plan = match mode {
            UseMode::Expression => self.expression_use_plan(expression),
            UseMode::Operand => self.operand_use_plan(expression),
        };
        self.apply_use_plan(plan)
    }

    fn expression_use_plan<'a>(&self, expression: &'a Expr) -> UsePlan<'a> {
        if !crate::conditional_source::expression_attributes(expression).is_empty() {
            return UsePlan::boundary(
                self.span(expression),
                "Expression attributes may hide original formal uses",
            );
        }
        match expression {
            Expr::Path(path) => UsePlan::Leaf(UseLeaf::Path(path, super::UseRole::Read)),
            Expr::Assign(assign) => UsePlan::Wrapper(UseWrapper::Assignment(
                Assignment::plain(expression, assign),
                false,
            )),
            Expr::Binary(binary) if compound(&binary.op) => UsePlan::Wrapper(
                UseWrapper::Assignment(Assignment::compound(expression, binary), false),
            ),
            Expr::Reference(reference) => UsePlan::Wrapper(UseWrapper::Borrow(reference)),
            Expr::Return(returned) => UsePlan::Wrapper(UseWrapper::Return(returned)),
            Expr::Binary(_) | Expr::Unary(_) | Expr::Lit(_) | Expr::Paren(_) | Expr::Group(_) => {
                UsePlan::Leaf(UseLeaf::Ignored)
            }
            Expr::Block(block) if block.label.is_none() && block.attrs.is_empty() => {
                UsePlan::Leaf(UseLeaf::Ignored)
            }
            _ => UsePlan::boundary(
                self.span(expression),
                "Opaque original expression may hide formal uses",
            ),
        }
    }

    fn apply_use_plan(&mut self, plan: UsePlan<'_>) -> Result<(), String> {
        match plan {
            UsePlan::Leaf(leaf) => self.apply_use_leaf(leaf),
            UsePlan::Wrapper(wrapper) => self.apply_use_wrapper(wrapper),
            UsePlan::Evaluation(evaluation) => self.apply_use_evaluation(evaluation),
        }
    }

    fn apply_use_leaf(&mut self, leaf: UseLeaf<'_>) -> Result<(), String> {
        match leaf {
            UseLeaf::Path(path, role) => self.path_use(path, role),
            UseLeaf::Boundary(span, reason) => {
                self.use_boundary(span, reason);
                Ok(())
            }
            UseLeaf::Ignored => Ok(()),
        }
    }

    fn apply_use_wrapper(&mut self, wrapper: UseWrapper<'_>) -> Result<(), String> {
        match wrapper {
            UseWrapper::Assignment(site, read_rhs) => self.assignment_role(site, read_rhs),
            UseWrapper::Borrow(reference) => self.borrow_use(reference),
            UseWrapper::Return(returned) => self.return_use(returned),
        }
    }

    fn apply_use_evaluation(&mut self, evaluation: UseEvaluation<'_>) -> Result<(), String> {
        match evaluation {
            UseEvaluation::Unary(expression, child) => {
                self.operand_children(expression, "expr_unary", &[child])
            }
            UseEvaluation::Binary(expression, left, right) => {
                self.operand_children(expression, "expr_binary", &[left, right])
            }
            UseEvaluation::Literal(expression) => self.literal_use(expression),
        }
    }

    fn assignment_use(&mut self, site: Assignment<'_>) -> Result<(), String> {
        let (kind, role) = match site.operation {
            Operation::Assignment => ("expr_assign", super::UseRole::Write),
            Operation::CompoundAssignment => ("expr_assign_op", super::UseRole::ReadWrite),
        };
        if self.checked_use_children(site.expression, kind, &[site.left, site.right])? {
            self.place_use(site.left, role)?;
        }
        Ok(())
    }

    fn place_use(&mut self, expression: &Expr, role: super::UseRole) -> Result<(), String> {
        match transparent(expression) {
            Expr::Path(path) => self.path_use(path, role),
            _ => {
                self.use_boundary(
                    self.span(expression),
                    "Projection or indirect place may hide formal uses",
                );
                Ok(())
            }
        }
    }

    fn checked_use_children(
        &mut self,
        expression: &impl Spanned,
        kind: &str,
        children: &[&Expr],
    ) -> Result<bool, String> {
        let span = self.span(expression);
        let Some(node) = self.index.node(kind, span)? else {
            self.use_boundary(
                span,
                "Original formal-use wrapper has no native correspondence",
            );
            return Ok(false);
        };
        if node.children.len() != children.len()
            || node
                .children
                .iter()
                .zip(children)
                .any(|(node, source)| node.span != self.span(transparent(source)))
        {
            return Err(
                "Formal-use wrapper native child roster contradicts original syntax".into(),
            );
        }
        Ok(true)
    }

    fn borrow_use(&mut self, reference: &syn::ExprReference) -> Result<(), String> {
        let role = if reference.mutability.is_some() {
            super::UseRole::MutableBorrow
        } else {
            super::UseRole::SharedBorrow
        };
        if self.checked_use_children(reference, "expr_addr_of", &[&reference.expr])? {
            self.place_use(&reference.expr, role)?;
        }
        Ok(())
    }

    fn return_use(&mut self, returned: &syn::ExprReturn) -> Result<(), String> {
        let children: Vec<&Expr> = returned
            .expr
            .iter()
            .map(|expression| &**expression)
            .collect();
        if self.checked_use_children(returned, "expr_ret", &children)?
            && let Some(value) = &returned.expr
        {
            self.expression_use(value, UseMode::Operand)?;
        }
        Ok(())
    }

    fn operand_use_plan<'a>(&self, expression: &'a Expr) -> UsePlan<'a> {
        if !crate::conditional_source::expression_attributes(expression).is_empty() {
            return UsePlan::boundary(
                self.span(expression),
                "Attributed evaluation may hide formal uses",
            );
        }
        let expression = transparent(expression);
        match expression {
            Expr::Path(path) => UsePlan::Leaf(UseLeaf::Path(path, super::UseRole::Read)),
            Expr::Assign(assign) => UsePlan::Wrapper(UseWrapper::Assignment(
                Assignment::plain(expression, assign),
                true,
            )),
            Expr::Binary(binary) if compound(&binary.op) => UsePlan::Wrapper(
                UseWrapper::Assignment(Assignment::compound(expression, binary), true),
            ),
            Expr::Binary(binary) => UsePlan::Evaluation(UseEvaluation::Binary(
                expression,
                &binary.left,
                &binary.right,
            )),
            Expr::Unary(unary) => {
                UsePlan::Evaluation(UseEvaluation::Unary(expression, &unary.expr))
            }
            Expr::Reference(reference) => UsePlan::Wrapper(UseWrapper::Borrow(reference)),
            Expr::Lit(_) => UsePlan::Evaluation(UseEvaluation::Literal(expression)),
            _ => UsePlan::boundary(
                self.span(expression),
                "Unexamined evaluation may hide original formal uses",
            ),
        }
    }

    fn literal_use(&mut self, expression: &Expr) -> Result<(), String> {
        if self
            .index
            .node("expr_lit", self.span(expression))?
            .is_none()
        {
            self.use_boundary(
                self.span(expression),
                "Original literal evaluation was not captured",
            );
        }
        Ok(())
    }

    fn assignment_role(&mut self, site: Assignment<'_>, read_rhs: bool) -> Result<(), String> {
        let right = site.right;
        self.assignment_use(site)?;
        if read_rhs {
            self.expression_use(right, UseMode::Operand)?;
        }
        Ok(())
    }

    fn operand_children(
        &mut self,
        expression: &Expr,
        kind: &str,
        children: &[&Expr],
    ) -> Result<(), String> {
        if self.checked_use_children(expression, kind, children)? {
            for child in children {
                self.expression_use(child, UseMode::Operand)?;
            }
        }
        Ok(())
    }

    fn call_uses(&mut self, syntax: &syn::ExprCall, call: &super::Call) -> Result<(), String> {
        let call_index = self.calls.as_ref().unwrap().len();
        if !matches!(&*syntax.func, Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none() && path.path.segments.iter().all(|segment| segment.arguments.is_empty()))
        {
            self.use_boundary(
                self.span(&syntax.func),
                "Unexamined call target evaluation may hide formal uses",
            );
        }
        for (argument_index, (syntax, argument)) in
            syntax.args.iter().zip(&call.arguments).enumerate()
        {
            let unchanged = matches!(
                argument.unchanged_formal,
                super::Availability::Known(Some(_))
            );
            self.call_argument_use(
                syntax,
                super::UseRole::CallArgument {
                    call_index,
                    argument_index,
                    unchanged,
                },
            )?;
        }
        Ok(())
    }
    fn call_argument_use(&mut self, syntax: &Expr, role: super::UseRole) -> Result<(), String> {
        if !crate::conditional_source::expression_attributes(syntax).is_empty() {
            self.use_boundary(
                self.span(syntax),
                "Attributed call argument may hide formal uses",
            );
            return Ok(());
        }
        match transparent(syntax) {
            Expr::Path(path) => self.path_use(path, role),
            _ => self.expression_use(syntax, UseMode::Operand),
        }
    }
}

enum ConditionalPiece<'a> {
    Expression(&'a Expr),
    Block(&'a syn::Block),
}

fn plain_condition(expression: &Expr) -> bool {
    let mut condition = ConditionShape { plain: true };
    condition.visit_expr(expression);
    condition.plain
}

struct ConditionShape {
    plain: bool,
}

impl<'ast> Visit<'ast> for ConditionShape {
    fn visit_expr(&mut self, expression: &'ast Expr) {
        self.plain &= crate::conditional_source::expression_attributes(expression).is_empty()
            && matches!(
                expression,
                Expr::Path(_)
                    | Expr::Lit(_)
                    | Expr::Binary(_)
                    | Expr::Unary(_)
                    | Expr::Paren(_)
                    | Expr::Group(_)
                    | Expr::Call(_)
            );
        if self.plain {
            syn::visit::visit_expr(self, expression);
        }
    }
}

// Pure use plans distinguish ordinary scan children from operands evaluated only here.
enum UseMode {
    Expression,
    Operand,
}

enum UsePlan<'a> {
    Leaf(UseLeaf<'a>),
    Wrapper(UseWrapper<'a>),
    Evaluation(UseEvaluation<'a>),
}

enum UseLeaf<'a> {
    Path(&'a syn::ExprPath, super::UseRole),
    Boundary(ByteSpan, &'static str),
    Ignored,
}

enum UseWrapper<'a> {
    Assignment(Assignment<'a>, bool),
    Borrow(&'a syn::ExprReference),
    Return(&'a syn::ExprReturn),
}

enum UseEvaluation<'a> {
    Unary(&'a Expr, &'a Expr),
    Binary(&'a Expr, &'a Expr, &'a Expr),
    Literal(&'a Expr),
}

impl UsePlan<'_> {
    fn boundary(span: ByteSpan, reason: &'static str) -> Self {
        Self::Leaf(UseLeaf::Boundary(span, reason))
    }
}

enum UseCheck<T> {
    Known(T),
    Unavailable(&'static str),
}

type LexicalScopes = [BTreeMap<String, Option<Binding>>];

fn checked_use_formal<'a>(
    scopes: &'a LexicalScopes,
    pattern: &Pat,
    ty: &Type,
) -> UseCheck<&'a Binding> {
    let Pat::Ident(ident) = pattern else {
        return UseCheck::Unavailable("Formal use declaration requires a direct owned-u32 binding");
    };
    if !crate::source_evidence::plain_type_path(ty, "u32") {
        return UseCheck::Unavailable("Formal use inventory does not support this formal family");
    }
    match scopes
        .last()
        .and_then(|scope| scope.get(&ident.ident.to_string()))
        .and_then(Option::as_ref)
    {
        Some(binding) => UseCheck::Known(binding),
        None => UseCheck::Unavailable("Original formal binding evidence is unavailable"),
    }
}

struct UseContext<'a, 'b> {
    source: &'a str,
    index: &'b native::Index<'a>,
    scopes: &'b LexicalScopes,
}

impl<'a, 'b> UseContext<'a, 'b> {
    fn path(&self, path: &syn::ExprPath) -> Result<UseCheck<(&'a HirNode, &'b Binding)>, String> {
        let span = crate::source_evidence::original_span(self.source, path.span());
        let Some(name) = plain_path(path) else {
            return Ok(UseCheck::Unavailable(
                "Formal use path is outside direct lexical bindings",
            ));
        };
        let Some(node) = self.index.node("expr_path", span)? else {
            return Ok(UseCheck::Unavailable(
                "Original formal-use native path is unavailable",
            ));
        };

        let binding = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&name))
            .and_then(Option::as_ref);
        let Some(binding) = binding else {
            return Ok(UseCheck::Unavailable(
                "Formal use has no checked lexical root",
            ));
        };
        if binding.root.provenance == Provenance::FormalBorrowed {
            return Ok(UseCheck::Unavailable(
                "Borrowed formal values require another use-inventory family",
            ));
        }
        if !check_call_argument(self.source, path, node, binding)? {
            return Ok(UseCheck::Unavailable(
                "Original formal-use path type or resolution is unavailable",
            ));
        }
        Ok(UseCheck::Known((node, binding)))
    }
}

fn check_call_argument(
    source: &str,
    path: &syn::ExprPath,
    node: &HirNode,
    binding: &Binding,
) -> Result<bool, String> {
    if node.kind != "expr_path"
        || node.span != crate::source_evidence::original_span(source, path.span())
    {
        return Err("Call argument native path differs from original syntax".into());
    }
    let Some(ty) = &node.ty else {
        return Ok(false);
    };

    if binding.root.provenance != Provenance::FormalBorrowed
        && ty.normalized.as_deref() != Some("u32")
    {
        return Err("Call argument type contradicts its lexical scalar binding".into());
    }
    if binding.root.provenance == Provenance::FormalBorrowed && ty.normalized.is_some() {
        return Err("Call argument scalar type contradicts its mutable reference binding".into());
    }

    let Some(resolution) = &node.res else {
        return Ok(false);
    };
    if resolution.kind != "local" || resolution.target.as_ref() != Some(&binding.root.native) {
        return Err("Call argument resolution contradicts its lexical declaration".into());
    }

    Ok(true)
}

fn plain_path(path: &syn::ExprPath) -> Option<String> {
    (path.attrs.is_empty()
        && path.qself.is_none()
        && path.path.leading_colon.is_none()
        && path.path.segments.len() == 1
        && path.path.segments[0].arguments.is_empty())
    .then(|| path.path.segments[0].ident.to_string())
}

fn benign_formal(ty: &Type) -> bool {
    matches!(ty, Type::BareFn(_))
}

fn benign_node(node: &HirNode) -> bool {
    node.expansion.is_none()
        && node
            .ty
            .as_ref()
            .is_some_and(|ty| matches!(ty.normalized.as_deref(), Some("u32" | "bool" | "unit")))
}

fn compound(op: &syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

fn compound_builtin(node: &HirNode, expression: &Expr) -> bool {
    let Expr::Binary(binary) = expression else {
        return false;
    };
    let expected = match binary.op {
        syn::BinOp::AddAssign(_) => "+=",
        syn::BinOp::SubAssign(_) => "-=",
        syn::BinOp::MulAssign(_) => "*=",
        syn::BinOp::DivAssign(_) => "/=",
        syn::BinOp::RemAssign(_) => "%=",
        syn::BinOp::BitXorAssign(_) => "^=",
        syn::BinOp::BitAndAssign(_) => "&=",
        syn::BinOp::BitOrAssign(_) => "|=",
        syn::BinOp::ShlAssign(_) => "<<=",
        syn::BinOp::ShrAssign(_) => ">>=",
        _ => return false,
    };
    node.dispatch.as_ref().is_some_and(|dispatch| {
        dispatch.kind == "builtin" && dispatch.op == expected && dispatch.result_ty == "unit"
    }) && node.children.iter().all(native::scalar)
}

#[derive(Default)]
struct Poison {
    names: BTreeSet<String>,
    opaque_references: bool,
}

impl Poison {
    fn place(&mut self, expression: &Expr) {
        match transparent(expression) {
            Expr::Path(path) => {
                if let Some(name) = plain_path(path) {
                    self.names.insert(name);
                } else {
                    self.opaque_references = true;
                }
            }
            Expr::Unary(unary)
                if matches!(unary.op, syn::UnOp::Deref(_))
                    && matches!(transparent(&unary.expr), Expr::Path(path) if plain_path(path).is_some()) =>
                {}
            _ => self.opaque_references = true,
        }
    }
}

fn transparent(mut expression: &Expr) -> &Expr {
    loop {
        expression = match expression {
            Expr::Paren(paren) => &paren.expr,
            Expr::Group(group) => &group.expr,
            _ => return expression,
        };
    }
}

#[derive(Default)]
struct PatternNames {
    names: Vec<String>,
    opaque: bool,
}

impl<'ast> Visit<'ast> for PatternNames {
    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.opaque = true;
    }

    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.names.push(pattern.ident.to_string());
        syn::visit::visit_pat_ident(self, pattern);
    }
}

impl<'ast> Visit<'ast> for Poison {
    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {
        self.opaque_references = true;
    }

    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        if pattern.by_ref.is_some() && pattern.mutability.is_some() {
            self.opaque_references = true;
        }
        syn::visit::visit_pat_ident(self, pattern);
    }

    fn visit_expr_raw_addr(&mut self, _: &'ast syn::ExprRawAddr) {
        self.opaque_references = true;
    }

    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.opaque_references = true;
    }

    fn visit_expr_unsafe(&mut self, _: &'ast syn::ExprUnsafe) {
        self.opaque_references = true;
    }

    fn visit_expr_assign(&mut self, assignment: &'ast syn::ExprAssign) {
        self.place(&assignment.left);
        syn::visit::visit_expr_assign(self, assignment);
    }

    fn visit_expr_binary(&mut self, binary: &'ast syn::ExprBinary) {
        if compound(&binary.op) {
            self.place(&binary.left);
        }
        syn::visit::visit_expr_binary(self, binary);
    }

    fn visit_expr_reference(&mut self, reference: &'ast syn::ExprReference) {
        if reference.mutability.is_some() {
            self.place(&reference.expr);
        }
        syn::visit::visit_expr_reference(self, reference);
    }
}

fn value_effects(node: &HirNode, span: ByteSpan) -> Effects {
    if node.ty.is_none() {
        return Effects::unknown(span, "Native value type was not captured");
    }

    if benign_node(node) {
        Effects::default()
    } else {
        Effects::unsupported(span, "Unsupported primitive value")
    }
}

fn primitive_effects(node: &HirNode, span: ByteSpan) -> Effects {
    if node.ty.is_none() || node.dispatch.is_none() {
        return Effects::unknown(span, "Native operation type or dispatch was not captured");
    }
    let known = benign_node(node)
        && node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.kind == "builtin")
        && node.children.iter().all(benign_node);

    if known {
        Effects::default()
    } else {
        Effects::unsupported(span, "Unproven builtin primitive operation")
    }
}
