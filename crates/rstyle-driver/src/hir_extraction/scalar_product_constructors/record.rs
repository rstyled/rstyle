//! Retain the real callee and declaration locations without adding an expression child.
use super::*;

impl<'tcx> Context<'tcx> {
    pub(super) fn record_scalar_constructor(
        &self,
        expression: &hir::Expr<'tcx>,
        selected: &selected::Constructor<'tcx>,
    ) -> Option<Observation> {
        let ExprKind::Call(callee, values) = expression.kind else {
            return None;
        };
        let [first, second] = values else {
            return None;
        };
        let owner = definition_record(self.tcx, selected.owner.did());
        let arguments = arguments(selected.scalar);

        Some(Observation {
            identity: IDENTITY.into(),
            hir: reference(expression.hir_id),
            span: self.span(expression.span),
            callee: Callee {
                hir: reference(callee.hir_id),
                span: self.span(callee.span),
                snippet: self.snippet(callee.span),
            },
            constructor: definition_record(self.tcx, selected.definition),
            owner: owner.clone(),
            declaration: self.scalar_constructor_declaration(selected.owner.did())?,
            arguments: arguments.clone(),
            fields: self.scalar_constructor_fields(selected)?,
            values: [reference(first.hir_id), reference(second.hir_id)],
            signature: Signature {
                inputs: [
                    ValueType::SharedByteSlice,
                    ValueType::Scalar(selected.scalar),
                ],
                output_owner: owner,
                output_arguments: arguments,
                bound_variables: 0,
                abi: selected.signature.abi().to_string(),
                safety: format!("{:?}", selected.signature.safety()),
                variadic: false,
            },
        })
    }

    fn scalar_constructor_fields(
        &self,
        selected: &selected::Constructor<'tcx>,
    ) -> Option<[Field; 2]> {
        let mut fields = selected.owner.non_enum_variant().fields.iter();
        let first = fields.next()?;
        let second = fields.next()?;

        Some([
            Field {
                position: 0,
                definition: definition_record(self.tcx, first.did),
                ty: ValueType::SharedByteSlice,
            },
            Field {
                position: 1,
                definition: definition_record(self.tcx, second.did),
                ty: ValueType::Scalar(selected.scalar),
            },
        ])
    }

    fn scalar_constructor_declaration(&self, owner: DefId) -> Option<Declaration> {
        let hir::Node::Item(item) = self.tcx.hir_node_by_def_id(owner.as_local()?) else {
            return None;
        };
        if item.span.from_expansion() {
            return None;
        }

        let file = self
            .tcx
            .sess
            .source_map()
            .lookup_source_file(item.span.lo());
        let rustc_span::FileName::Real(real) = &file.name else {
            return None;
        };
        let path = real.local_path()?;
        let (_, span) = self.sources.range(self.tcx, item.span)?;

        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().ok()?.join(path)
        };
        Some(Declaration {
            source_file: absolute.to_string_lossy().into_owned(),
            span,
        })
    }
}

fn arguments(scalar: Argument) -> [GenericArgument; 2] {
    [
        GenericArgument::ErasedLifetime,
        GenericArgument::Scalar(scalar),
    ]
}
