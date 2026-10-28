use super::{Assessment, Execution, Operation, Provenance, Root, WriteSite, native};
use crate::{ByteSpan, HirFnFacts, HirNode};
use std::collections::{BTreeMap, BTreeSet};
use syn::{Expr, Pat, Type, spanned::Spanned, visit::Visit};

pub(super) use crate::consumer_facts::syntax_budget::within_budget;

pub(super) fn inspect(
    facts: &HirFnFacts,
    source: &str,
    result: Assessment,
) -> Result<Assessment, String> {
    let function = match parsed(source) {
        Ok(function) => function,
        Err(reason) => return Ok(Effects::unsupported(whole(source), &reason).merge(result)),
    };
    super::super::check_signature(facts, &function)?;
    let Some(index) = native::Index::new(facts)? else {
        return Ok(
            Effects::exhausted(whole(source), "Native inventory budget exhausted").merge(result),
        );
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
    };

    scan.signature(&function)?;
    scan.block(&function.block)?;
    Ok(scan.result)
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
}

impl Scan<'_> {
    fn span(&self, value: &impl Spanned) -> ByteSpan {
        crate::source_evidence::original_span(self.source, value.span())
    }

    fn boundary(&mut self, value: &impl Spanned, reason: &str) {
        self.result.boundary(self.span(value), reason);
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

        self.declarations += 1;
        self.apply(plan.effects);
        self.scopes
            .last_mut()
            .expect("root scope exists")
            .extend(plan.bindings);
        Ok(())
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
                _ => self.apply(Effects::opaque(
                    self.span(statement),
                    "Opaque original item or macro statement",
                )),
            }
        }
        Ok(())
    }

    fn local(&mut self, local: &syn::Local, scope: ByteSpan) -> Result<(), String> {
        let plan = self.local_plan(local);
        if let Some(expression) = plan.initializer {
            self.expression(expression)?;
        }
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
        let invalidate = effects.invalidate;
        let previous = std::mem::replace(&mut self.result, Assessment::empty());
        self.result = effects.merge(previous);

        if invalidate {
            self.invalidate_roots();
        }
    }

    fn expression(&mut self, expression: &Expr) -> Result<(), String> {
        let plan = self.expression_plan(expression)?;
        self.apply(plan.effects);

        if let Some(block) = plan.block {
            self.block(block)?;
        }
        for child in plan.children {
            self.expression(child)?;
        }
        Ok(())
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
                effects: self.primitive(expression, "expr_binary")?,
                ..ExpressionPlan::children(vec![&binary.left, &binary.right])
            },
            Expr::Unary(unary) => ExpressionPlan {
                effects: self.primitive(expression, "expr_unary")?,
                ..ExpressionPlan::children(vec![&unary.expr])
            },
            Expr::Lit(_) => ExpressionPlan::effects(self.value(expression, "expr_lit")?),
            Expr::Path(_) => ExpressionPlan::effects(self.value(expression, "expr_path")?),
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

    fn value(&self, expression: &Expr, kind: &str) -> Result<Effects, String> {
        let Some(node) = self.index.node(kind, self.span(expression))? else {
            return Ok(Effects::unknown(
                self.span(expression),
                "Missing original native value evidence",
            ));
        };
        if node.ty.is_none() {
            return Ok(Effects::unknown(
                self.span(expression),
                "Native value type was not captured",
            ));
        }

        Ok(if benign_node(node) {
            Effects::default()
        } else {
            Effects::unsupported(self.span(expression), "Unsupported primitive value")
        })
    }

    fn primitive(&self, expression: &Expr, kind: &str) -> Result<Effects, String> {
        let Some(node) = self.index.node(kind, self.span(expression))? else {
            return Ok(Effects::unknown(
                self.span(expression),
                "Missing original native operation evidence",
            ));
        };
        if node.ty.is_none() || node.dispatch.is_none() {
            return Ok(Effects::unknown(
                self.span(expression),
                "Native operation type or dispatch was not captured",
            ));
        }

        let known = benign_node(node)
            && node
                .dispatch
                .as_ref()
                .is_some_and(|dispatch| dispatch.kind == "builtin")
            && node.children.iter().all(benign_node);

        Ok(if known {
            Effects::default()
        } else {
            Effects::unsupported(
                self.span(expression),
                "Unproven builtin primitive operation",
            )
        })
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
