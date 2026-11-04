//! The body tree: the document's source nodes as the driver's construct records, with
//! the driver's tags, child order and extraction-profile gates.

use super::{Context, hir_ref, symbol_name};
use crate::frontend::{ByteSpan, ExpansionSite, HirNode, ResolvedTy};
use rstyle_core::cpg::model::{Node, NodeKind, Origin, TypeKind};

type Built = Result<HirNode, String>;

impl<'a> Context<'_, 'a> {
    pub(super) fn tree(&self, node: &'a Node) -> Built {
        self.expression(node)
    }

    fn expression(&self, node: &'a Node) -> Built {
        let tag = self.tag(node);
        let ty = match tag {
            "expr_ret" => Some(
                self.reader
                    .resolved(&self.body.function.signature.result, String::new())?,
            ),
            _ => self.ty(node)?,
        };
        // Children follow the HIR construct, whatever tag the profile gives it.
        let children = self.expression_children(node, &node.original_kind)?;
        let base = self.node(node, tag)?;

        Ok(HirNode {
            ty,
            // Operator evidence follows the HIR construct, whatever tag the profile gives it.
            dispatch: self.dispatch(node, &node.original_kind)?,
            res: self.resolution(node, tag),
            lit: (tag == "expr_lit").then(|| base.snippet.clone()),
            is_unsafe_block: tag == "expr_block"
                && node
                    .modifiers
                    .iter()
                    .any(|m| m == "unsafe" || m == "rust:compiler-unsafe"),
            children,
            ..base
        })
    }

    /// The driver's construct tag: the HIR's own, refined by the profile.
    fn tag(&self, node: &'a Node) -> &'a str {
        let supports = |capability: &str| self.reader.profile.supports(capability);
        let constructed = match node.original_kind.as_str() {
            "expr_call" if supports("rust.tuple-struct-values") => self
                .body
                .child(node, 0)
                .is_ok_and(|callee| self.constructs(callee, "Ctor(Struct, Fn)")),
            "expr_path" if supports("rust.unit-struct-values") => {
                self.constructs(node, "Ctor(Struct, Const)")
                    && node
                        .ty
                        .as_deref()
                        .and_then(|ty| self.reader.ty(ty).ok())
                        .is_some_and(|ty| matches!(ty.kind, TypeKind::Record | TypeKind::Sum))
            }
            _ => false,
        };
        match constructed {
            true => "expr_struct",
            false => node.original_kind.as_str(),
        }
    }

    /// Whether a path names a struct constructor of the compiler's `kind`.
    fn constructs(&self, path: &Node, kind: &str) -> bool {
        let Some(target) = self
            .body
            .reference(&path.id)
            .and_then(|reference| reference.target.as_ref())
        else {
            return false;
        };
        target.kind == rstyle_core::cpg::model::TargetKind::Constructor
            && target
                .id
                .as_deref()
                .and_then(|id| self.reader.tables.definition(id).ok())
                .and_then(|definition| definition.extensions.get("rust:compiler-identity"))
                .and_then(|identity| identity.get("def_kind")?.as_str())
                == Some(kind)
    }

    fn expression_children(&self, node: &'a Node, tag: &str) -> Result<Vec<HirNode>, String> {
        let children = self.body.source_children(node)?;
        let supports = |capability: &str| self.reader.profile.supports(capability);
        let selected: Vec<&'a Node> = match tag {
            "expr_block" => return self.block_children(&children),
            "expr_loop" => return self.loop_children(&children),
            "expr_binary" | "expr_assign" | "expr_assign_op" | "expr_index" => {
                children.into_iter().take(2).collect()
            }
            "expr_unary" | "expr_cast" | "expr_addr_of" => children.into_iter().take(1).collect(),
            "expr_if" | "expr_tup" | "expr_ret" | "expr_break" | "expr_method_call"
            | "expr_while" => children,
            "expr_let" if supports("rust.scalar-option-if-let") => {
                return self.let_children(&children);
            }
            "expr_match" if node.kind == NodeKind::Match => return self.match_children(&children),
            "expr_call" => children.into_iter().skip(1).collect(),
            "expr_field" if supports("rust.verified-helper-calls") => {
                children.into_iter().take(1).collect()
            }
            "expr_struct" | "expr_array" | "expr_repeat" => {
                return self.aggregate_children(node, tag, &children);
            }
            _ => Vec::new(),
        };
        selected
            .into_iter()
            .map(|child| self.expression(child))
            .collect()
    }

    fn block_children(&self, children: &[&'a Node]) -> Result<Vec<HirNode>, String> {
        children.iter().map(|child| self.statement(child)).collect()
    }

    /// An unlabeled `loop`'s block, typed as the unit its body evaluates to.
    fn loop_children(&self, children: &[&'a Node]) -> Result<Vec<HirNode>, String> {
        let Some(block) = children.first() else {
            return Ok(Vec::new());
        };
        let unit = self.reader.unit_type()?;
        let node = self.node(block, "expr_block")?;
        Ok(vec![HirNode {
            children: self.block_children(&self.body.source_children(block)?)?,
            ty: Some(unit),
            ..node
        }])
    }

    fn let_children(&self, children: &[&'a Node]) -> Result<Vec<HirNode>, String> {
        match children {
            [pattern, init, ..] => Ok(vec![self.pattern(pattern)?, self.expression(init)?]),
            _ => Ok(Vec::new()),
        }
    }

    fn match_children(&self, children: &[&'a Node]) -> Result<Vec<HirNode>, String> {
        let Some((scrutinee, arms)) = children.split_first() else {
            return Ok(Vec::new());
        };
        std::iter::once(self.expression(scrutinee))
            .chain(arms.iter().map(|arm| self.arm(arm)))
            .collect()
    }

    fn arm(&self, arm: &'a Node) -> Built {
        let mut children = Vec::new();
        for part in self.body.source_children(arm)? {
            children.push(match part.kind {
                NodeKind::MatchGuard => self.guard(part)?,
                _ if children.is_empty() => self.pattern(part)?,
                _ => self.expression(part)?,
            });
        }
        Ok(HirNode {
            children,
            ..self.node(arm, "match_arm")?
        })
    }

    /// A guard is recorded at its condition, as the HIR records it.
    fn guard(&self, guard: &'a Node) -> Built {
        let condition = self.body.child(guard, 0)?;
        let node = self.node(guard, "match_guard")?;
        Ok(HirNode {
            hir: hir_ref(&condition.id),
            children: vec![self.expression(condition)?],
            ..node
        })
    }

    fn aggregate_children(
        &self,
        node: &'a Node,
        tag: &str,
        children: &[&'a Node],
    ) -> Result<Vec<HirNode>, String> {
        let supports = |capability: &str| self.reader.profile.supports(capability);
        let selected: &[&'a Node] = match tag {
            "expr_struct" if supports("rust.named-struct-values") => {
                let fields = node.members.as_ref().map_or(children.len(), Vec::len);
                if children.len() > fields && !supports("rust.struct-update-values") {
                    return Ok(Vec::new());
                }
                children
            }
            "expr_array" if supports("rust.aggregate-literal-values") => children,
            "expr_repeat" if supports("rust.repeat-array-values") => {
                &children[..children.len().min(1)]
            }
            _ => &[],
        };
        selected
            .iter()
            .map(|child| self.expression(child))
            .collect()
    }

    fn statement(&self, node: &'a Node) -> Built {
        match node.original_kind.as_str() {
            "stmt_let" => self.declaration(node),
            tag @ ("stmt_semi" | "stmt_expr") => {
                let inner = self.body.child(node, 0)?;
                Ok(HirNode {
                    children: vec![self.expression(inner)?],
                    ..self.node(node, tag)?
                })
            }
            "stmt_item" => self.node(node, "stmt_item"),
            _ => self.expression(node),
        }
    }

    fn declaration(&self, local: &'a Node) -> Built {
        let mut children = Vec::new();
        for (index, part) in self.body.source_children(local)?.into_iter().enumerate() {
            children.push(match part.kind {
                _ if index == 0 => self.pattern(part)?,
                NodeKind::TypeAnnotation => self.annotation(local, part)?,
                NodeKind::Block
                    if index + 1 == local.children.len() && self.is_else(local, index) =>
                {
                    self.node(part, "let_else")?
                }
                _ => self.expression(part)?,
            });
        }
        Ok(HirNode {
            children,
            ..self.node(local, "stmt_let")?
        })
    }

    /// Whether the declaration's part at `index` is its `else` block.
    fn is_else(&self, local: &Node, index: usize) -> bool {
        let parts = local.children.len();
        let annotated = self.body.children(local).is_ok_and(|parts| {
            parts
                .iter()
                .any(|part| part.kind == NodeKind::TypeAnnotation)
        });
        index == parts - 1 && parts == 3 + usize::from(annotated)
    }

    fn annotation(&self, local: &'a Node, annotation: &'a Node) -> Built {
        let pattern = self.body.child(local, 0)?;
        let node = self.node(annotation, "ty_annotation")?;
        let ty = pattern
            .ty
            .as_deref()
            .map(|ty| self.reader.resolved(ty, node.snippet.clone()))
            .transpose()?;
        Ok(HirNode { ty, ..node })
    }

    pub(super) fn pattern(&self, node: &'a Node) -> Built {
        let tag = match node.original_kind.as_str() {
            tag @ ("pat_binding" | "pat_binding_pattern" | "pat_wild") => tag,
            "pat_expr" if node.kind == NodeKind::PatternLiteral => "pat_literal",
            _ => "pat_unsupported",
        };
        let symbol = self.declared(node);
        let mut base = self.node(node, tag)?;
        // A plain binding is located at its identifier, without `mut`.
        if tag == "pat_binding"
            && let Some(declared) =
                symbol.and_then(|(symbol, _)| self.body.symbol(symbol).ok()?.declared.as_ref())
        {
            base.span = self.span(declared);
            base.snippet = self.snippet(base.span);
        }
        Ok(HirNode {
            ty: self.ty(node)?,
            binding: symbol.map(|(symbol, _)| symbol_name(symbol)),
            binding_mut: symbol.map(|(_, mutable)| mutable),
            lit: (tag == "pat_literal").then(|| base.snippet.clone()),
            ..base
        })
    }

    /// The record every construct starts from: identity, tag, written span and source.
    /// A construct a macro or desugaring generated is located at its outermost call site.
    fn node(&self, node: &'a Node, tag: &str) -> Built {
        let written = self.written(node);
        let expansion = self.expansion(node, written);
        let span = expansion.as_ref().map_or(written, |site| site.callsite);
        // Only an expression records where it was expanded; any construct is located there.
        let expansion = expansion.filter(|_| tag.starts_with("expr_"));
        Ok(HirNode {
            hir: hir_ref(&node.id),
            kind: tag.into(),
            span,
            snippet: self.snippet(span),
            ty: None,
            res: None,
            dispatch: None,
            expansion,
            binding: None,
            binding_mut: None,
            lit: None,
            param_index: None,
            is_unsafe_block: false,
            children: Vec::new(),
        })
    }

    /// The node's span as written, with the parentheses the document's span excludes.
    fn written(&self, node: &Node) -> ByteSpan {
        let Some(span) = node.span.as_ref() else {
            return ByteSpan { start: 0, end: 0 };
        };
        let mut span = self.span(span);
        let bytes = self.source.as_bytes();
        // A span outside the function's own bytes has no parentheses of its own to restore.
        if span.end as usize > bytes.len() {
            return span;
        }
        for _ in 0..node.parentheses.unwrap_or(0) {
            let mut start = span.start as usize;
            let mut end = span.end as usize;
            while start > 0 && bytes[start - 1].is_ascii_whitespace() {
                start -= 1;
            }
            while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                end += 1;
            }
            span = ByteSpan {
                start: start.saturating_sub(1) as u32,
                end: (end + 1).min(bytes.len()) as u32,
            };
        }
        span
    }

    fn ty(&self, node: &Node) -> Result<Option<ResolvedTy>, String> {
        node.ty
            .as_deref()
            .map(|ty| self.reader.resolved(ty, String::new()))
            .transpose()
    }

    /// The symbol a binding pattern declares, with its mutability.
    fn declared(&self, node: &Node) -> Option<(&'a str, bool)> {
        self.body
            .function
            .symbols
            .iter()
            .find(|symbol| symbol.node.as_deref() == Some(node.id.as_str()))
            .map(|symbol| (symbol.id.as_str(), symbol.mutable))
    }

    /// The call site of a construct a macro or desugaring expanded. A restored `for` is
    /// the desugaring the compiler expanded at its own span.
    fn expansion(&self, node: &Node, span: ByteSpan) -> Option<ExpansionSite> {
        let restored = node.kind == NodeKind::For;
        if node.provenance.origin == Origin::Source && !restored {
            return None;
        }
        let callsite = node
            .provenance
            .call_site
            .as_ref()
            .map_or(span, |site| self.span(site));
        Some(ExpansionSite {
            callsite,
            snippet: self.snippet(callsite),
        })
    }
}

/// Operator evidence: the driver's rendering, operand and result types, and dispatch.
mod dispatch {
    use super::super::Context;
    use crate::frontend::Dispatch;
    use rstyle_core::cpg::model::{Dispatch as Kind, Node, NodeKind};

    impl<'a> Context<'_, 'a> {
        pub(in super::super) fn dispatch(
            &self,
            node: &'a Node,
            tag: &str,
        ) -> Result<Option<Dispatch>, String> {
            let children = self.body.source_children(node)?;
            let (op, operands): (String, &[&'a Node]) = match tag {
                "expr_binary" => (rendering(node, "")?, &children[..children.len().min(2)]),
                "expr_assign_op" => (rendering(node, "=")?, &children[..children.len().min(2)]),
                "expr_unary" => (unary(node)?, &children[..children.len().min(1)]),
                "expr_index" => ("[]".into(), &children[..children.len().min(2)]),
                "expr_cast" => ("as".into(), &children[..children.len().min(1)]),
                "expr_method_call" => match self.method_path(node) {
                    Some(path) => (path, &children[..]),
                    None => return Ok(None),
                },
                "expr_call" => match self.call_path(node) {
                    Some(path) => (path, &children[children.len().min(1)..]),
                    None => return Ok(None),
                },
                _ => return Ok(None),
            };
            let operand_tys = operands
                .iter()
                .map(|operand| self.token_type(operand))
                .collect::<Result<_, _>>()?;
            let overloaded = self
                .body
                .reference(&node.id)
                .is_some_and(|reference| reference.dispatch == Some(Kind::Overloaded));
            let kind = match tag {
                "expr_call" => "resolved_direct",
                "expr_method_call" => "resolved_method",
                _ if overloaded => "overloaded",
                _ => "builtin",
            };

            Ok(Some(Dispatch {
                kind: kind.into(),
                op,
                operand_tys,
                result_ty: self.token_type(node)?,
            }))
        }

        /// The path a direct call's resolved helper callee records.
        fn call_path(&self, node: &Node) -> Option<String> {
            let callee = self.body.child(node, 0).ok()?;
            let resolution = self.direct_callee(callee)?;
            let value: serde_json::Value = serde_json::from_str(&resolution.description).ok()?;
            let path = match resolution.kind == crate::size_calls::RESOLUTION {
                true => &value["callee"]["path"],
                false => &value["path"],
            };
            Some(path.as_str()?.to_owned())
        }

        /// The method a call resolves to, as the compiler names it.
        fn method_path(&self, node: &Node) -> Option<String> {
            let reference = self.body.reference(&node.id)?;
            let id = reference
                .target
                .as_ref()
                .and_then(|target| target.id.as_deref())
                .or(reference.via.as_deref())?;
            Some(self.reader.tables.definition(id).ok()?.name.clone())
        }

        /// A type as an operator records it: the scalar token, or the compiler's spelling.
        fn token_type(&self, node: &Node) -> Result<String, String> {
            let Some(ty) = node.ty.as_deref() else {
                return Ok(String::new());
            };
            let resolved = self.reader.resolved(ty, String::new())?;
            Ok(resolved.normalized.unwrap_or(resolved.observed))
        }
    }

    /// A binary operator as Rust spells it, with `suffix` for compound assignment.
    fn rendering(node: &Node, suffix: &str) -> Result<String, String> {
        let spelling = match node.operator.as_deref() {
            Some("add") => "+",
            Some("sub") => "-",
            Some("mul") => "*",
            Some("div") => "/",
            Some("rem") => "%",
            Some("and") => "&&",
            Some("or") => "||",
            Some("bit_xor") => "^",
            Some("bit_and") => "&",
            Some("bit_or") => "|",
            Some("shl") => "<<",
            Some("shr") => ">>",
            Some("eq") => "==",
            Some("lt") => "<",
            Some("le") => "<=",
            Some("ne") => "!=",
            Some("ge") => ">=",
            Some("gt") => ">",
            other => return Err(format!("Node {} has operator {other:?}", node.id)),
        };
        debug_assert!(matches!(
            node.kind,
            NodeKind::Operator | NodeKind::CompoundAssignment
        ));
        Ok(format!("{spelling}{suffix}"))
    }

    fn unary(node: &Node) -> Result<String, String> {
        Ok(match node.operator.as_deref() {
            Some("deref") => "*",
            Some("not") => "!",
            Some("neg") => "-",
            other => return Err(format!("Node {} has operator {other:?}", node.id)),
        }
        .into())
    }
}
