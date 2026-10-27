//! Exact compiler provenance for the bounded scalar-instance diagnostic lens.
use super::{comparisons, projection::Projection, types};
use crate::hir_extraction::hir::{Context, reference};
use rstyle_pdg::{ByteSpan, HirNode, HirRef};
use rustc_hir::{
    self as hir,
    def_id::{DefId, LocalDefId},
};
use rustc_middle::ty::{self, TyCtxt};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Observation {
    identity: &'static str,
    complete: bool,
    parameters: Vec<Parameter>,
    bounds: Vec<Bound>,
    compiler_bounds: Vec<CompilerBound>,
    assertions: Vec<Assertion>,
}

#[derive(Serialize)]
struct Parameter {
    index: u32,
    name: String,
    hir: HirRef,
    span: ByteSpan,
    declaration_span: ByteSpan,
    definition: super::Definition,
}

#[derive(Serialize)]
struct Argument {
    kind: &'static str,
    index: u32,
    name: String,
}

#[derive(Serialize)]
struct Bound {
    status: &'static str,
    parameter_index: u32,
    parameter_hir: HirRef,
    parameter_span: ByteSpan,
    predicate_span: ByteSpan,
    bound_span: ByteSpan,
    path_span: ByteSpan,
    origin: &'static str,
    trait_definition: comparisons::CompilerDefinition,
    original_arguments: Vec<Argument>,
    substituted_arguments: types::Arguments,
}

#[derive(Serialize)]
struct CompilerBound {
    status: &'static str,
    span: ByteSpan,
    trait_definition: comparisons::CompilerDefinition,
    original_arguments: Vec<Argument>,
    substituted_arguments: types::Arguments,
}

#[derive(Serialize)]
struct Assertion {
    status: &'static str,
    hir: HirRef,
    span: ByteSpan,
    callsite: ByteSpan,
    definition: comparisons::CompilerDefinition,
}

pub(super) fn capture<'tcx>(
    context: &Context<'tcx>,
    owner: LocalDefId,
    projection: &Projection<'tcx>,
    body: &HirNode,
) -> Result<Observation, String> {
    let generics = context
        .tcx
        .hir_get_generics(owner)
        .ok_or("Missing owner generics")?;
    let [parameter] = generics.params else {
        return Err("Provenance requires exactly one source type parameter".into());
    };

    let parameter_record = parameter_record(context, owner, parameter)?;
    let compiler_bounds = compiler_bounds(context, owner, projection, &parameter_record)?;
    let bounds = Bounds {
        context,
        owner,
        projection,
        parameter: &parameter_record,
    }
    .capture(parameter)?;

    let mut assertions = Vec::new();
    assertion_census(context, body, &mut assertions)?;
    Ok(Observation {
        identity: "rstyle.rust.scalar-instance-provenance/1",
        complete: true,
        parameters: vec![parameter_record],
        bounds,
        compiler_bounds,
        assertions,
    })
}

fn parameter_record(
    context: &Context<'_>,
    owner: LocalDefId,
    parameter: &hir::GenericParam<'_>,
) -> Result<Parameter, String> {
    if !matches!(
        parameter.kind,
        hir::GenericParamKind::Type {
            default: None,
            synthetic: false
        }
    ) || parameter.pure_wrt_drop
    {
        return Err("Unsupported generic parameter provenance".into());
    }

    let definition = context.tcx.generics_of(owner).param_at(0, context.tcx);
    if definition.def_id != parameter.def_id.to_def_id() {
        return Err("Generic parameter compiler/HIR identity mismatch".into());
    }

    let ident = parameter.name.ident();
    Ok(Parameter {
        index: definition.index,
        name: ident.name.to_string(),
        hir: reference(parameter.hir_id),
        span: context.span(ident.span),
        declaration_span: context.span(parameter.span),
        definition: super::definition_record(context.tcx, definition.def_id),
    })
}

fn original_arguments(
    arguments: ty::GenericArgsRef<'_>,
    parameter: &Parameter,
) -> Result<Vec<Argument>, String> {
    let mut records = Vec::with_capacity(arguments.len());
    for argument in arguments.iter() {
        let Some(ty) = argument.as_type() else {
            return Err("Non-type bound argument".into());
        };
        let ty::Param(original) = ty.kind() else {
            return Err("Bound argument is not the original source parameter".into());
        };
        if original.index != parameter.index || original.name.as_str() != parameter.name {
            return Err("Bound argument parameter identity mismatch".into());
        }
        records.push(Argument {
            kind: "parameter",
            index: original.index,
            name: original.name.to_string(),
        });
    }
    Ok(records)
}

fn substituted_arguments<'tcx>(
    tcx: TyCtxt<'tcx>,
    projection: &Projection<'tcx>,
    arguments: ty::GenericArgsRef<'tcx>,
) -> Result<types::Arguments, String> {
    Ok(super::projection::complete_arguments(tcx, projection.args(tcx, arguments))?.observation)
}

fn compiler_bounds<'tcx>(
    context: &Context<'tcx>,
    owner: LocalDefId,
    projection: &Projection<'tcx>,
    parameter: &Parameter,
) -> Result<Vec<CompilerBound>, String> {
    context
        .tcx
        .clauses_of(owner)
        .clauses
        .iter()
        .map(|(clause, span)| {
            let predicate = clause.kind();
            if !predicate.bound_vars().is_empty() {
                return Err("Higher-ranked compiler bound provenance unsupported".into());
            }
            let ty::ClauseKind::Trait(bound) = predicate.skip_binder() else {
                return Err("Non-trait compiler bound provenance unsupported".into());
            };
            if bound.trait_ref.def_id.is_local() {
                return Err(
                    "Local trait bound provenance unsupported before metadata encoding".into(),
                );
            }
            if bound.polarity != ty::ClausePolarity::Positive {
                return Err("Non-positive compiler bound provenance unsupported".into());
            }
            Ok(CompilerBound {
                status: "resolved",
                span: context.span(*span),
                trait_definition: comparisons::definition(context.tcx, bound.trait_ref.def_id),
                original_arguments: original_arguments(bound.trait_ref.args, parameter)?,
                substituted_arguments: substituted_arguments(
                    context.tcx,
                    projection,
                    bound.trait_ref.args,
                )?,
            })
        })
        .collect()
}

struct Bounds<'a, 'tcx> {
    context: &'a Context<'tcx>,
    owner: LocalDefId,
    projection: &'a Projection<'tcx>,
    parameter: &'a Parameter,
}

impl<'tcx> Bounds<'_, 'tcx> {
    fn capture(&self, parameter: &hir::GenericParam<'tcx>) -> Result<Vec<Bound>, String> {
        let generics = self
            .context
            .tcx
            .hir_get_generics(self.owner)
            .ok_or("Missing source generics")?;
        let mut result = Vec::new();
        for predicate in generics.predicates {
            let hir::WherePredicateKind::BoundPredicate(bound) = predicate.kind else {
                return Err("Non-type source predicate unsupported".into());
            };
            if !bound.bound_generic_params.is_empty()
                || !bound.is_param_bound(parameter.def_id.to_def_id())
            {
                return Err("Source bound is not on the original parameter".into());
            }
            for item in bound.bounds {
                result.push(self.bound(predicate, item)?);
            }
        }
        Ok(result)
    }

    fn arguments(&self, id: DefId) -> Result<ty::GenericArgsRef<'tcx>, String> {
        let candidates = self
            .context
            .tcx
            .explicit_clauses_of(self.owner)
            .clauses
            .iter()
            .filter_map(|(clause, _)| {
                let ty::ClauseKind::Trait(candidate) = clause.kind().skip_binder() else {
                    return None;
                };
                (candidate.trait_ref.def_id == id).then_some(candidate.trait_ref.args)
            })
            .collect::<Vec<_>>();
        let [arguments] = candidates.as_slice() else {
            return Err("Source bound lacks a unique compiler trait clause".into());
        };
        Ok(arguments)
    }

    fn bound(
        &self,
        predicate: &hir::WherePredicate<'tcx>,
        item: &hir::GenericBound<'tcx>,
    ) -> Result<Bound, String> {
        let hir::GenericBound::Trait(bound) = item else {
            return Err("Non-trait source bound unsupported".into());
        };
        if bound.modifiers != hir::TraitBoundModifiers::NONE
            || !bound.bound_generic_params.is_empty()
        {
            return Err("Modified or higher-ranked source bound unsupported".into());
        }
        let id = bound
            .trait_ref
            .trait_def_id()
            .ok_or("Unresolved source bound trait")?;
        let arguments = self.arguments(id)?;
        let context = self.context;
        let parameter = self.parameter;

        Ok(Bound {
            status: "resolved",
            parameter_index: parameter.index,
            parameter_hir: parameter.hir.clone(),
            parameter_span: parameter.span,
            predicate_span: context.span(predicate.span),
            bound_span: context.span(bound.span),
            path_span: context.span(bound.trait_ref.path.span),
            origin: bound_origin(predicate)?,
            trait_definition: comparisons::definition(context.tcx, id),
            original_arguments: original_arguments(arguments, parameter)?,
            substituted_arguments: substituted_arguments(context.tcx, self.projection, arguments)?,
        })
    }
}

fn bound_origin(predicate: &hir::WherePredicate<'_>) -> Result<&'static str, String> {
    let hir::WherePredicateKind::BoundPredicate(value) = predicate.kind else {
        return Err("Non-type source bound provenance unsupported".into());
    };
    match value.origin {
        hir::PredicateOrigin::GenericParam => Ok("generic_parameter"),
        hir::PredicateOrigin::WhereClause => Ok("where_clause"),
        hir::PredicateOrigin::ImplTrait => Err("Impl-trait bound provenance unsupported".into()),
    }
}

fn assertion_census(
    context: &Context<'_>,
    node: &HirNode,
    result: &mut Vec<Assertion>,
) -> Result<(), String> {
    if node.kind == "expr_assert" {
        let id = hir::HirId {
            owner: hir::OwnerId {
                def_id: LocalDefId {
                    local_def_index: rustc_hir::def_id::DefIndex::from_u32(node.hir.owner),
                },
            },
            local_id: hir::ItemLocalId::from_u32(node.hir.local),
        };
        let hir::Node::Expr(expression) = context.tcx.hir_node(id) else {
            return Err("Assertion HIR is not an expression".into());
        };
        let expansion = expression.span.ctxt().outer_expn_data();
        let definition = expansion
            .macro_def_id
            .ok_or("Missing assertion macro definition")?;
        if definition.is_local() || context.tcx.crate_name(definition.krate).as_str() != "core" {
            return Err("Assertion macro is outside core".into());
        }
        result.push(Assertion {
            status: "resolved",
            hir: reference(expression.hir_id),
            span: context.span(expression.span),
            callsite: context.span(expansion.call_site),
            definition: comparisons::definition(context.tcx, definition),
        });
    }
    for child in &node.children {
        assertion_census(context, child, result)?;
    }
    Ok(())
}
