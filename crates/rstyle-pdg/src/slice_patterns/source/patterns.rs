//! Independently parsed digit ranges and the real lowered rest node.
use super::*;

impl<'a> Proof<'a, '_> {
    pub(super) fn digit(
        &self,
        syntax: &syn::Pat,
        binding: &'a HirNode,
    ) -> Result<CheckedDigit<'a>, String> {
        self.binding(syntax, binding, false)?;
        let syn::Pat::Ident(ident) = syntax else {
            unreachable!("Checked binding syntax");
        };
        let (_, subpattern) = ident
            .subpat
            .as_ref()
            .ok_or_else(|| unsupported("Missing @ range"))?;
        let syn::Pat::Range(range) = subpattern.as_ref() else {
            return Err(unsupported(
                "Prefix binding requires an inclusive byte range",
            ));
        };

        let node = &binding.children[0];
        self.source(node, range.span())?;
        let receipt = self.leaf(node, "pat_slice_range", &TypeShape::Byte)?;
        let Details::Range {
            start,
            end,
            inclusive,
        } = receipt.details
        else {
            return Err(unsupported("Missing raw range descriptor"));
        };

        if !range.attrs.is_empty()
            || !matches!(range.limits, syn::RangeLimits::Closed(_))
            || !inclusive
            || node.children.len() != 2
            || start.as_ref() != Some(&node.children[0].hir)
            || end.as_ref() != Some(&node.children[1].hir)
        {
            return Err(unsupported(
                "Original inclusive range and endpoint roster disagree",
            ));
        }
        let lower = self.endpoint(range.start.as_deref(), &node.children[0], b'0')?;
        let upper = self.endpoint(range.end.as_deref(), &node.children[1], b'9')?;

        Ok(CheckedDigit {
            binding,
            range: node,
            lower,
            upper,
        })
    }

    fn endpoint(
        &self,
        syntax: Option<&syn::Expr>,
        node: &'a HirNode,
        expected: u8,
    ) -> Result<&'a HirNode, String> {
        let Some(syn::Expr::Lit(literal)) = syntax else {
            return Err(unsupported(
                "Digit endpoint requires an original byte literal",
            ));
        };
        let syn::Lit::Byte(byte) = &literal.lit else {
            return Err(unsupported("Non-byte digit range endpoint"));
        };

        self.source(node, literal.span())?;
        let receipt = self.leaf(node, "pat_slice_bound", &TypeShape::Byte)?;
        if !literal.attrs.is_empty()
            || byte.value() != expected
            || !node.children.is_empty()
            || receipt.details
                != (Details::ByteLiteral {
                    value: expected,
                    negated: false,
                })
        {
            return Err(unsupported(
                "Original ASCII endpoint and compiler observation disagree",
            ));
        }

        Ok(node)
    }

    pub(super) fn remainder(
        &self,
        syntax: &syn::Pat,
        binding: &'a HirNode,
    ) -> Result<CheckedRemainder<'a>, String> {
        self.binding(syntax, binding, true)?;
        let syn::Pat::Ident(ident) = syntax else {
            unreachable!("Checked binding syntax");
        };
        let (_, subpattern) = ident
            .subpat
            .as_ref()
            .ok_or_else(|| unsupported("Missing named rest"))?;
        let syn::Pat::Rest(rest) = subpattern.as_ref() else {
            return Err(unsupported("Remainder binding requires original .. syntax"));
        };

        let node = &binding.children[0];
        self.source(node, rest.span())?;
        let receipt = self.leaf(
            node,
            "pat_slice_rest",
            &TypeShape::Slice(Box::new(TypeShape::Byte)),
        )?;
        if !rest.attrs.is_empty() || !node.children.is_empty() || receipt.details != Details::Rest {
            return Err(unsupported(
                "Original remainder and real rest node disagree",
            ));
        }

        Ok(CheckedRemainder {
            binding,
            rest: node,
        })
    }
}
