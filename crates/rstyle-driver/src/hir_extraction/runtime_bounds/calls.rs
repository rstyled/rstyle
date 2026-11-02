//! Call-site facts for summary composition: formal provenance, targets and argument sizes.
use super::*;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::TypeVisitableExt;

impl<'tcx> Collector<'_, 'tcx> {
    pub(super) fn formal_positions(&mut self, parameters: &[hir::Param<'tcx>]) {
        for (position, parameter) in parameters.iter().enumerate() {
            if let PatKind::Binding(_, id, _, None) = parameter.pat.kind {
                self.formals.insert(id, position as u32);
            }
        }
    }

    pub(super) fn parameters(&mut self, parameters: &[hir::Param<'tcx>]) {
        for parameter in parameters {
            let formal = self.parameter(parameter);
            self.program.formals.push(formal);
        }
    }

    fn parameter(&mut self, parameter: &hir::Param<'tcx>) -> Formal {
        let PatKind::Binding(mode, id, name, None) = parameter.pat.kind else {
            return Formal {
                name: "_".into(),
                measure: Measure::Opaque,
            };
        };
        let label = name.name.to_string();
        let ty = self.ctx.typeck.pat_ty(parameter.pat);

        let measure = match self.bits(ty) {
            Some(bits) => self.scalar_parameter(id, &label, bits),
            None => shared_measure(mode, ty),
        };
        Formal {
            name: label,
            measure,
        }
    }

    fn scalar_parameter(&mut self, id: HirId, label: &str, bits: u8) -> Measure {
        let variable = self.variable(label.into(), Value::Input(label.into()), bits);
        self.program.inputs.push(label.into());
        self.locals.insert(id, variable);
        Measure::Scalar {
            input: label.into(),
        }
    }

    pub(super) fn root(&self, id: HirId) -> Root {
        self.formals
            .get(&id)
            .map_or(Root::Local(id.local_id.as_u32()), |&position| {
                Root::Formal(position)
            })
    }

    pub(super) fn provenance(&self, place: &extent::Place) -> Extent {
        Extent {
            input: place.label.clone(),
            root: self.root(place.root),
            fields: place.path.clone(),
        }
    }

    /// Trait items resolve only through concrete compiler instance selection.
    pub(super) fn target(&self, callee: DefId, arguments: ty::GenericArgsRef<'tcx>) -> Callee {
        let tcx = self.ctx.tcx;
        if matches!(tcx.def_kind(callee), DefKind::Ctor(..)) {
            return Callee::Constructor;
        }
        let trait_item = tcx
            .opt_associated_item(callee)
            .and_then(|item| item.trait_container(tcx))
            .is_some();
        if !trait_item {
            return Callee::Resolved(crate::ids::def_id_str(tcx, callee));
        }

        // Lang-item paths such as `?` desugaring may record no arguments for the item.
        if arguments.len() != tcx.generics_of(callee).count()
            || arguments.has_non_region_param()
            || arguments.has_infer()
            || arguments.has_escaping_bound_vars()
        {
            return Callee::Unresolved;
        }
        let owner = self.ctx.typeck.hir_owner.def_id;
        crate::ids::selected_instance(tcx, owner, callee, arguments)
            .filter(|instance| matches!(instance.def, ty::InstanceKind::Item(_)))
            .map_or(Callee::Unresolved, |instance| {
                Callee::Resolved(crate::ids::def_id_str(tcx, instance.def_id()))
            })
    }

    pub(super) fn call_target(&self, expression: &Expr<'tcx>, resolved: Res) -> Callee {
        match resolved {
            Res::Def(DefKind::Ctor(..), _) => Callee::Constructor,
            // The callee's `FnDef` type carries its instantiation, even for lang-item paths.
            Res::Def(DefKind::Fn | DefKind::AssocFn, id) => {
                match self.ctx.typeck.expr_ty(expression).kind() {
                    ty::FnDef(def, arguments) if *def == id => {
                        self.target(id, arguments.skip_binder())
                    }
                    _ => Callee::Unresolved,
                }
            }
            _ => Callee::Unresolved,
        }
    }

    pub(super) fn method_target(&self, expression: &Expr<'tcx>) -> Callee {
        self.ctx
            .typeck
            .type_dependent_def_id(expression.hir_id)
            .map_or(Callee::Unresolved, |id| {
                self.target(id, self.ctx.typeck.node_args(expression.hir_id))
            })
    }

    /// Overloaded operators call their trait impl after the operands are evaluated.
    pub(super) fn overloaded(
        &mut self,
        expression: &'tcx Expr<'tcx>,
        operands: &[&'tcx Expr<'tcx>],
        next: u32,
    ) -> u32 {
        let Some(method) = self.ctx.typeck.type_dependent_def_id(expression.hir_id) else {
            return next;
        };
        let action = Action::Call {
            callee: self.ctx.tcx.def_path_str(method),
            target: self.method_target(expression),
            arguments: operands
                .iter()
                .map(|operand| self.argument(operand))
                .collect(),
            may_modify: false,
        };
        self.node(action, Flow::Goto(next), expression.span)
    }

    /// Scalars keep their captured value; shared places keep their root and field path.
    pub(super) fn argument(&self, argument: &Expr<'tcx>) -> Argument {
        if self
            .bits(self.ctx.typeck.expr_ty_adjusted(argument))
            .is_some()
        {
            return Argument::Scalar(self.value(argument));
        }
        extent::place(self.ctx, argument).map_or(Argument::Opaque, |place| Argument::Place {
            root: self.root(place.root),
            fields: place.path.clone(),
        })
    }
}

fn shared_measure(mode: hir::BindingMode, ty: Ty<'_>) -> Measure {
    if mode == hir::BindingMode(hir::ByRef::No, hir::Mutability::Not)
        && matches!(ty.kind(), ty::Ref(_, _, hir::Mutability::Not))
    {
        Measure::Shared
    } else {
        Measure::Opaque
    }
}
